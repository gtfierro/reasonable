//! The `reasonable` package offers a library, binary and Python bindings for performing OWL 2 RL
//! reasoning on RDF graphs. This package implements the Datalog rules as communicated on the [W3C
//! OWL2
//! Profile](https://www.w3.org/TR/owl2-profiles/#Reasoning_in_OWL_2_RL_and_RDF_Graphs_using_Rules)
//! website.
use rusqlite::{ffi, NO_PARAMS, params};
use std::os::raw::c_int;
use rusqlite::functions::FunctionFlags;
use rusqlite::types::{Value, ValueRef};
use rusqlite::functions::Context;
use rusqlite::types::ToSql;

mod index;
pub mod owl;
mod disjoint_sets;
#[allow(dead_code)]
mod common;

#[cfg(feature="python-library")]
mod python;

#[no_mangle]
pub extern "C" fn sqlite3_reasonable_init(
    db: *mut ffi::sqlite3,
    _pz_err_msg: &mut &mut std::os::raw::c_char,
    p_api: *mut ffi::sqlite3_api_routines,
) -> c_int {
    // SQLITE_EXTENSION_INIT2 equivalent
    unsafe {
        ffi::sqlite3_api = p_api;
    }
    /* Insert here calls to
     **     sqlite3_create_function_v2(),
     **     sqlite3_create_collation_v2(),
     **     sqlite3_create_module_v2(), and/or
     **     sqlite3_vfs_register()
     ** to register the new features that your extension adds.
     */
    match init(db) {
        Ok(()) => {
            eprintln!("[sqlite-reasonable] initialized");
            // TODO: make permanent with sqlite3_auto_extension ?
            ffi::SQLITE_OK
            // 256
        }
        Err(e) => {
            eprintln!("[sqlite-reasonable] error: {:?}", e);
            ffi::SQLITE_ERROR
        }
    }
}

fn init(db_handle: *mut ffi::sqlite3) -> anyhow::Result<()> {
    let db = unsafe { rusqlite::Connection::from_handle(db_handle)? };
    add_functions(&db)?;
    Ok(())
}

fn add_functions(db: &rusqlite::Connection) -> anyhow::Result<()> {
    // let nondeterministic = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DIRECTONLY;
    let deterministic = FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC;
    db.create_scalar_function("reason", 1, deterministic, do_reason)?;
    Ok(())
}

fn do_reason(ctx: &Context) -> Result<Box<dyn ToSql>, rusqlite::Error> {
    
    let src_table: String = ctx.get::<String>(0)?;
    // let dst_table: String = ctx.get::<String>(1)?;
    // eprintln!("Pull triples from {}, inserting reasoned triples in {}", src_table, dst_table);

    let db = ctx.get_connection()?;
    // TODO: load in the ontology
    let mut res: Vec<(String, String, String)> = Vec::new();
    {
        let qstr = format!("SELECT subject, predicate, object FROM {}", src_table);
        let mut stmt = db.prepare(&qstr)?;
        let rows = stmt.query_map(NO_PARAMS, |row| {
                let t: (String, String, String) = (row.get(0)?, row.get(1)?, row.get(2)?);
                Ok(t)
            })?;
        for row in rows {
            res.push(row?);
        }
    }
    eprintln!("start with {} triples", res.len());

    db.execute("DROP TABLE IF EXISTS reasoned;", NO_PARAMS)?;
    db.execute("CREATE TABLE IF NOT EXISTS reasoned(subject TEXT, predicate TEXT, object TEXT);", NO_PARAMS)?;

    let mut r = owl::Reasoner::new();
    r.load_triples(res);
    r.reason();
    let reasoned = r.get_triples();
    eprintln!("now have {} triples", reasoned.len());

    for (s, p, o) in reasoned.iter() {
        db.execute("INSERT INTO reasoned(subject, predicate, object) VALUES (?, ?, ?)",
                    params![s, p, o])?;
    }

    // if let Ok(table) = ctx.get::<String>(0) {
    //     
    //     eprintln!("started with {} triples", res.len());
    //     // if let Some(r) = ctx.get_aux::<owl::Reasoner>(0 as c_int)? {
    //     //     eprintln!("reasoner");
    //     // } else {
    //     //     let mut r = owl::Reasoner::new();
    //     //     ctx.set_aux(0 as c_int, r);
    //     // }

    //     // let mut r = ctx.get_aux::<owl::Reasoner>(0 as c_int)?.unwrap();
    //     eprintln!("now have {} triples", reasoned.len());
    // }
    Ok(Box::new("a"))
}
