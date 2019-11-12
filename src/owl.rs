extern crate datafrog;
use datafrog::{Iteration, Variable};

use crate::index::URIIndex;
use crate::types::{URI, has_pred, has_obj, has_pred_obj};

use std::fs;
use rdf::reader::turtle_parser::TurtleParser;
use rdf::reader::n_triples_parser::NTriplesParser;
use rdf::reader::rdf_parser::RdfParser;
use rdf::node::Node;
use rdf::graph::Graph;

pub struct Reasoner {
    iter1: Iteration,
    index: URIIndex,

    spo: Variable<(URI, (URI, URI))>,
    pso: Variable<(URI, (URI, URI))>,
    osp: Variable<(URI, (URI, URI))>,
    all_triples_input: Variable<(URI, (URI, URI))>,

    prp_dom: Variable<(URI, URI)>,
    prp_rng: Variable<(URI, URI)>,
    rdf_type: Variable<(URI, URI)>,

    prp_fp_1: Variable<(URI, ())>,
    prp_fp_join1: Variable<(URI, (URI, URI))>,
    prp_fp_join2: Variable<(URI, URI)>,

    prp_ifp_1: Variable<(URI, ())>,
    prp_ifp_join1: Variable<(URI, (URI, URI))>,
    prp_ifp_join2: Variable<(URI, URI)>,

    prp_spo1_1: Variable<(URI, URI)>,

    cax_sco_1: Variable<(URI, URI)>,
    cax_sco_2: Variable<(URI, URI)>,

    owl_inverseOf: Variable<(URI, URI)>,
    owl_inverseOf2: Variable<(URI, URI)>,

    symmetric_properties: Variable<(URI, ())>,

    equivalent_properties: Variable<(URI, URI)>,
    equivalent_properties_2: Variable<(URI, URI)>,
}

impl Reasoner {
    pub fn new() -> Self {
        let mut iter1 = Iteration::new();
        let mut index = URIIndex::new();

        // variables within the iteration
        let spo = iter1.variable::<(URI, (URI, URI))>("spo");
        let pso = iter1.variable::<(URI, (URI, URI))>("pso");
        let osp = iter1.variable::<(URI, (URI, URI))>("osp");
        let all_triples_input = iter1.variable::<(URI, (URI, URI))>("all_triples_input");

        let prp_dom = iter1.variable::<(URI, URI)>("prp_dom");
        let prp_rng = iter1.variable::<(URI, URI)>("prp_rng");
        let rdf_type = iter1.variable::<(URI, URI)>("rdf_type");
 
        //prp-fp variables
        // T(?p, rdf:type, owl:FunctionalProperty
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
        let prp_fp_1 = iter1.variable::<(URI, ())>("prp_fp_1");
        let prp_fp_join1 = iter1.variable::<(URI, (URI, URI))>("prp_fp_2");
        let prp_fp_join2 = iter1.variable::<(URI, URI)>("prp_fp_3");
        // T(?p, ?x, ?y1), T(?p, ?x, ?y2) fulfilled from PSO index

        // T(?p, rdf:type, owl:InverseFunctionalProperty
        // prp-ifp
        //      T(?p, rdf:type, owl:InverseFunctionalProperty) .
        //      T(?p, ?x1, ?y) . (pso)
        //      T(?p, ?x2, ?y) . (pso) =>
        //          T(?x1, owl:sameAs, ?x2) .
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

        Reasoner {
            iter1: iter1,
            index: index,
            spo: spo,
            pso: pso,
            osp: osp,
            all_triples_input: all_triples_input,

            prp_dom: prp_dom,
            prp_rng: prp_rng,
            rdf_type: rdf_type,

            prp_fp_1: prp_fp_1,
            prp_fp_join1: prp_fp_join1,
            prp_fp_join2: prp_fp_join2,

            prp_ifp_1: prp_ifp_1,
            prp_ifp_join1: prp_ifp_join1,
            prp_ifp_join2: prp_ifp_join2,

            prp_spo1_1: prp_spo1_1,
            cax_sco_1: cax_sco_1,
            cax_sco_2: cax_sco_2,
            owl_inverseOf: owl_inverseOf,
            owl_inverseOf2: owl_inverseOf2,
            symmetric_properties: symmetric_properties,
            equivalent_properties: equivalent_properties,
            equivalent_properties_2: equivalent_properties_2,
        }
    }

    pub fn load_file(&mut self, filename: &str) -> Result<(), String> {
        let data = fs::read_to_string(filename).expect("Unable to read file");

        let graph: Graph = {
            if filename.ends_with(".ttl") {
                TurtleParser::from_string(data).decode().expect("bad turtle")
            } else if  filename.ends_with(".n3") {
                NTriplesParser::from_string(data).decode().expect("bad turtle")
            } else {
                return Err("no parser for file".to_string());
            }
        };

        //} else if filename.ends_with(".n3") {
        //    NTriplesParser::from_string(data)
        //}
        //let graph = Box::new(reader.decode().expect("bad reader"));
        //if let Ok(graph) = reader.decode() {
        println!("count: {} {}", filename, graph.count());
        let triples : Vec<(URI, (URI, URI))> = graph.triples_iter().map(|_triple| {
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

            let (s, (p, o)) = (self.index.put(subject.to_string()), (self.index.put(predicate.to_string()), self.index.put(object.to_string())));


            (s, (p,o))

        }).collect();

        self.all_triples_input.insert(triples.into());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_reasoner() -> Result<(), String> {
        let r = Reasoner::new();
        Ok(())
    }

    #[test]
    fn test_load_file_ttl() -> Result<(), String> {
        let mut r = Reasoner::new();
        r.load_file("rdfs.ttl")
    }

    #[test]
    fn test_load_file_n3() -> Result<(), String> {
        let mut r = Reasoner::new();
        r.load_file("Brick.n3")
    }
}
