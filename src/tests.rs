use crate::owl::*;
use crate::common::*;

#[test]
fn test_make_reasoner() -> Result<(), String> {
    let _r = Reasoner::new();
    Ok(())
}

#[test]
fn test_load_file_ttl() -> Result<(), String> {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/rdfs.ttl")
}

#[test]
fn test_load_file_n3() -> Result<(), String> {
    let mut r = Reasoner::new();
    r.load_file("example_models/ontologies/Brick.n3")
}

#[test]
#[ignore]
fn test_eq_ref() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("a", "b", "c")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), OWL_SAMEAS.to_string(), "a".to_string())));
    assert!(res.contains(&("b".to_string(), OWL_SAMEAS.to_string(), "b".to_string())));
    assert!(res.contains(&("c".to_string(), OWL_SAMEAS.to_string(), "c".to_string())));
    Ok(())
}

#[test]
fn test_eq_sym() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_SAMEAS, "y")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("y".to_string(), OWL_SAMEAS.to_string(), "x".to_string())));
    Ok(())
}

#[test]
fn test_eq_trans() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_SAMEAS, "y"),
        ("y", OWL_SAMEAS, "z"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("x".to_string(), OWL_SAMEAS.to_string(), "z".to_string())));
    Ok(())
}

#[test]
fn test_eq_rep_s() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("s1", OWL_SAMEAS, "s2"),
        ("s1", "p", "o"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("s2".to_string(), "p".to_string(), "o".to_string())));
    Ok(())
}

#[test]
fn test_eq_rep_p() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p1", OWL_SAMEAS, "p2"),
        ("s", "p1", "o"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("s".to_string(), "p2".to_string(), "o".to_string())));
    Ok(())
}

#[test]
fn test_eq_rep_o() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("o1", OWL_SAMEAS, "o2"),
        ("s", "p", "o1"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("s".to_string(), "p".to_string(), "o2".to_string())));
    Ok(())
}

#[test]
fn test_cax_sco() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("Class2", RDFS_SUBCLASSOF, "Class1"),
        ("a", RDF_TYPE, "Class2")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class1".to_string())));
    Ok(())
}

#[test]
fn test_cax_eqc1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("Class1", OWL_EQUIVALENTCLASS, "Class2"),
        ("a", RDF_TYPE, "Class1")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class2".to_string())));
    Ok(())
}

#[test]
fn test_cax_eqc2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("Class1", OWL_EQUIVALENTCLASS, "Class2"),
        ("a", RDF_TYPE, "Class2")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class1".to_string())));
    Ok(())
}

#[test]
fn test_cax_eqc_chain_1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("Class1", OWL_EQUIVALENTCLASS, "Class2"),
        ("Class2", OWL_EQUIVALENTCLASS, "Class3"),
        ("Class3", OWL_EQUIVALENTCLASS, "Class4"),
        ("Class4", OWL_EQUIVALENTCLASS, "Class5"),
        ("Class5", OWL_EQUIVALENTCLASS, "Class6"),
        ("a", RDF_TYPE, "Class1")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class2".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class3".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class4".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class5".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class6".to_string())));
    Ok(())
}

#[test]
fn test_cax_eqc_chain_2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("Class1", OWL_EQUIVALENTCLASS, "Class2"),
        ("Class2", OWL_EQUIVALENTCLASS, "Class3"),
        ("Class3", OWL_EQUIVALENTCLASS, "Class4"),
        ("Class4", OWL_EQUIVALENTCLASS, "Class5"),
        ("Class5", OWL_EQUIVALENTCLASS, "Class6"),
        ("a", RDF_TYPE, "Class6")
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class1".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class2".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class3".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class4".to_string())));
    assert!(res.contains(&("a".to_string(), RDF_TYPE.to_string(), "Class5".to_string())));
    Ok(())
}


#[test]
fn test_prp_fp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("PRED", RDF_TYPE, OWL_FUNCPROP),
        ("SUB", "PRED", "OBJECT_1"),
        ("SUB", "PRED", "OBJECT_2"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("OBJECT_1".to_string(), OWL_SAMEAS.to_string(), "OBJECT_2".to_string())));
    Ok(())
}

#[test]
fn test_prp_ifp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("PRED", RDF_TYPE, OWL_INVFUNCPROP),
        ("SUB_1", "PRED", "OBJECT"),
        ("SUB_2", "PRED", "OBJECT"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("SUB_1".to_string(), OWL_SAMEAS.to_string(), "SUB_2".to_string())));
    Ok(())
}

#[test]
fn test_spo1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p1", RDFS_SUBPROP, "p2"),
        ("x", "p1", "y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("x".to_string(), "p2".to_string(), "y".to_string())));
    Ok(())
}

#[test]
fn test_prp_inv1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p1", OWL_INVERSEOF, "p2"),
        ("x", "p1", "y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("y".to_string(), "p2".to_string(), "x".to_string())));
    Ok(())
}

#[test]
fn test_prp_symp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p", RDF_TYPE, OWL_SYMMETRICPROP),
        ("x", "p", "y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("y".to_string(), "p".to_string(), "x".to_string())));
    Ok(())
}

#[test]
fn test_prp_trp() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p", RDF_TYPE, OWL_TRANSPROP),
        ("x", "p", "y"),
        ("y", "p", "z"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("x".to_string(), "p".to_string(), "z".to_string())));
    Ok(())
}

#[test]
fn test_prp_eqp1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p1", OWL_EQUIVPROP, "p2"),
        ("x", "p1", "y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("x".to_string(), "p2".to_string(), "y".to_string())));
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
    assert!(res.contains(&(OWL_THING.to_string(), RDF_TYPE.to_string(), OWL_CLASS.to_string())));
    assert!(res.contains(&(OWL_NOTHING.to_string(), RDF_TYPE.to_string(), OWL_CLASS.to_string())));
    Ok(())
}

#[test]
fn test_cls_hv1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_HASVALUE, "y"),
        ("x", OWL_ONPROPERTY, "p"),
        ("u", RDF_TYPE, "x"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("u".to_string(), "p".to_string(), "y".to_string())));
    Ok(())
}

#[test]
fn test_cls_hv2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_HASVALUE, "y"),
        ("x", OWL_ONPROPERTY, "p"),
        ("u", "p", "y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("u".to_string(), RDF_TYPE.to_string(), "x".to_string())));
    Ok(())
}

#[test]
fn test_cls_avf() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_ALLVALUESFROM, "y"),
        ("x", OWL_ONPROPERTY, "p"),
        ("u", RDF_TYPE, "x"),
        ("u", "p", "v"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("v".to_string(), RDF_TYPE.to_string(), "y".to_string())));
    Ok(())
}

#[test]
fn test_cls_svf1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_SOMEVALUESFROM, "y"),
        ("x", OWL_ONPROPERTY, "p"),
        ("u", "p", "v"),
        ("v", RDF_TYPE, "y"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("u".to_string(), RDF_TYPE.to_string(), "x".to_string())));
    Ok(())
}

#[test]
fn test_cls_svf2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("x", OWL_SOMEVALUESFROM, OWL_THING),
        ("x", OWL_ONPROPERTY, "p"),
        ("u", "p", "v"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("u".to_string(), RDF_TYPE.to_string(), "x".to_string())));
    Ok(())
}

#[test]
fn test_cls_int1() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("c", OWL_INTERSECTION, "x"),
        ("x", RDF_FIRST, "c1"),
        ("x", RDF_REST, "z2"),
        ("z2", RDF_FIRST, "c2"),
        ("z2", RDF_REST, "z3"),
        ("z3", RDF_FIRST, "c3"),
        ("z3", RDF_REST, RDF_NIL),
        ("y", RDF_TYPE, "c1"),
        ("y", RDF_TYPE, "c2"),
        ("y", RDF_TYPE, "c3"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c".to_string())));
    Ok(())
}

#[test]
fn test_cls_int2() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("c", OWL_INTERSECTION, "x"),
        ("x", RDF_FIRST, "c1"),
        ("x", RDF_REST, "z2"),
        ("z2", RDF_FIRST, "c2"),
        ("z2", RDF_REST, "z3"),
        ("z3", RDF_FIRST, "c3"),
        ("z3", RDF_REST, RDF_NIL),
        ("y", RDF_TYPE, "c"),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c1".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c3".to_string())));
    Ok(())
}

#[test]
fn test_cls_int2_withequivalent() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("c", OWL_INTERSECTION, "x"),
        ("x", RDF_FIRST, "c1"),
        ("x", RDF_REST, "z2"),
        ("z2", RDF_FIRST, "c2"),
        ("z2", RDF_REST, "z3"),
        ("z3", RDF_FIRST, "c3"),
        ("z3", RDF_REST, RDF_NIL),
        ("y", RDF_TYPE, "c"),

        ("c", OWL_EQUIVALENTCLASS, "C"),

        ("C", OWL_INTERSECTION, "X"),
        ("X", RDF_FIRST, "C1"),
        ("X", RDF_REST, "Z2"),
        ("Z2", RDF_FIRST, "C2"),
        ("Z2", RDF_REST, "Z3"),
        ("Z3", RDF_FIRST, "C3"),
        ("Z3", RDF_REST, RDF_NIL),

    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c1".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "c3".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "C1".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "C2".to_string())));
    assert!(res.contains(&("y".to_string(), RDF_TYPE.to_string(), "C3".to_string())));
    Ok(())
}

#[test]
fn test_cls_int1_withhasvalue() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("intersection_class", OWL_INTERSECTION, "x"),
        ("x", RDF_FIRST, "c1"),
        ("x", RDF_REST, "z2"),
        ("z2", RDF_FIRST, "c2"),
        ("z2", RDF_REST, RDF_NIL),

        ("c1", OWL_HASVALUE, "c1p_value"),
        ("c1", OWL_ONPROPERTY, "c1p"),
        ("c2", OWL_HASVALUE, "c2p_value"),
        ("c2", OWL_ONPROPERTY, "c2p"),

        ("inst", "c1p", "c1p_value"),
        ("inst", "c2p", "c2p_value"),

    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("inst".to_string(), RDF_TYPE.to_string(), "c1".to_string())));
    assert!(res.contains(&("inst".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
    assert!(res.contains(&("inst".to_string(), RDF_TYPE.to_string(), "intersection_class".to_string())));
    Ok(())
}

#[test]
fn test_complementof() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("c", OWL_EQUIVALENTCLASS, "c2"),
        ("c2", OWL_COMPLEMENT, "x"),
        ("x", OWL_HASVALUE, "v"),
        ("x", OWL_ONPROPERTY, "p"),
        ("inst1", "p", "v"),
        ("inst2", "p", "v2"),
        ("x", RDF_TYPE, OWL_CLASS),
        ("c", RDF_TYPE, OWL_CLASS),
        ("c2", RDF_TYPE, OWL_CLASS),
        ("inst2", RDF_TYPE, OWL_THING),
    ];
    r.load_triples_str(trips);
    r.reason();
    let res = r.get_triples_string();
    for i in res.iter() {
        let (s, p, o) = i;
        println!("{} {} {}", s, p, o);
    }
    assert!(res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), OWL_THING.to_string())));
    assert!(res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), "x".to_string())));
    assert!(!res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), "c".to_string())));
    assert!(!res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
    assert!(res.contains(&("inst2".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
    Ok(())
}

#[test]
fn test_error_asymmetric() -> Result<(), String> {
    let mut r = Reasoner::new();
    let trips = vec![
        ("p", RDF_TYPE, OWL_ASYMMETRICPROP),
        ("x", "p", "y"),
        ("y", "p", "x"),
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
