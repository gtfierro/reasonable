extern crate datafrog;
extern crate rdf;
use datafrog::Iteration;

use std::fs;
use std::collections::HashMap;

use std::hash::{Hash, Hasher};
use fasthash::{city, CityHasher};

use rdf::reader::turtle_parser::TurtleParser;
use rdf::reader::n_triples_parser::NTriplesParser;
use rdf::reader::rdf_parser::RdfParser;
use rdf::node::Node;
use rdf::graph::Graph;

//type URI = &'static str;
type URI = u64;


// RDFS rules
// prp-dom:
//      T(?p, rdfs:domain, ?c) AND T(?x, ?p, ?y) =>
//          T(?x, rdf:type, ?c)
// prp-rng:
//      T(?p, rdfs:range, ?c) AND T(?x ?p ?y) =>
//          T(?y, rdf:type, ?c)
// prp-fp:
//      T(?p, rdf:type, owl:FunctionalProperty) .
//      T(?x, ?p, ?y1) .
//      T(?x, ?p, ?y2) =>
//          T(?y1, owl:sameAs, ?y2) .
//   ----- rewritten -----
//      T(?p, rdf:type, owl:FunctionalProperty) .
//      T(?p, ?x, ?y1) . (pso)
//      T(?p, ?x, ?y2) . (pso) =>
//          T(?y1, owl:sameAs, ?y2) .
// prp-ifp
//      T(?p, rdf:type, owl:InverseFunctionalProperty) .
//      T(?p, ?x1, ?y) . (pso)
//      T(?p, ?x2, ?y) . (pso) =>
//          T(?x1, owl:sameAs, ?x2) .
const _NONE : (URI, ())= (0, ());
const _NONE_TUP : (URI, URI) = (0,0);
const _NONE_TRIP : (URI, (URI, URI)) = (0,(0,0));

struct URIIndex {
    map : HashMap<URI, String>
}

impl URIIndex {
    fn new() -> Self {
        let mut idx = URIIndex {
            map: HashMap::new(),
        };
        idx.map.insert(0, "_".to_string());
        idx
    }

    fn put(&mut self, key: String) -> URI {
        let h = hash(&key);
        self.map.insert(h, key);
        h
    }

    fn put_str(&mut self, _key: &'static str) -> URI {
        let key = _key.to_string();
        let h = hash(&key);
        self.map.insert(h, key);
        h
    }

    fn get(&self, key: URI) -> Option<&String> {
        self.map.get(&key)
    }
}

fn hash(key: &String) -> URI {
    city::hash64(key)
}

fn hash_str(key: &'static str) -> URI {
    let s = key.to_string();
    city::hash64(&s)
}

fn has_pred(triple: (URI, (URI, URI)), pred: URI) -> (URI, URI) {
    let (s, (p, o)) = triple;
    if p == pred {
        (s, o)
    } else {
        _NONE_TUP
    }
}

fn has_obj(triple: (URI, (URI, URI)), obj: URI) -> (URI, URI) {
    let (s, (p, o)) = triple;
    if o == obj {
        (s, p)
    } else {
        _NONE_TUP
    }
}

fn has_pred_obj(triple: (URI, (URI, URI)), predobj: (URI, URI)) -> (URI, ()) {
    let (s, (p, o)) = triple;
    let (pred, obj) = predobj;
    if p == pred && o == obj{
        (s, ())
    } else {
        _NONE
    }
}

fn load_file(filename: &str, index: &mut URIIndex) -> Vec<(URI, (URI, URI))> {
    let data = fs::read_to_string(filename).expect("Unable to read file");

    let graph: Graph = {
        if filename.ends_with(".ttl") {
            TurtleParser::from_string(data).decode().expect("bad turtle")
        } else if  filename.ends_with(".n3") {
            NTriplesParser::from_string(data).decode().expect("bad turtle")
        } else {
            panic!("no parser");
        }
    };

    //} else if filename.ends_with(".n3") {
    //    NTriplesParser::from_string(data)
    //}
    //let graph = Box::new(reader.decode().expect("bad reader"));
    //if let Ok(graph) = reader.decode() {
    println!("count: {} {}", filename, graph.count());
    graph.triples_iter().map(|_triple| {
        let triple = _triple;
        let subject = match triple.subject() {
            Node::UriNode{uri: uri} => uri.to_string(),
            Node::LiteralNode{literal: literal, data_type: _, language: _} => &literal,
            Node::BlankNode{id: id} => &id,
        };

        let predicate = match triple.predicate() {
            Node::UriNode{uri: uri} => uri.to_string(),
            Node::LiteralNode{literal: literal, data_type: _, language: _} => &literal,
            Node::BlankNode{id: id} => &id,
        };
        
        let object = match triple.object() {
            Node::UriNode{uri: uri} => uri.to_string(),
            Node::LiteralNode{literal: literal, data_type: _, language: _} => &literal,
            Node::BlankNode{id: id} => &id,
        };
        println!("{} {} {}", subject, predicate, object);

        let (s, (p, o)) = (index.put(subject.to_string()), (index.put(predicate.to_string()), index.put(object.to_string())));


        (s, (p,o))
        
    }).collect()
}

// TODO: implement lists; how do we translate this?

fn main() {
    // iteration context
    let mut iter1 = Iteration::new();

    let mut index = URIIndex::new();

    // variables within the iteration
    let spo = iter1.variable::<(URI, (URI, URI))>("spo");
    let pso = iter1.variable::<(URI, (URI, URI))>("pso");
    let osp = iter1.variable::<(URI, (URI, URI))>("osp");
    let all_triples_input = iter1.variable::<(URI, (URI, URI))>("all_triples_input");

    /* RDFS inference */

    // T(?s rdfs:domain ?o)
    let s_domain_o = iter1.variable::<(URI, URI)>("s_domain_o");
    // T(?s rdfs:range ?o)
    let s_range_o = iter1.variable::<(URI, URI)>("s_range_o");
    // T(?s rdf:type ?o)
    let y_type_c = iter1.variable::<(URI, URI)>("y_type_c");

    //prp-fp variables
    // T(?p, rdf:type, owl:FunctionalProperty
    let prp_fp_1 = iter1.variable::<(URI, ())>("prp_fp_1");
    let prp_fp_join1 = iter1.variable::<(URI, (URI, URI))>("prp_fp_2");
    let prp_fp_join2 = iter1.variable::<(URI, URI)>("prp_fp_3");
    // T(?p, ?x, ?y1), T(?p, ?x, ?y2) fulfilled from PSO index
    
    //prp-ifp variables
    // T(?p, rdf:type, owl:InverseFunctionalProperty
    let prp_ifp_1 = iter1.variable::<(URI, ())>("prp_ifp_1");
    let prp_ifp_join1 = iter1.variable::<(URI, (URI, URI))>("prp_ifp_2");
    let prp_ifp_join2 = iter1.variable::<(URI, URI)>("prp_ifp_3");
    // T(?p, ?x1, ?y), T(?p, ?x2, ?y) fulfilled from PSO index
    
    // prp-spo1
    // T(?p1, rdfs:subPropertyOf, ?p2) .
    // T(?p1, ?x, ?y) (pso) =>
    //  T(?x, ?p2, ?y)
    // IMPLS
    // T(?p1, rdfs:subPropertyOf, ?p2)
    let prp_spo1_1 = iter1.variable::<(URI, URI)>("prp_spo1_1");

    // cax-sco
    //  T(?c1, rdfs:subClassOf, ?c2)
    //  T(?c1, ?x, rdf:type) (osp) => T(?x, rdf:type, ?c2)
    //
    //  T(?c1, rdfs:subClassOf, ?c2)
    let cax_sco_1 = iter1.variable::<(URI, URI)>("cax_sco_1");
    //  T(?c1, ?x, rdf:type)
    let cax_sco_2 = iter1.variable::<(URI, URI)>("cax_sco_2");

    // prp-inv1
    // T(?p1, owl:inverseOf, ?p2)
    // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
    // prp-inv2
    // T(?p1, owl:inverseOf, ?p2)
    // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
    let owl_inverseOf = iter1.variable::<(URI, URI)>("owl_inverseOf");
    let owl_inverseOf2 = iter1.variable::<(URI, URI)>("owl_inverseOf2");
    
    // prp-symp
    //      T(?p, rdf:type, owl:SymmetricProperty)
    //      T(?x, ?p, ?y)
    //      => T(?y, ?p, ?x)
    let symmetric_properties = iter1.variable::<(URI, ())>("symmetric_properties");

    // prp-eqp1
    // T(?p1, owl:equivalentProperty, ?p2)
    // T(?x, ?p1, ?y)
    // => T(?x, ?p2, ?y)
    //
    // prp-eqp2
    // T(?p1, owl:equivalentProperty, ?p2)
    // T(?x, ?p2, ?y)
    // => T(?x, ?p1, ?y)
    let equivalent_properties = iter1.variable::<(URI, URI)>("equivalent_properties");
    let equivalent_properties_2 = iter1.variable::<(URI, URI)>("equivalent_properties_2");



    // TODO: load in datasets
    all_triples_input.insert(load_file("rdfs.ttl", &mut index).into());
    // Brick.ttl has some parse error so we use n3
    all_triples_input.insert(load_file("Brick.n3", &mut index).into());

    let v1 : Vec::<(URI, (URI, URI))> = vec![
        (index.put_str("a"), (index.put_str("rdf:type"), index.put_str("Class1"))),
        (index.put_str("b"), (index.put_str("rdf:type"), index.put_str("Class1"))),
        (index.put_str("Class1"), (index.put_str("rdfs:subClassOf"), index.put_str("Class2"))),
        (index.put_str("Class3"), (index.put_str("rdfs:subClassOf"), index.put_str("Class2"))),
        (index.put_str("brick:feeds"), (index.put_str("rdfs:domain"), index.put_str("Class2"))),
        (index.put_str("brick:feeds"), (index.put_str("rdfs:range"), index.put_str("Class3"))),
        (index.put_str("brick:isFedBy"), (index.put_str("owl:inverseOf"), index.put_str("brick:feeds"))),
        (index.put_str("c"), (index.put_str("brick:feeds"), index.put_str("d"))),

        // cls-thing
        (index.put_str("owl:Thing"), (index.put_str("rdf:type"), index.put_str("owl:Class"))),
        // cls-nothing1
        (index.put_str("owl:Nothing"), (index.put_str("rdf:type"), index.put_str("owl:Class"))),

        // owl definitions
        (index.put_str("owl:inverseOf"), (index.put_str("rdf:type"), index.put_str("owl:SymmetricProperty"))),
    ];
    //all_triples_input.insert(v1.into());

    while iter1.changed() {

        spo.from_map(&all_triples_input, |&(sub, (pred, obj))| (sub, (pred, obj)));
        pso.from_map(&all_triples_input, |&(sub, (pred, obj))| (pred, (sub, obj)));
        osp.from_map(&all_triples_input, |&(sub, (pred, obj))| (obj, (sub, pred)));

        y_type_c.from_map(&spo, |&triple| { has_pred(triple, index.put_str("rdf:type")) });
        s_domain_o.from_map(&spo, |&triple| { has_pred(triple, index.put_str("rdfs:domain")) });
        s_range_o.from_map(&spo, |&triple| { has_pred(triple, index.put_str("rdfs:range")) });

        owl_inverseOf.from_map(&spo, |&triple| has_pred(triple, index.put_str("owl:inverseOf")) );
        owl_inverseOf2.from_map(&owl_inverseOf, |&(p1, p2)| (p2, p1) );
        
        symmetric_properties.from_map(&spo, |&triple| has_pred_obj(triple, (index.put_str("rdf:type"), index.put_str("owl:SymmetricProperty"))) );

        equivalent_properties.from_map(&spo, |&triple| has_pred(triple, index.put_str("owl:equivalentProperty")) );
        equivalent_properties_2.from_map(&equivalent_properties, |&(p1, p2)| (p2, p1));

        all_triples_input.from_join(&s_domain_o, &pso, |&tpred, &class, &(sub, obj)| {
            (sub, (index.put_str("rdf:type"), class))
        });
        all_triples_input.from_join(&s_range_o, &pso, |&tpred, &class, &(sub, obj)| {
            (obj, (index.put_str("rdf:type"), class))
        });

        // prp-fp
        prp_fp_1.from_map(&spo, |&triple| { has_pred_obj(triple, (index.put_str("rdf:type"), index.put_str("owl:FunctionalProperty"))) });
        prp_fp_join1.from_join(&prp_fp_1, &spo, |&p, &(), &(x, y1)| (p, (x, y1)) );
        prp_fp_join2.from_join(&prp_fp_join1, &spo, |&p, &(x1, y2), &(x2, y1)| (y1, y2) );
        //all_triples_input.from_map(&prp_fp_join2, |&(y1, y2)| (y1, (index.put_str("owl:sameAs"), y2)));
        
        // prp-ifp
        prp_ifp_1.from_map(&spo, |&triple| { has_pred_obj(triple, (index.put_str("rdf:type"), index.put_str("owl:InverseFunctionalProperty"))) });
        prp_ifp_join1.from_join(&prp_ifp_1, &spo, |&p, &(), &(x1, y)| (p, (x1, y)) );
        prp_ifp_join2.from_join(&prp_ifp_join1, &spo, |&p, &(x1, y2), &(x2, y1)| (x1, x2) );
        //all_triples_input.from_map(&prp_ifp_join2, |&(x1, x2)| (x1, (index.put_str("owl:sameAs"), x2)));

        // prp-spo1
        prp_spo1_1.from_map(&spo, |&triple| has_pred(triple, index.put_str("rdfs:subPropertyOf")));
        all_triples_input.from_join(&prp_spo1_1, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)));

        // cax-sco
        cax_sco_1.from_map(&spo, |&triple| has_pred(triple, index.put_str("rdfs:subClassOf")));
        // ?c1, ?x, rdf:type
        cax_sco_2.from_map(&y_type_c, |&(inst, class)| (class, inst));
        all_triples_input.from_join(&cax_sco_1, &cax_sco_2, |&class, &parent, &inst| (inst, (index.put_str("rdf:type"), parent)));


        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        all_triples_input.from_join(&owl_inverseOf, &pso, |&p1, &p2, &(x, y)| (y, (p2, x)) );

        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        all_triples_input.from_join(&owl_inverseOf2, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );

        // prp-symp
        // T(?p, rdf:type, owl:SymmetricProperty)
        // T(?x, ?p, ?y)
        //  => T(?y, ?p, ?x)
        all_triples_input.from_join(&symmetric_properties, &pso, |&prop, &(), &(x, y)| (y, (prop, x)) );

        // prp-eqp1
        // T(?p1, owl:equivalentProperty, ?p2)
        // T(?x, ?p1, ?y)
        // => T(?x, ?p2, ?y)
        all_triples_input.from_join(&equivalent_properties, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );
        // prp-eqp2
        // T(?p1, owl:equivalentProperty, ?p2)
        // T(?x, ?p2, ?y)
        // => T(?x, ?p1, ?y)
        all_triples_input.from_join(&equivalent_properties_2, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );

    }

    let instances = spo.complete();

    for inst in instances.iter() {
        let (_s, (_p, _o)) = inst;
        let s = index.get(*_s).unwrap();
        let p = index.get(*_p).unwrap();
        let o = index.get(*_o).unwrap();
        println!("{} {} {}", s, p, o)
    }

}
