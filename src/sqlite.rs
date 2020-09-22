#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
use std::sync::Mutex;
use std::thread;
use rocket::{Rocket, State, response::Debug};
use rocket_contrib::json::Json;

use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;
use std::str;
use regex::Regex;
use std::sync::mpsc;
use ::reasonable::owl::{
    Reasoner,
    node_to_string,
};
use ::reasonable::error::{
    Result,
    ReasonableError,
};
use ::reasonable::manager::{
    Manager,
    ViewRef,
    ViewMetadata,
    parse_file,
};
use rdf::{
    node::Node,
    uri::Uri,
};
use oxigraph::model::{
    NamedNode,
    BlankNode,
    NamedOrBlankNode,
};
use rusqlite::{
    Connection,
    NO_PARAMS,
    Action,
    params,
    functions::FunctionFlags,
};

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

macro_rules! uri {
    ($t:expr) => (Node::UriNode{uri: Uri::new($t)});
}
macro_rules! bnode {
    ($t:expr) => (Node::BlankNode{id: $t});
}
macro_rules! literal {
    ($t:expr, $d:expr, $l:expr) => (Node::LiteralNode{literal: $t, data_type: $d, language: $l});
}

// TODO: "sqlite3 manager" which closes over the manager + sqlite connection
// and does the necessary queries when the table is updated.
// TODO: for every row? every commit? need to do some tests

struct SQLiteManager {
    mgr: Manager,
    conn: Connection,
    recv: mpsc::Receiver<()>,
    viewrecv: mpsc::Receiver<MakeView>,
    viewsend: mpsc::SyncSender<MakeView>,
    views: Vec<ViewMetadata>,
}

impl SQLiteManager {
    fn new(filename: &str) -> Result<Self> {
        let (sender, receiver) = mpsc::channel();
        let (viewsend, viewrecv) = mpsc::sync_channel(1);
        let mgr = SQLiteManager{
            mgr: Manager::new(),
            conn: Connection::open(filename)?,
            recv: receiver,
            viewsend: viewsend,
            viewrecv: viewrecv,
            views: Vec::new(),
        };

        // mgr.conn.create_module("reasonable", &read_only_module::<ReasonableTable>(), None)?;

        mgr.conn.execute("CREATE TABLE IF NOT EXISTS triples (
            subject TEXT NOT NULL,
            predicate TEXT NOT NULL,
            object TEXT NOT NULL
        )", NO_PARAMS)?;

        //mgr.conn.update_hook(Some(mgr.get_update_hook(sender)));
        mgr.conn.commit_hook(Some(mgr.get_commit_hook(sender)));
        Ok(mgr)
    }

    fn get_view_channel(&self) -> mpsc::SyncSender<MakeView> {
        self.viewsend.clone()
    }

    fn add_view(&mut self, name: String, query: &str) -> Result<()> {
        let view = self.mgr.add_view2(name, query)?;

        // create table
        let table_def = view.get_create_tab();
        self.conn.execute(&table_def, NO_PARAMS)?;

        self.views.push(view);
        Ok(())
    }

    fn get_update_hook(&self, sender: mpsc::Sender<i64>) -> Box<dyn FnMut(Action, &str, &str, i64) + Send> {
        Box::new(move |act, db_name, table_name, rowid| {
            if table_name == "triples" {
                // println!("got {:?} {} {} {}", act, db_name, table_name, rowid);
                sender.send(rowid).unwrap();
            }
        })
        //|act: Action, db_name: &str, table_name: &str, rowid: i64) {
    }

    fn get_commit_hook(&self, sender: mpsc::Sender<()>) -> Box<dyn FnMut() -> bool + Send> {
        Box::new(move || {
            sender.send(()).unwrap();
            false
        })
    }

    fn get_triples(&self) -> Result<Vec<(String, String, String)>>{
        println!("triggered");
        let mut stmt = self.conn.prepare("SELECT subject, predicate, object FROM triples")?;

        let triples: Vec<(String, String, String)> = stmt.query_map(NO_PARAMS, |row| {
            //println!("row {:?}", row.columns());
            let s: String = row.get(0)?;
            let p: String = row.get(1)?;
            let o: String = row.get(2)?;

            Ok((s, p, o))
        })?.filter_map(|tres| {
            match tres {
                Ok(t) => Some(t),
                Err(e) => None
            }
        }).collect();

        Ok(triples)
    }

    fn load_file(&mut self, filename: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        let triples = parse_file(filename)?;
        for trip in triples {
            tx.execute(
                "INSERT INTO triples(subject, predicate, object) VALUES (?1, ?2, ?3)",
                params![node_to_string(&trip.0), node_to_string(&trip.1), node_to_string(&trip.2)]
            )?;
        }
        match tx.commit() {
            Err(e) => Err(ReasonableError::SQLite(e)),
            Ok(_) => Ok(())
        }

    }

    fn update(&mut self) -> Result<()> {
        // try to see if there are any new views

        self.mgr.load_triples(self.get_triples()?)?;
        let tx = self.conn.transaction()?;
        for view in self.views.iter() {
            tx.execute(format!("DELETE FROM {};", view.name()).as_str(), NO_PARAMS)?;
            println!("insert: {}", view.get_insert_sql());
            let mut stmt = tx.prepare(&view.get_insert_sql())?;
            let tuples: Vec<Vec<String>> = view.contents_string()?;
            println!("got {} for {}", tuples.len(), view.name());
            for tup in tuples {
                stmt.execute(&tup)?;
            }
        }
        // loop through views and update
        match tx.commit() {
            Err(e) => Err(ReasonableError::SQLite(e)),
            Ok(_) => Ok(())
        }
    }

    fn update_loop(&mut self) -> Result<()> {
        loop {
            let res = self.viewrecv.try_recv();
            match res {
                Ok(vdef) => self.add_view(vdef.name, &vdef.query)?,
                Err(e) => match e {
                    mpsc::TryRecvError::Empty => println!("no view yet"),
                    mpsc::TryRecvError::Disconnected => return Err(ReasonableError::ManagerError("bad".to_string())),
                }
            };
            self.recv.recv()?;
            if let Err(e) = self.update() {
                return Err(e);
            }
        }
    }
}

#[derive(Deserialize, Serialize)]
struct TableResponse {
    rows: Vec<Vec<String>>
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct MakeView {
    name: String,
    query: String,
}


type JsonTriple = (String, String, String);
struct ViewChannel(mpsc::SyncSender<MakeView>);
type DbConn = Mutex<Connection>;

#[get("/view/<name>", format = "json")]
fn hello(name: String, conn: State<DbConn>, tx: State<ViewChannel>) -> Json<TableResponse>  {
    let mut rows: Vec<Vec<String>> = Vec::new();
    println!("in here");
    conn.lock()
        .expect("db connection lock")
        .prepare(&format!("SELECT * FROM {};", name))
        .expect("bad query")
        .query_map(NO_PARAMS, |row| {
            let rowvec: Vec<String> = (0..(row.column_count())).map(|i| row.get(i).unwrap()).collect();
            rows.push(rowvec);
            Ok(())
        }).unwrap().count();
    println!("rows {}", rows.len());
    Json(TableResponse{rows: rows})
}

#[post("/make", data = "<data>", format = "json")]
fn makeview(data: Json<MakeView>, conn: State<DbConn>, tx: State<ViewChannel>) -> Json<()>  {
    tx.0.try_send(data.0).expect("send view def");
    Json(())
}


fn rocket(filename: &str) {
    let mut mgr = SQLiteManager::new(filename).unwrap();

    mgr.load_file("example_models/ontologies/Brick.n3").unwrap();
    mgr.load_file("example_models/soda_hall.n3").unwrap();

    mgr.conn.execute("DROP TABLE IF EXISTS test1;", NO_PARAMS).unwrap();
    mgr.update().unwrap();
    mgr.add_view("test1".to_string(), "SELECT ?x ?y WHERE { ?x rdf:type brick:Sensor . ?x brick:isPointOf ?y }").unwrap();

    mgr.update().unwrap();

    let conn = Connection::open(filename).unwrap();
    let tx = mgr.get_view_channel();
    thread::spawn(move || {
        rocket::ignite()
            .manage(Mutex::new(conn))
            .manage(ViewChannel(tx))
            .mount("/", routes![hello, makeview])
            .launch();
    });

    mgr.update_loop().unwrap();

}

fn main() {
    env_logger::init();
    rocket("test.db");
}
