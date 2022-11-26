//use crate::manager::{Manager, TripleUpdate};
use crate::reasoner::*;

use std::io::Error;

const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
const RDFS_SUBPROP: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
const OWL_SAMEAS: &str = "http://www.w3.org/2002/07/owl#sameAs";
const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
const OWL_HASVALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
const OWL_ALLVALUESFROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
const OWL_SOMEVALUESFROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
const OWL_SYMMETRICPROP: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
const OWL_EQUIVPROP: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
const OWL_FUNCPROP: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
const OWL_INVFUNCPROP: &str = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
const OWL_TRANSPROP: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
const OWL_INTERSECTION: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
const OWL_UNION: &str = "http://www.w3.org/2002/07/owl#unionOf";
const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
const OWL_COMPLEMENT: &str = "http://www.w3.org/2002/07/owl#complementOf";
const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
const OWL_ASYMMETRICPROP: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";

macro_rules! wrap {
    ($t:expr) => {
        format!("<{}>", $t)
    };
}

macro_rules! triple {
    ($s:expr, $p:expr, $o:expr) => {
        make_triple(uri!("urn:{}", $s), uri!("urn:{}", $p), uri!("urn:{}", $o)).unwrap()
    };
}

#[test]
fn test_make_reasoner() -> Result<(), Error> {
    let _r = Reasoner::new();
    Ok(())
}

#[test]
fn test_load_file_ttl() -> Result<(), Error> {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/rdfs.ttl")
}

#[test]
fn test_load_file_n3() -> Result<(), Error> {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/Brick.n3")
}

#[test]
#[ignore]
fn test_eq_ref() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![("a", "b", "c")];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), wrap!(OWL_SAMEAS), "a".to_string())));
    assert!(res.contains(&("b".to_string(), wrap!(OWL_SAMEAS), "b".to_string())));
    assert!(res.contains(&("c".to_string(), wrap!(OWL_SAMEAS), "c".to_string())));
    Ok(())
}

#[test]
fn test_eq_sym() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![("urn:x", OWL_SAMEAS, "urn:y")];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(OWL_SAMEAS),
        "<urn:x>".to_string()
    )));
    Ok(())
}

#[test]
fn test_eq_trans() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:x", OWL_SAMEAS, "urn:y"),
        ("urn:y", OWL_SAMEAS, "urn:z"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:x>".to_string(),
        wrap!(OWL_SAMEAS),
        "<urn:z>".to_string()
    )));
    Ok(())
}

#[test]
fn test_eq_rep_s() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:s1", OWL_SAMEAS, "urn:s2"),
        ("urn:s1", "urn:p", "urn:o"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:s2>".to_string(),
        "<urn:p>".to_string(),
        "<urn:o>".to_string()
    )));
    Ok(())
}

#[test]
fn test_eq_rep_p() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p1", OWL_SAMEAS, "urn:p2"),
        ("urn:s", "urn:p1", "urn:o"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:s>".to_string(),
        "<urn:p2>".to_string(),
        "<urn:o>".to_string()
    )));
    Ok(())
}

#[test]
fn test_eq_rep_o() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:o1", OWL_SAMEAS, "urn:o2"),
        ("urn:s", "urn:p", "urn:o1"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:s>".to_string(),
        "<urn:p>".to_string(),
        "<urn:o2>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cax_sco() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:Class2", RDFS_SUBCLASSOF, "urn:Class1"),
        ("urn:a", RDF_TYPE, "urn:Class2"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let _res = r.get_triples();
    let res: Vec<(String, String, String)> = _res
        .iter()
        .map(|t| {
            (
                t.subject.to_string(),
                t.predicate.to_string(),
                t.object.to_string(),
            )
        })
        .collect();
    // let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class1>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cax_eqc1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:a", RDF_TYPE, "urn:Class1"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class2>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cax_eqc2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:a", RDF_TYPE, "urn:Class2"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class1>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cax_eqc_chain_1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:Class2", OWL_EQUIVALENTCLASS, "urn:Class3"),
        ("urn:Class3", OWL_EQUIVALENTCLASS, "urn:Class4"),
        ("urn:Class4", OWL_EQUIVALENTCLASS, "urn:Class5"),
        ("urn:Class5", OWL_EQUIVALENTCLASS, "urn:Class6"),
        ("urn:a", RDF_TYPE, "urn:Class1"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class3>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class4>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class5>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class6>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cax_eqc_chain_2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:Class1", OWL_EQUIVALENTCLASS, "urn:Class2"),
        ("urn:Class2", OWL_EQUIVALENTCLASS, "urn:Class3"),
        ("urn:Class3", OWL_EQUIVALENTCLASS, "urn:Class4"),
        ("urn:Class4", OWL_EQUIVALENTCLASS, "urn:Class5"),
        ("urn:Class5", OWL_EQUIVALENTCLASS, "urn:Class6"),
        ("urn:a", RDF_TYPE, "urn:Class6"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class1>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class3>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class4>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:a>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:Class5>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_fp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:PRED", RDF_TYPE, OWL_FUNCPROP),
        ("urn:SUB", "urn:PRED", "urn:OBJECT_1"),
        ("urn:SUB", "urn:PRED", "urn:OBJECT_2"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:OBJECT_1>".to_string(),
        wrap!(OWL_SAMEAS),
        "<urn:OBJECT_2>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_fp_2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:PRED", RDF_TYPE, OWL_FUNCPROP),
        ("urn:SUB", "urn:PRED", "urn:OBJECT_1"),
        ("urn:SUB1", "urn:PRED", "urn:OBJECT_2"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(!res.contains(&(
        "<urn:OBJECT_1>".to_string(),
        wrap!(OWL_SAMEAS),
        "<urn:OBJECT_2>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_ifp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:PRED", RDF_TYPE, OWL_INVFUNCPROP),
        ("urn:SUB_1", "urn:PRED", "urn:OBJECT"),
        ("urn:SUB_2", "urn:PRED", "urn:OBJECT"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:SUB_1>".to_string(),
        wrap!(OWL_SAMEAS),
        "<urn:SUB_2>".to_string()
    )));
    Ok(())
}

#[test]
fn test_spo1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p1", RDFS_SUBPROP, "urn:p2"),
        ("urn:x", "urn:p1", "urn:y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:x>".to_string(),
        "<urn:p2>".to_string(),
        "<urn:y>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_inv1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p1", OWL_INVERSEOF, "urn:p2"),
        ("urn:x", "urn:p1", "urn:y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        "<urn:p2>".to_string(),
        "<urn:x>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_symp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p", RDF_TYPE, OWL_SYMMETRICPROP),
        ("urn:x", "urn:p", "urn:y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        "<urn:p>".to_string(),
        "<urn:x>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_trp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p", RDF_TYPE, OWL_TRANSPROP),
        ("urn:x", "urn:p", "urn:y"),
        ("urn:y", "urn:p", "urn:z"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:x>".to_string(),
        "<urn:p>".to_string(),
        "<urn:z>".to_string()
    )));
    Ok(())
}

#[test]
fn test_prp_eqp1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p1", OWL_EQUIVPROP, "urn:p2"),
        ("urn:x", "urn:p1", "urn:y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:x>".to_string(),
        "<urn:p2>".to_string(),
        "<urn:y>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_thing_nothing() -> Result<(), String> {
    let mut r = Reasoner::new();
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(wrap!(OWL_THING), wrap!(RDF_TYPE), wrap!(OWL_CLASS))));
    assert!(res.contains(&(wrap!(OWL_NOTHING), wrap!(RDF_TYPE), wrap!(OWL_CLASS))));
    Ok(())
}

#[test]
fn test_cls_hv1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:x", OWL_HASVALUE, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", RDF_TYPE, "urn:x"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:u>".to_string(),
        "<urn:p>".to_string(),
        "<urn:y>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_hv2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:x", OWL_HASVALUE, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", "urn:p", "urn:y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:u>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:x>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_avf() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:x", OWL_ALLVALUESFROM, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", RDF_TYPE, "urn:x"),
        ("urn:u", "urn:p", "urn:v"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:v>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:y>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_svf1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:x", OWL_SOMEVALUESFROM, "urn:y"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", "urn:p", "urn:v"),
        ("urn:v", RDF_TYPE, "urn:y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:u>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:x>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_svf2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:x", OWL_SOMEVALUESFROM, OWL_THING),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:u", "urn:p", "urn:v"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:u>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:x>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_int1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:c", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, "urn:z3"),
        ("urn:z3", RDF_FIRST, "urn:c3"),
        ("urn:z3", RDF_REST, RDF_NIL),
        ("urn:y", RDF_TYPE, "urn:c1"),
        ("urn:y", RDF_TYPE, "urn:c2"),
        ("urn:y", RDF_TYPE, "urn:c3"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_int2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:c", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, "urn:z3"),
        ("urn:z3", RDF_FIRST, "urn:c3"),
        ("urn:z3", RDF_REST, RDF_NIL),
        ("urn:y", RDF_TYPE, "urn:c"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c1>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c3>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_int2_withequivalent() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:c", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, "urn:z3"),
        ("urn:z3", RDF_FIRST, "urn:c3"),
        ("urn:z3", RDF_REST, RDF_NIL),
        ("urn:y", RDF_TYPE, "urn:c"),
        ("urn:c", OWL_EQUIVALENTCLASS, "urn:C"),
        ("urn:C", OWL_INTERSECTION, "urn:X"),
        ("urn:X", RDF_FIRST, "urn:C1"),
        ("urn:X", RDF_REST, "urn:Z2"),
        ("urn:Z2", RDF_FIRST, "urn:C2"),
        ("urn:Z2", RDF_REST, "urn:Z3"),
        ("urn:Z3", RDF_FIRST, "urn:C3"),
        ("urn:Z3", RDF_REST, RDF_NIL),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c1>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c3>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:C1>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:C2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:y>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:C3>".to_string()
    )));
    Ok(())
}

#[test]
fn test_cls_int1_withhasvalue() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:intersection_class", OWL_INTERSECTION, "urn:x"),
        ("urn:x", RDF_FIRST, "urn:c1"),
        ("urn:x", RDF_REST, "urn:z2"),
        ("urn:z2", RDF_FIRST, "urn:c2"),
        ("urn:z2", RDF_REST, RDF_NIL),
        ("urn:c1", OWL_HASVALUE, "urn:c1p_value"),
        ("urn:c1", OWL_ONPROPERTY, "urn:c1p"),
        ("urn:c2", OWL_HASVALUE, "urn:c2p_value"),
        ("urn:c2", OWL_ONPROPERTY, "urn:c2p"),
        ("urn:inst", "urn:c1p", "urn:c1p_value"),
        ("urn:inst", "urn:c2p", "urn:c2p_value"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&(
        "<urn:inst>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c1>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:inst>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:inst>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:intersection_class>".to_string()
    )));
    Ok(())
}

#[test]
fn test_complementof() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:c", OWL_EQUIVALENTCLASS, "urn:c2"),
        ("urn:c2", OWL_COMPLEMENT, "urn:x"),
        ("urn:x", OWL_HASVALUE, "urn:v"),
        ("urn:x", OWL_ONPROPERTY, "urn:p"),
        ("urn:inst1", "urn:p", "urn:v"),
        ("urn:inst2", "urn:p", "urn:v2"),
        ("urn:x", RDF_TYPE, OWL_CLASS),
        ("urn:c", RDF_TYPE, OWL_CLASS),
        ("urn:c2", RDF_TYPE, OWL_CLASS),
        ("urn:inst2", RDF_TYPE, OWL_THING),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("<urn:inst1>".to_string(), wrap!(RDF_TYPE), wrap!(OWL_THING))));
    assert!(res.contains(&(
        "<urn:inst1>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:x>".to_string()
    )));
    assert!(!res.contains(&(
        "<urn:inst1>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c>".to_string()
    )));
    assert!(!res.contains(&(
        "<urn:inst1>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c2>".to_string()
    )));
    assert!(res.contains(&(
        "<urn:inst2>".to_string(),
        wrap!(RDF_TYPE),
        "<urn:c2>".to_string()
    )));
    Ok(())
}

#[test]
fn test_error_asymmetric() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("urn:p", RDF_TYPE, OWL_ASYMMETRICPROP),
        ("urn:x", "urn:p", "urn:y"),
        ("urn:y", "urn:p", "urn:x"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    // assert!(res.errors.len() > 0);
    Ok(())
}

//#[test]
//fn test_triple_update1() -> Result<(), String> {
//    let mut u = TripleUpdate::new();
//    let t1 = triple!("x1", "y1", "z1");
//    let t1rem = triple!("x1", "y1", "z1");
//    let t2 = triple!("x2", "y2", "z2");
//    u.add_triple(t1);
//    u.add_triple(t2);
//    u.remove_triple(t1rem);
//    assert_eq!(*u.updates.get(&triple!("x1", "y1", "z1")).unwrap(), 0);
//    assert_eq!(*u.updates.get(&triple!("x2", "y2", "z2")).unwrap(), 1);
//    Ok(())
//}
//
//#[test]
//fn test_triple_update2() -> Result<(), String> {
//    let mut u = TripleUpdate::new();
//    let t1 = triple!("x1", "y1", "z1");
//    let t2 = triple!("x2", "y2", "z2");
//    u.add_triple(t1);
//    u.add_triple(t2);
//    assert_eq!(*u.updates.get(&triple!("x1", "y1", "z1")).unwrap(), 1);
//    assert_eq!(*u.updates.get(&triple!("x2", "y2", "z2")).unwrap(), 1);
//    Ok(())
//}
//
//#[test]
//fn test_triple_update3() -> Result<(), String> {
//    let mut u = TripleUpdate::new();
//    let t1a = triple!("x1", "y1", "z1");
//    let t1b = triple!("x1", "y1", "z1");
//    let t1c = triple!("x1", "y1", "z1");
//    let t2 = triple!("x2", "y2", "z2");
//    u.add_triple(t1a);
//    u.add_triple(t1b);
//    u.add_triple(t2);
//    assert_eq!(*u.updates.get(&triple!("x1", "y1", "z1")).unwrap(), 2);
//    assert_eq!(*u.updates.get(&triple!("x2", "y2", "z2")).unwrap(), 1);
//    u.remove_triple(t1c);
//    assert_eq!(*u.updates.get(&triple!("x1", "y1", "z1")).unwrap(), 1);
//    Ok(())
//}
//
//#[test]
//fn test_manager_update() -> Result<(), String> {
//    let mut mgr = Manager::new_in_memory();
//
//    let mut u = TripleUpdate::new();
//    u.add_triple(triple!("x1", "y1", "z1"));
//
//    assert_eq!(mgr.size(), 0);
//    mgr.process_updates(u);
//    assert_eq!(mgr.size(), 6);
//
//    let mut u = TripleUpdate::new();
//    u.remove_triple(triple!("x1", "y1", "z1"));
//    mgr.process_updates(u);
//    println!("{}", mgr.dump_string());
//    assert_eq!(mgr.size(), 4);
//
//    let mut u = TripleUpdate::new();
//    u.add_triple(triple!("x3", "y3", "z3"));
//    u.add_triple(triple!("x1", "y1", "z1"));
//    mgr.process_updates(u);
//    assert_eq!(mgr.size(), 8);
//
//    Ok(())
//}
