#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use] extern crate rocket;
#[macro_use] extern crate serde_derive;
use std::sync::Mutex;
use std::thread;
use rocket::{Rocket, State, response::Debug};
use rocket_contrib::json::Json;

use rdf::{
    node::Node,
    uri::Uri,
};
use oxigraph::model::*;

use std::str;

use std::sync::mpsc;
use ::reasonable::reasoner::{
    node_to_string,
};
use ::reasonable::error::{
    Result,
    ReasonableError,
};
use ::reasonable::manager::{
    Manager,
    ViewMetadata,
    parse_file,
};


use rusqlite::{
    Connection,
    NO_PARAMS,
    Action,
    params,
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

struct SQLiteManager {
    mgr: Manager,
    conn: Connection,
    recv: mpsc::Receiver<ChannelMessage>,
    send: mpsc::SyncSender<ChannelMessage>,
    views: Vec<ViewMetadata>,
}

impl SQLiteManager {
    fn new(filename: &str) -> Result<Self> {
        let (send, recv) = mpsc::sync_channel(5);
        let mgr = SQLiteManager{
            mgr: Manager::new(),
            conn: Connection::open(filename)?,
            recv,
            send,
            views: Vec::new(),
        };

        // mgr.conn.create_module("reasonable", &read_only_module::<ReasonableTable>(), None)?;

        mgr.conn.execute("CREATE TABLE IF NOT EXISTS triples (
            subject TEXT NOT NULL,
            predicate TEXT NOT NULL,
            object TEXT NOT NULL
        )", NO_PARAMS)?;

        let sendc = mgr.get_view_channel();
        //mgr.conn.commit_hook(Some(mgr.get_commit_hook(sendc)));
        mgr.conn.update_hook(Some(mgr.get_update_hook(sendc)));
        Ok(mgr)
    }

    fn get_view_channel(&self) -> mpsc::SyncSender<ChannelMessage> {
        self.send.clone()
    }

    fn add_view(&mut self, name: String, query: &str) -> Result<()> {
        let view = self.mgr.add_view2(name, query)?;

        // create table
        let table_def = view.get_create_tab();
        self.conn.execute(&table_def, NO_PARAMS)?;

        //TODO do not add view def more than once
        self.views.push(view);
        self.update()
    }

    fn add_triples(&mut self, triples: Vec<(String, String, String)>) -> Result<()> {
        let load_triples: Vec<(Node, Node, Node)> = triples.into_iter().filter_map(|(s_, p_, o_)| {
            let s: Node = {
                if let Ok(named) = NamedNode::new(s_.clone()) {
                    uri!(named.into_string())
                } else if let Ok(bnode) = BlankNode::new(s_) {
                    bnode!(bnode.into_string())
                } else {
                    return None
                }
            };

            let p: Node = uri!(p_);

            let o: Node = {
                if let Ok(named) = NamedNode::new(o_.clone()) {
                    uri!(named.into_string())
                } else if let Ok(bnode) = BlankNode::new(o_.clone()) {
                    bnode!(bnode.into_string())
                } else {
                    literal!(o_, None, None)
                }
            };

            Some((s, p, o))
        }).collect();
        let tx = self.conn.transaction()?;
        for trip in load_triples {
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

    fn get_update_hook(&self, sender: mpsc::SyncSender<ChannelMessage>) -> Box<dyn FnMut(Action, &str, &str, i64) + Send> {
        Box::new(move |_act, _db_name, table_name, _rowid| {
            if table_name == "triples" {
                // println!("got {:?} {} {} {}", act, db_name, table_name, rowid);
                match sender.try_send(ChannelMessage::Refresh) {
                    Ok(_) =>  return,
                    Err(mpsc::TrySendError::Full(_e)) => return,
                    Err(mpsc::TrySendError::Disconnected(e)) => panic!(e)
                };
               // sender.send(rowid).unwrap();
            }
        })
        //|act: Action, db_name: &str, table_name: &str, rowid: i64) {
    }

    fn get_commit_hook(&self, sender: mpsc::SyncSender<ChannelMessage>) -> Box<dyn FnMut() -> bool + Send> {
        Box::new(move || {
            sender.try_send(ChannelMessage::Refresh).unwrap();
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
                Err(_e) => None
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
            let res = self.recv.recv();
            match res {
                Ok(ChannelMessage::ViewDef(vdef)) => self.add_view(vdef.name, &vdef.query)?,
                Ok(ChannelMessage::TripleAdd(trips)) => { self.add_triples(trips)?; self.update()? },
                Ok(ChannelMessage::Refresh) => self.update()?,
                Err(e) => return Err(ReasonableError::ChannelRecv(e)),
            };
        }
    }
}

#[derive(Deserialize, Serialize)]
struct TableResponse {
    rows: Vec<Vec<String>>
}

#[derive(Debug)]
enum ChannelMessage {
    ViewDef(MakeView),
    TripleAdd(Vec<JsonTriple>),
    Refresh,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
struct MakeView {
    name: String,
    query: String,
}


type JsonTriple = (String, String, String);
struct ViewChannel(mpsc::SyncSender<ChannelMessage>);
type DbConn = Mutex<Connection>;

#[get("/view/<name>", format = "json")]
fn hello(name: String, conn: State<DbConn>, _tx: State<ViewChannel>) -> Json<TableResponse>  {
    let mut rows: Vec<Vec<String>> = Vec::new();
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
    Json(TableResponse{rows})
}

#[post("/make", data = "<data>", format = "json")]
fn makeview(data: Json<MakeView>, _conn: State<DbConn>, tx: State<ViewChannel>) -> Json<()>  {
    tx.0.try_send(ChannelMessage::ViewDef(data.0)).expect("send view def");
    Json(())
}

#[post("/add", data = "<data>", format = "json")]
fn addtriples(data: Json<Vec<JsonTriple>>, _conn: State<DbConn>, tx: State<ViewChannel>) -> Json<()>  {
    tx.0.try_send(ChannelMessage::TripleAdd(data.0)).expect("add triples");
    Json(())
}

fn rocket(filename: &str) {
    let mut mgr = SQLiteManager::new(filename).unwrap();

    mgr.load_file("example_models/ontologies/Brick.n3").unwrap();
    //mgr.load_file("example_models/soda_hall.n3").unwrap();

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
            .mount("/", routes![hello, makeview, addtriples])
            .launch();
    });

    mgr.update_loop().unwrap();

}

fn main() {
    env_logger::init();
    rocket("test.db");
}
