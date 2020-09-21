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
use std::sync::mpsc::{
    channel,
    Sender,
    Receiver
};
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
};

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
    recv: Receiver<i64>,
    views: Vec<ViewMetadata>,
}

impl SQLiteManager {
    fn new(filename: &str) -> Result<Self> {
        let (sender, receiver) = channel();
        let mgr = SQLiteManager{
            mgr: Manager::new(),
            conn: Connection::open(filename)?,
            recv: receiver,
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

    fn add_view(&mut self, name: String, query: &str) -> Result<()> {
        let view = self.mgr.add_view2(name, query)?;

        // create table
        let table_def = view.get_create_tab();
        self.conn.execute(&table_def, NO_PARAMS)?;

        self.views.push(view);
        Ok(())
    }

    fn get_update_hook(&self, sender: Sender<i64>) -> Box<dyn FnMut(Action, &str, &str, i64) + Send> {
        Box::new(move |act, db_name, table_name, rowid| {
            if table_name == "triples" {
                // println!("got {:?} {} {} {}", act, db_name, table_name, rowid);
                sender.send(rowid).unwrap();
            }
        })
        //|act: Action, db_name: &str, table_name: &str, rowid: i64) {
    }

    fn get_commit_hook(&self, sender: Sender<i64>) -> Box<dyn FnMut() -> bool + Send> {
        Box::new(move || {
            sender.send(0).unwrap();
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

#[derive(Deserialize, Serialize)]
struct MakeView {
    name: String,
    query: String,
}


type JsonTriple = (String, String, String);

#[get("/")]
fn hello(conn: State<Mutex<Connection>>) -> Json<TableResponse>  {
    let mut rows: Vec<Vec<String>> = Vec::new();
    conn.lock()
        .expect("db connection lock")
        .prepare("SELECT * FROM test1;")
        .expect("bad query")
        .query_map(NO_PARAMS, |row| {
            rows.push(vec![row.get(0).unwrap(), row.get(1).unwrap()]);
            Ok(())
        }).unwrap().count();
    println!("rows {}", rows.len());
    Json(TableResponse{rows: rows})
}

#[post("/make", data = "<data>")]
fn makeview(conn: State<Mutex<Connection>>, data: Json<MakeView>) -> Json<()>  {
    Json(())
}


fn rocket(filename: &str) {
    let mut mgr = SQLiteManager::new(filename).unwrap();

    mgr.load_file("example_models/ontologies/Brick.n3").unwrap();
    mgr.load_file("example_models/soda_hall.n3").unwrap();
    mgr.conn.execute(
        "INSERT INTO triples(subject, predicate, object) VALUES (?1, ?2, ?3)",
        params!["x", "y", "z"]
    ).unwrap();

    mgr.conn.execute("DROP TABLE IF EXISTS test1;", NO_PARAMS).unwrap();
    //mgr.conn.execute("CREATE VIRTUAL TABLE test1 USING reasonable('SELECT ?x ?y WHERE { ?x rdf:type brick:Sensor . ?x brick:isPointOf ?y }')", NO_PARAMS).unwrap();
    mgr.update().unwrap();
    mgr.add_view("test1".to_string(), "SELECT ?x ?y WHERE { ?x rdf:type brick:Sensor . ?x brick:isPointOf ?y }").unwrap();

    mgr.update().unwrap();

    let conn = Connection::open(filename);
    thread::spawn(move || {
        rocket::ignite()
            .manage(Mutex::new(conn))
            .mount("/", routes![hello])
            .launch();
    });

    loop {
        mgr.update().unwrap();
    }

}

fn main() {
    env_logger::init();
    rocket("test.db");
}

