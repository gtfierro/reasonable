#![feature(proc_macro_hygiene, decl_macro)]
#[macro_use]
extern crate rocket;
#[macro_use]
extern crate serde_derive;
use log::{debug, info};
use oxigraph::model::*;
use oxigraph::sparql::{QueryOptions, QueryResults};
use oxigraph::store::MemoryStore;
use rdf::{node::Node, uri::Uri};
use reasonable::error::{ReasonableError, Result};
use reasonable::manager::{parse_file, Manager, ViewMetadata};
use reasonable::reasoner::node_to_string;
use rocket::State;
use rocket_contrib::json::Json;
use rusqlite::params;
use rusqlite::NO_PARAMS;
use std::str;
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;

type BoxError = Box<dyn std::error::Error + Send + Sync + 'static>;

#[allow(non_upper_case_globals)]
const qfmt: &str = "PREFIX brick: <https://brickschema.org/schema/1.1/Brick#>
    PREFIX tag: <https://brickschema.org/schema/1.1/BrickTag#>
    PREFIX rdf: <http://www.w3.org/1999/02/22-rdf-syntax-ns#>
    PREFIX rdfs: <http://www.w3.org/2000/01/rdf-schema#>
    PREFIX owl: <http://www.w3.org/2002/07/owl#>
    PREFIX qudt: <http://qudt.org/schema/qudt/>
    ";

macro_rules! uri {
    ($t:expr) => {
        Node::UriNode { uri: Uri::new($t) }
    };
}
macro_rules! bnode {
    ($t:expr) => {
        Node::BlankNode { id: $t }
    };
}
macro_rules! literal {
    ($t:expr, $d:expr, $l:expr) => {
        Node::LiteralNode {
            literal: $t,
            data_type: $d,
            language: $l,
        }
    };
}

struct SQLiteManager {
    mgr: Manager,
    conn: rusqlite::Connection,
    recv: mpsc::Receiver<ChannelMessage>,
    send: mpsc::SyncSender<ChannelMessage>,
    views: Vec<ViewMetadata>,
    changed: bool,
}

impl SQLiteManager {
    fn new(filename: &str) -> Result<Self> {
        let (send, recv) = mpsc::sync_channel(1);
        let mut mgr = SQLiteManager {
            mgr: Manager::new(),
            conn: rusqlite::Connection::open(filename)?,
            recv,
            send,
            views: Vec::new(),
            changed: true,
        };

        // mgr.conn.create_module("reasonable", &read_only_module::<ReasonableTable>(), None)?;

        mgr.conn.execute(
            "CREATE TABLE IF NOT EXISTS triples (
            subject TEXT NOT NULL,
            predicate TEXT NOT NULL,
            object TEXT NOT NULL,
            UNIQUE(subject, predicate, object)
        )",
            NO_PARAMS,
        )?;

        let sendc = mgr.get_view_channel();
        //mgr.conn.commit_hook(Some(mgr.get_commit_hook(sendc)));
        mgr.conn.update_hook(Some(mgr.get_update_hook(sendc)));
        mgr.add_view(
            String::from("reasoned"),
            "SELECT ?s ?p ?o WHERE { ?s ?p ?o }",
        )?;
        Ok(mgr)
    }

    fn get_view_channel(&self) -> mpsc::SyncSender<ChannelMessage> {
        self.send.clone()
    }

    fn add_view(&mut self, name: String, query: &str) -> Result<()> {
        for view in &self.views {
            if view.table_name == name && view.query_string == query {
                return Ok(());
            }
        }
        // remove old table if exists
        self.conn
            .execute(&format!("DROP TABLE IF EXISTS view_{}", &name), NO_PARAMS)?;
        self.views.retain(|view| view.name() != name);

        // create table and add view
        let view = self.mgr.add_view2(name, query)?;
        let table_def = view.get_create_tab();
        self.conn.execute(&table_def, NO_PARAMS)?;

        //TODO do not add view def more than once
        self.views.push(view);
        self.changed = true;
        self.update()
    }

    fn add_triples(&mut self, triples: Vec<(String, String, String)>) -> Result<()> {
        let load_triples: Vec<(Node, Node, Node)> = triples
            .into_iter()
            .filter_map(|(s_, p_, o_)| {
                let s: Node = {
                    if let Ok(named) = NamedNode::new(s_.clone()) {
                        uri!(named.into_string())
                    } else if let Ok(bnode) = BlankNode::new(s_) {
                        bnode!(bnode.into_string())
                    } else {
                        return None;
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
            })
            .collect();
        let tx = self.conn.transaction()?;
        for trip in load_triples {
            tx.execute(
                "INSERT OR IGNORE INTO triples(subject, predicate, object) VALUES (?1, ?2, ?3)",
                params![
                    node_to_string(&trip.0),
                    node_to_string(&trip.1),
                    node_to_string(&trip.2)
                ],
            )?;
        }
        match tx.commit() {
            Err(e) => Err(ReasonableError::SQLite(e)),
            Ok(_) => {
                self.changed = true;
                self.update()?;
                Ok(())
            }
        }
    }

    fn get_update_hook(
        &self,
        sender: mpsc::SyncSender<ChannelMessage>,
    ) -> Box<dyn FnMut(rusqlite::Action, &str, &str, i64) + Send> {
        Box::new(move |_act, _db_name, table_name, _rowid| {
            if table_name == "triples" {
                match sender.try_send(ChannelMessage::Refresh) {
                    Ok(_) => return,
                    Err(mpsc::TrySendError::Full(_e)) => return,
                    Err(mpsc::TrySendError::Disconnected(e)) => panic!(e),
                };
            }
        })
    }

    fn get_triples(&self) -> Result<Vec<(String, String, String)>> {
        debug!("triggered");
        let mut stmt = self
            .conn
            .prepare("SELECT subject, predicate, object FROM triples")?;

        let triples: Vec<(String, String, String)> = stmt
            .query_map(NO_PARAMS, |row| {
                let s: String = row.get(0)?;
                let p: String = row.get(1)?;
                let o: String = row.get(2)?;

                Ok((s, p, o))
            })?
            .filter_map(|tres| match tres {
                Ok(t) => Some(t),
                Err(_e) => None,
            })
            .collect();

        Ok(triples)
    }

    fn load_file(&mut self, filename: &str) -> Result<()> {
        let tx = self.conn.transaction()?;
        let triples = parse_file(filename)?;
        for trip in triples {
            tx.execute(
                "INSERT OR IGNORE INTO triples(subject, predicate, object) VALUES (?1, ?2, ?3)",
                params![
                    node_to_string(&trip.0),
                    node_to_string(&trip.1),
                    node_to_string(&trip.2)
                ],
            )?;
        }
        match tx.commit() {
            Err(e) => Err(ReasonableError::SQLite(e)),
            Ok(_) => {
                self.changed = true;
                self.update()?;
                Ok(())
            }
        }
    }

    fn update(&mut self) -> Result<()> {
        // try to see if there are any new views
        if !self.changed {
            debug!("triggered but no change");
            return Ok(());
        } else {
            debug!("changing row");
        }

        // TODO: return true if the input changed
        self.mgr.load_triples(self.get_triples()?)?;
        let tx = self.conn.transaction()?;
        for view in self.views.iter() {
            tx.execute(&view.get_delete_tab(), NO_PARAMS)?;
            //tx.execute(format!("DELETE FROM view_{};", view.name()).as_str(), NO_PARAMS)?;
            debug!("insert: {}", view.get_insert_sql());
            let mut stmt = tx.prepare(&view.get_insert_sql())?;
            let tuples: Vec<Vec<String>> = view.contents_string()?;
            info!("got {} for {}", tuples.len(), view.name());
            for tup in tuples {
                stmt.execute(&tup)?;
            }
        }
        // loop through views and update
        match tx.commit() {
            Err(e) => Err(ReasonableError::SQLite(e)),
            Ok(_) => {
                self.changed = false;
                Ok(())
            }
        }
    }

    fn update_loop(&mut self) -> Result<()> {
        loop {
            let res = self.recv.recv();
            match res {
                Ok(ChannelMessage::ViewDef(vdef, tx)) => {
                    self.add_view(vdef.name, &vdef.query)?;
                    tx.send(()).unwrap();
                }
                Ok(ChannelMessage::TripleAdd(trips, tx)) => {
                    self.add_triples(trips)?;
                    if let Some(c) = tx {
                        c.send(()).unwrap();
                    }
                }
                Ok(ChannelMessage::Refresh) => self.update()?,
                Err(e) => return Err(ReasonableError::ChannelRecv(e)),
            };
        }
    }
}

#[derive(Deserialize, Serialize)]
struct TableResponse {
    header: Vec<String>,
    rows: Vec<Vec<String>>,
}

#[derive(Debug)]
enum ChannelMessage {
    ViewDef(ViewDef, mpsc::Sender<()>),
    TripleAdd(Vec<JsonTriple>, Option<mpsc::Sender<()>>),
    Refresh,
}

#[derive(Deserialize, Serialize, Debug)]
struct ViewDef {
    name: String,
    query: String,
}

type JsonTriple = (String, String, String);
struct ViewChannel(mpsc::SyncSender<ChannelMessage>);
type DbConn = Mutex<rusqlite::Connection>;
type RdfConn = Mutex<MemoryStore>;

#[get("/view/<name>", format = "json")]
fn getview(
    name: String,
    conn: State<DbConn>,
    _store: State<RdfConn>,
    _tx: State<ViewChannel>,
) -> Json<TableResponse> {
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut header: Vec<String> = Vec::new();
    conn.lock()
        .expect("db connection lock")
        .prepare(&format!("SELECT * FROM view_{};", name))
        .expect("bad query")
        .query_map(NO_PARAMS, |row| {
            let rowvec: Vec<String> = (0..(row.column_count()))
                .map(|i| row.get(i).unwrap())
                .collect();
            rows.push(rowvec);
            header = row.column_names().iter().map(|s| s.to_string()).collect();
            Ok(())
        })
        .unwrap()
        .count();
    info!("rows {}", rows.len());
    Json(TableResponse { header, rows })
}

#[post("/make", data = "<data>", format = "json")]
fn makeview(
    data: Json<ViewDef>,
    _conn: State<DbConn>,
    _store: State<RdfConn>,
    tx: State<ViewChannel>,
) -> Json<()> {
    let (send, recv) = mpsc::channel();
    tx.0.send(ChannelMessage::ViewDef(data.0, send))
        .expect("make view");
    recv.recv().unwrap();
    Json(())
}

#[post("/add", data = "<data>", format = "json")]
fn addtriples(data: Json<Vec<JsonTriple>>, tx: State<ViewChannel>) -> Json<()> {
    let (send, recv) = mpsc::channel();
    tx.0.send(ChannelMessage::TripleAdd(data.0, Some(send)))
        .expect("add triples");
    recv.recv().unwrap();
    Json(())
}

#[post("/addnb", data = "<data>", format = "json")]
fn addtriples_nonblock(data: Json<Vec<JsonTriple>>, tx: State<ViewChannel>) -> Json<()> {
    tx.0.send(ChannelMessage::TripleAdd(data.0, None))
        .expect("add triples");
    Json(())
}

#[post("/query", data = "<data>", format = "json")]
fn doquery(
    data: Json<String>,
    _conn: State<DbConn>,
    store: State<RdfConn>,
    _tx: State<ViewChannel>,
) -> Result<Json<Vec<Vec<String>>>> {
    let sparql = format!("{}{}", qfmt, data.0);
    info!("do query {}", sparql);
    let q = store
        .lock()
        .expect("rdf lock")
        .prepare_query(&sparql, QueryOptions::default())?;
    let res = q.exec()?;
    let mut rows: Vec<Vec<String>> = Vec::new();
    match res {
        QueryResults::Solutions(solutions) => {
            let vars: Vec<String> = solutions
                .variables()
                .to_vec()
                .iter()
                .map(|t| t.to_string())
                .collect();
            for soln in solutions {
                let vals = soln?;
                let mut row: Vec<String> = Vec::new();
                for col in &vars {
                    row.push(vals.get(col.as_str()).unwrap().clone().to_string());
                }
                rows.push(row);
            }
        }
        QueryResults::Boolean(b) => rows.push(vec![format!("{}", b)]),
        _ => {}
    };
    Ok(Json(rows))
}

fn rocket(filename: &str) {
    let mut mgr = SQLiteManager::new(filename).unwrap();

    mgr.load_file("Brick.n3").unwrap();
    mgr.update().unwrap();

    let conn = rusqlite::Connection::open(filename).unwrap();
    let store = mgr.mgr.store();
    let tx = mgr.get_view_channel();
    thread::spawn(move || {
        rocket::ignite()
            .manage(Mutex::new(conn))
            .manage(Mutex::new(store))
            .manage(ViewChannel(tx))
            .mount(
                "/",
                routes![getview, makeview, addtriples, addtriples_nonblock, doquery],
            )
            .launch();
    });

    mgr.update_loop().unwrap();
}

fn main() {
    env_logger::init();
    rocket("test.db");
}
