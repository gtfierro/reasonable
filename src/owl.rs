extern crate datafrog;
use datafrog::{Iteration, Variable};

use crate::index::URIIndex;
use crate::disjoint_sets::DisjointSets;
use crate::common::*;

use log::{info, debug, warn, error};
use std::fmt;
use std::fs;
use std::io::{Write, Error};
use std::collections::{HashMap, HashSet};
use rdf::reader::turtle_parser::TurtleParser;
use rdf::reader::n_triples_parser::NTriplesParser;
use rdf::reader::rdf_parser::RdfParser;
use rdf::node::Node;
use rdf::namespace::Namespace;
use rdf::graph::Graph;
use rdf::triple;
use rdf::uri::Uri;
use rdf::writer::turtle_writer::TurtleWriter;
use rdf::writer::rdf_writer::RdfWriter;
// use rdf::writer::n_triples_writer::NTriplesWriter;
#[allow(dead_code)]
use crate::common::*;

pub struct ReasoningError {
    rule: String,
    message: String,
}

impl ReasoningError {
    pub fn new(rule: String, message: String) -> Self {
        ReasoningError{rule, message}
    }
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "ReasoningError<rule: {}>: {}", self.rule, self.message)
    }
}

pub struct Reasoner {
    iter1: Iteration,
    index: URIIndex,
    input: Vec<Triple>,
    errors: Vec<ReasoningError>,

    spo: Variable<Triple>,
    pso: Variable<Triple>,
    osp: Variable<Triple>,
    all_triples_input: Variable<Triple>,

    prp_dom: Variable<(URI, URI)>,
    prp_rng: Variable<(URI, URI)>,
    rdf_type: Variable<(URI, URI)>,

    prp_fp_1: Variable<(URI, ())>,
    prp_fp_2: Variable<Triple>,

    prp_ifp_1: Variable<(URI, ())>,
    prp_ifp_2: Variable<Triple>,

    prp_spo1_1: Variable<(URI, URI)>,

    cax_sco_1: Variable<(URI, URI)>,
    cax_sco_2: Variable<(URI, URI)>,

    owl_inverse_of: Variable<(URI, URI)>,
    owl_inverse_of2: Variable<(URI, URI)>,

    symmetric_properties: Variable<(URI, ())>,

    equivalent_properties: Variable<(URI, URI)>,
    equivalent_properties_2: Variable<(URI, URI)>,
}

#[allow(unused)]
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
        let prp_fp_2 = iter1.variable::<Triple>("prp_fp_2");
        // T(?p, ?x, ?y1), T(?p, ?x, ?y2) fulfilled from PSO index

        // T(?p, rdf:type, owl:InverseFunctionalProperty
        // prp-ifp
        //      T(?p, rdf:type, owl:InverseFunctionalProperty) .
        //      T(?p, ?x1, ?y) . (pso)
        //      T(?p, ?x2, ?y) . (pso) =>
        //          T(?x1, owl:sameAs, ?x2) .
        let prp_ifp_1 = iter1.variable::<(URI, ())>("prp_ifp_1");
        let prp_ifp_2 = iter1.variable::<Triple>("prp_ifp_2");
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
        let owl_inverse_of = iter1.variable::<(URI, URI)>("owl_inverseOf");
        let owl_inverse_of2 = iter1.variable::<(URI, URI)>("owl_inverse_of2");

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

        // cls-thing, cls-nothing1
        let u_owl_thing = index.put_str(OWL_THING);
        let u_owl_nothing = index.put_str(OWL_NOTHING);
        let u_rdf_type = index.put_str(RDF_TYPE);
        let u_owl_class = index.put_str(OWL_CLASS);
        let mut input = Vec::new();
        input.push((u_owl_thing, (u_rdf_type, u_owl_class)));
        input.push((u_owl_nothing, (u_rdf_type, u_owl_class)));

        Reasoner {
            iter1: iter1,
            index: index,
            input: input,
            errors: Vec::new(),
            spo: spo,
            pso: pso,
            osp: osp,
            all_triples_input: all_triples_input,

            prp_dom: prp_dom,
            prp_rng: prp_rng,
            rdf_type: rdf_type,

            prp_fp_1: prp_fp_1,
            prp_fp_2: prp_fp_2,

            prp_ifp_1: prp_ifp_1,
            prp_ifp_2: prp_ifp_2,

            prp_spo1_1: prp_spo1_1,
            cax_sco_1: cax_sco_1,
            cax_sco_2: cax_sco_2,
            owl_inverse_of: owl_inverse_of,
            owl_inverse_of2: owl_inverse_of2,
            symmetric_properties: symmetric_properties,
            equivalent_properties: equivalent_properties,
            equivalent_properties_2: equivalent_properties_2,
        }
    }

    pub fn load_triples(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) {
        let trips: Vec<(URI, (URI, URI))> = triples.iter().map(|trip| {
            (self.index.put_str(trip.0), (self.index.put_str(trip.1), self.index.put_str(trip.2)))
        }).collect();
        self.input.extend(trips);
        // self.all_triples_input.insert(trips.into());
    }

    // TODO: inspect the range of the predicate to tell if it should be a literal
    fn suggests_literal(&self, pred: &String) -> bool {
        *pred == "http://www.w3.org/2000/01/rdf-schema#label".to_string() ||
        *pred == "http://www.w3.org/2000/01/rdf-schema#comment".to_string() ||
        *pred == "http://schema.org#email".to_string() ||
        *pred == "http://schema.org#name".to_string() ||
        *pred == "http://www.w3.org/2004/02/skos/core#definition".to_string() ||
        *pred == "http://purl.org/dc/elements/1.1/title"
    }

    fn add_error(&mut self, rule: String, message: String) {
        let error = ReasoningError::new(rule, message);
        error!("Got error {}", error);
        self.errors.push(error);
    }

    fn escape_literal(&self, literal: &str) -> String {
        //if literal.contains('\\') {
        //    println!("original {}", literal);
        //}
        let escaped_literal = literal.to_string().replace("\n", "\\n");
        //let characters: Vec<char> = vec!['\'', '"', '\\'];
        //for c in characters {
        //    let mut escaped_char = "\\".to_string();
        //    escaped_char.push(c);
        //    println!("escaped: {:?}", escaped_char);
        //    escaped_literal.replace(c, &escaped_char);
        //}
        //if literal.contains("\n") {
        //    println!("=> {}", escaped_literal);
        //}
        escaped_literal
    }

    pub fn dump_file(&mut self, filename: &str) -> Result<(), Error> {
        // let mut abbrevs: HashMap<String, Uri> = HashMap::new();
        let mut graph = Graph::new(None);
        graph.add_namespace(&Namespace::new("owl".to_string(), Uri::new("http://www.w3.org/2002/07/owl#".to_string())));
        graph.add_namespace(&Namespace::new("rdf".to_string(), Uri::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string())));
        graph.add_namespace(&Namespace::new("rdfs".to_string(), Uri::new("http://www.w3.org/2000/01/rdf-schema#".to_string())));
        graph.add_namespace(&Namespace::new("brick".to_string(), Uri::new("https://brickschema.org/schema/1.1.0/Brick#".to_string())));
        graph.add_namespace(&Namespace::new("tag".to_string(), Uri::new("https://brickschema.org/schema/1.1.0/BrickTag#".to_string())));
        for i in self.get_triples() {
            let (s, p, o) = i;

            // skip these because they put Literals as the subject of a triple and some parsers
            // complain about this
            if p == RDF_TYPE && o == RDFS_LITERAL {
                continue
            }
            if p == RDF_TYPE && o == RDFS_RESOURCE {
                continue
            }
            if p ==  "http://www.w3.org/2000/01/rdf-schema#seeAlso" {
                continue
            }
            if p ==  "http://www.w3.org/2000/01/rdf-schema#isDefinedBy" {
                continue
            }
            let subject = graph.create_uri_node(&Uri::new(s));
            //let subject = if p == RDF_TYPE && o == RDFS_LITERAL {
            //    graph.create_literal_node(s)
            //} else {
            //    graph.create_uri_node(&Uri::new(s))
            //};

            // determine if the object should be encoded as a literal. Checking for a ' ' is a poor
            // heuristic; TODO: correct answer is to use the datatype of the predicate's rdfs:range
            // property
            //let object = if self.suggests_literal(&p) {
            // TODO: remove newlines from the literal? maybe this is put in here by the serializer
            let object = if o.contains(" ") || self.suggests_literal(&p) {
                // graph.create_literal_node(o.escape_default().to_string())
                graph.create_literal_node(self.escape_literal(&o))
            } else {
                graph.create_uri_node(&Uri::new(o))
            };

            let predicate = graph.create_uri_node(&Uri::new(p));
            let t = triple::Triple::new(&subject, &predicate, &object);
            graph.add_triple(&t);
        }

        let mut output = fs::File::create(filename)?;
        let writer = TurtleWriter::new(graph.namespaces());
        // let writer = NTriplesWriter::new();
        let serialized = writer.write_to_string(&graph).unwrap();
        output.write_all(serialized.as_bytes())?;
        info!("Wrote {} triples to {}", graph.count(), filename);
        Ok(())
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
        info!("Loaded {} triples from file {}", graph.count(), filename);
        let triples : Vec<(URI, (URI, URI))> = graph.triples_iter().map(|_triple| {
            let triple = _triple;
            let subject = match triple.subject() {
                Node::UriNode{uri} => uri.to_string(),
                Node::LiteralNode{literal, data_type: _, language: _} => &literal,
                Node::BlankNode{id} => &id,
            };

            let predicate = match triple.predicate() {
                Node::UriNode{uri} => uri.to_string(),
                Node::LiteralNode{literal, data_type: _, language: _} => &literal,
                Node::BlankNode{id} => &id,
            };

            let object = match triple.object() {
                Node::UriNode{uri} => uri.to_string(),
                Node::LiteralNode{literal, data_type: _, language: _} => &literal,
                Node::BlankNode{id} => &id,
            };
            debug!("{} {} {}", subject, predicate, object);

            let (s, (p, o)) = (self.index.put(subject.to_string()), (self.index.put(predicate.to_string()), self.index.put(object.to_string())));


            (s, (p,o))

        }).collect();

        //self.all_triples_input.insert(triples.into());
        self.input.extend(triples);

        Ok(())
    }

    pub fn reason(&mut self) {
        // TODO: put these URIs inside the index initialization and give easy ways of referring to
        // them
        let rdftype_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#type");
        let rdfsdomain_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#domain");
        let rdfsrange_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#range");
        let owlthing_node = self.index.put_str("http://www.w3.org/2002/07/owl#Thing");
        let owlsameas_node = self.index.put_str("http://www.w3.org/2002/07/owl#sameAs");
        let owlinverseof_node = self.index.put_str("http://www.w3.org/2002/07/owl#inverseOf");
        let owlsymmetricprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#SymmetricProperty");
        let owltransitiveprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#TransitiveProperty");
        let owlequivprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#equivalentProperty");
        let owlequivclassprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#equivalentClass");
        let owlfuncprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#FunctionalProperty");
        let owlinvfuncprop_node = self.index.put_str("http://www.w3.org/2002/07/owl#InverseFunctionalProperty");
        let rdfssubprop_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#subPropertyOf");
        let rdfssubclass_node = self.index.put_str("http://www.w3.org/2000/01/rdf-schema#subClassOf");
        let owlintersection_node = self.index.put_str("http://www.w3.org/2002/07/owl#intersectionOf");
        let owlunion_node = self.index.put_str("http://www.w3.org/2002/07/owl#unionOf");
        let owlhasvalue_node = self.index.put_str("http://www.w3.org/2002/07/owl#hasValue");
        let owlallvaluesfrom_node = self.index.put_str("http://www.w3.org/2002/07/owl#allValuesFrom");
        let owlsomevaluesfrom_node = self.index.put_str("http://www.w3.org/2002/07/owl#someValuesFrom");
        let owldisjointwith_node = self.index.put_str("http://www.w3.org/2002/07/owl#disjointWith");
        let owlonproperty_node = self.index.put_str("http://www.w3.org/2002/07/owl#onProperty");
        let owlcomplementof_node = self.index.put_str("http://www.w3.org/2002/07/owl#complementOf");

        let rdffirst_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#first");
        let rdfrest_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#rest");
        let rdfnil_node = self.index.put_str("http://www.w3.org/1999/02/22-rdf-syntax-ns#nil");
        let rdf_type_inv = self.iter1.variable::<(URI, URI)>("rdf_type_inv");

        let prp_fp_isfuncprop = self.iter1.variable::<Triple>("a");
        let prp_fp_hasprop1 = self.iter1.variable::<Triple>("b");

        let owl_intersection_of = self.iter1.variable::<(URI, URI)>("owl_intersection_of");
        let owl_union_of = self.iter1.variable::<(URI, URI)>("owl_union_of");
        let mut intersections: HashMap<URI, URI> = HashMap::new();
        let mut unions: HashMap<URI, URI> = HashMap::new();
        let mut instances: HashSet<(URI, URI)> = HashSet::new();
        let mut complements: HashMap<URI, URI> = HashMap::new();

        // eq-ref
        //  T(?s, ?p, ?o) =>
        //  T(?s, owl:sameAs, ?s)
        //  T(?p, owl:sameAs, ?p)
        //  T(?o, owl:sameAs, ?o)

        // eq-sym
        //  T(?x, owl:sameAs, ?y)  =>  T(?y, owl:sameAs, ?x)
        let owl_same_as = self.iter1.variable::<(URI, URI)>("owl_same_as");

        // eq-trans
        // T(?x, owl:sameAs, ?y)
        // T(?y, owl:sameAs, ?z)  =>  T(?x, owl:sameAs, ?z)
        let eq_trans_1 = self.iter1.variable::<(URI, URI)>("eq_trans_1");

        // eq-rep-s, eq-rep-o, eq-rep-p
        // T(?s, owl:sameAs, ?s')
        // TODO: make more efficient
        // T(?s, ?p, ?o) => T(?s', ?p, ?o) (and then for p' and o')

        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        //
        // (p1, p2)
        let prp_inv1 = self.iter1.variable::<(URI, URI)>("prp_inv1");

        // prp-trp
        // T(?p, rdf:type, owl:TransitiveProperty)
        // T(?x, ?p, ?y)
        // T(?y, ?p, ?z) =>  T(?x, ?p, ?z)
        let transitive_properties = self.iter1.variable::<(URI, ())>("transitive_properties");
        let prp_trp_1 = self.iter1.variable::<((URI, URI), URI)>("prp_trp_1");
        let prp_trp_2 = self.iter1.variable::<((URI, URI), URI)>("prp_trp_2");

        let list_test1 = self.iter1.variable::<(URI, URI)>("list_test1");

        // cls-int1
        //    T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
        //    T(?y rdf:type ?c_i) for i in range(1,n) =>
        //     T(?y rdf:type ?c)

        // cls-int2
        //    T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
        //     T(?y rdf:type ?c) =>
        //    T(?y rdf:type ?c_i) for i in range(1,n)
        let cls_int_2_1 = self.iter1.variable::<(URI, URI)>("cls_int_2_1");

        // cls-uni
        // cls-uni  T(?c, owl:unionOf, ?x)
        // LIST[?x, ?c1, ..., ?cn]
        // T(?y, rdf:type, ?ci) (for any i in 1-n) =>  T(?y, rdf:type, ?c)

        // cls-hv1:
        // T(?x, owl:hasValue, ?y)
        // T(?x, owl:onProperty, ?p)
        // T(?u, rdf:type, ?x) =>  T(?u, ?p, ?y)
        let owl_has_value = self.iter1.variable::<(URI, URI)>("owl_has_value");
        let owl_on_property = self.iter1.variable::<(URI, URI)>("owl_on_property");
        let cls_hv1_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_hv1_1");

        // cls-hv2:
        // T(?x, owl:hasValue, ?y)
        // T(?x, owl:onProperty, ?p)
        // T(?u, ?p, ?y) =>  T(?u, rdf:type, ?x)
        let cls_hv2_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_hv2_1");

        // cls-avf:
        // T(?x, owl:allValuesFrom, ?y)
        // T(?x, owl:onProperty, ?p)
        // T(?u, rdf:type, ?x)
        // T(?u, ?p, ?v) =>  T(?v, rdf:type, ?y)
        let owl_all_values_from = self.iter1.variable::<(URI, URI)>("owl_all_values_from");
        let cls_avf_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_avf_1");
        let cls_avf_2 = self.iter1.variable::<(URI, (URI, URI))>("cls_avf_2");

        // cls-svf1
        // T(?x, owl:someValuesFrom, ?y)
        // T(?x, owl:onProperty, ?p)
        // T(?u, ?p, ?v)
        // T(?v, rdf:type, ?y) =>  T(?u, rdf:type, ?x)
        let owl_some_values_from = self.iter1.variable::<(URI, URI)>("owl_some_values_from");
        let cls_svf1_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_svf1_1");
        let cls_svf1_2 = self.iter1.variable::<(URI, (URI, URI, URI))>("cls_svf1_2");

        // cls-com
        let owl_complement_of = self.iter1.variable::<(URI, URI)>("owl_complement_of");
        let things = self.iter1.variable::<(URI, ())>("things");
        let cls_com_1 = self.iter1.variable::<(URI, URI)>("cls_com_1");

        // cax-eqc1
        // T(?c1, owl:equivalentClass, ?c2), T(?x, rdf:type, ?c1)  =>
        //  T(?x, rdf:type, ?c2)
        // cax-eqc2
        // T(?c1, owl:equivalentClass, ?c2), T(?x, rdf:type, ?c2)  =>
        //  T(?x, rdf:type, ?c1)
        let owl_equivalent_class = self.iter1.variable::<(URI, URI)>("owl_equivalent_class");

        // cax-dw
        // T(?c1, owl:disjointWith, ?c2)
        // T(?x, rdf:type, ?c1)
        // T(?x, rdf:type, ?c2) => false
        let owl_disjoint_with = self.iter1.variable::<(URI, URI)>("owl_disjoint_with");
        let cax_dw_1 = self.iter1.variable::<(URI, (URI, URI))>("cax_dw_1");
        let cax_dw_2 = self.iter1.variable::<(URI, URI)>("cax_dw_2");


        let ds = DisjointSets::new(&self.input);

        self.all_triples_input.extend(self.input.iter().cloned());
        while self.iter1.changed() {
            // TODO: pre-filter to eliminate triples that could cause issues
            // - check complements hashmap
            // - check disjoint relations
            // How: create a relation (monotonic) of all of the unallowed triples, antijoin this
            // against the triples input

            self.spo.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (sub, (pred, obj)));
            self.pso.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (pred, (sub, obj)));
            self.osp.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (obj, (sub, pred)));

            self.rdf_type.from_map(&self.spo, |&triple| {
                let tup = has_pred(triple, rdftype_node);
                instances.insert(tup);
                tup
            });
            things.from_map(&self.spo, |&triple| {
                let (inst, ()) = has_pred_obj(triple, (rdftype_node, owlthing_node));
                instances.insert((inst, owlthing_node));
                (inst, ())
            });
            rdf_type_inv.from_map(&self.rdf_type, |&(inst, class)| { (class, inst) });
            self.prp_dom.from_map(&self.spo, |&triple| { has_pred(triple, rdfsdomain_node) });
            self.prp_rng.from_map(&self.spo, |&triple| { has_pred(triple, rdfsrange_node) });

            self.owl_inverse_of.from_map(&self.spo, |&triple| has_pred(triple, owlinverseof_node) );
            self.owl_inverse_of2.from_map(&self.owl_inverse_of, |&(p1, p2)| (p2, p1) );

            owl_intersection_of.from_map(&self.spo, |&triple| {
                let (a, b) = has_pred(triple, owlintersection_node);
                if a > 0 && b > 0 {
                    intersections.insert(a, b);
                }
                (a, b)
            });
            owl_union_of.from_map(&self.spo, |&triple| {
                let (a, b) = has_pred(triple, owlunion_node);
                if a > 0 && b > 0 {
                    unions.insert(a, b);
                }
                (a, b)
            });
            owl_has_value.from_map(&self.spo, |&triple| has_pred(triple, owlhasvalue_node));
            owl_on_property.from_map(&self.spo, |&triple| has_pred(triple, owlonproperty_node));
            owl_all_values_from.from_map(&self.spo, |&triple| has_pred(triple, owlallvaluesfrom_node));
            owl_some_values_from.from_map(&self.spo, |&triple| has_pred(triple, owlsomevaluesfrom_node));
            owl_disjoint_with.from_map(&self.spo, |&triple| has_pred(triple, owldisjointwith_node));
            owl_same_as.from_map(&self.spo, |&triple| has_pred(triple, owlsameas_node));
            owl_complement_of.from_map(&self.spo, |&triple| {
                let (a, b) = has_pred(triple, owlcomplementof_node);
                if a >0 && b > 0 {
                    complements.insert(a, b);
                    complements.insert(b, a);
                }
                (a, b)
            });

            owl_equivalent_class.from_map(&self.spo, |&triple| {
                let (c1, c2) = has_pred(triple, owlequivclassprop_node);
                (c1, c2)
            });
            owl_equivalent_class.from_map(&self.spo, |&triple| {
                let (c1, c2) = has_pred(triple, owlequivclassprop_node);
                (c2, c1)
            });

            self.symmetric_properties.from_map(&self.spo, |&triple| {
                has_pred_obj(triple, (rdftype_node, owlsymmetricprop_node))
            });

            transitive_properties.from_map(&self.spo, |&triple| has_pred_obj(triple, (rdftype_node, owltransitiveprop_node)));

            self.equivalent_properties.from_map(&self.spo, |&triple| has_pred(triple, owlequivprop_node) );
            self.equivalent_properties_2.from_map(&self.equivalent_properties, |&(p1, p2)| (p2, p1));

            self.all_triples_input.from_join(&self.prp_dom, &self.pso, |&tpred, &class, &(sub, obj)| {
                (sub, (rdftype_node, class))
            });
            self.all_triples_input.from_join(&self.prp_rng, &self.pso, |&tpred, &class, &(sub, obj)| {
                (obj, (rdftype_node, class))
            });


            // eq-ref
            //  T(?s, ?p, ?o) =>
            //  T(?s, owl:sameAs, ?s)
            //  T(?p, owl:sameAs, ?p)
            //  T(?o, owl:sameAs, ?o)
            //self.all_triples_input.from_map(&self.spo, |&(s, (p, o))| {
            //    (s, (owlsameas_node, s))
            //});
            //self.all_triples_input.from_map(&self.spo, |&(s, (p, o))| {
            //    (p, (owlsameas_node, p))
            //});
            //self.all_triples_input.from_map(&self.spo, |&(s, (p, o))| {
            //    (o, (owlsameas_node, o))
            //});

            // eq-sym
            //  T(?x, owl:sameAs, ?y)  =>  T(?y, owl:sameAs, ?x)
            self.all_triples_input.from_join(&self.spo, &owl_same_as, |&x, &(p, o), &y| {
                (y, (owlsameas_node, x))
            });

            // eq-rep-s
            self.all_triples_input.from_join(&self.spo, &owl_same_as, |&s1, &(p, o), &s2| {
                (s2, (p, o))
            });
            // eq-rep-p
            self.all_triples_input.from_join(&self.pso, &owl_same_as, |&p1, &(s, o), &p2| {
                (s, (p2, o))
            });
            // eq-rep-o
            self.all_triples_input.from_join(&self.osp, &owl_same_as, |&o1, &(s, p), &o2| {
                (s, (p, o2))
            });

            // eq-trans
            // T(?x, owl:sameAs, ?y)
            // T(?y, owl:sameAs, ?z)  =>  T(?x, owl:sameAs, ?z)
            eq_trans_1.from_join(&owl_same_as, &self.spo, |&x, &y, &(p, o)| {
                (o, x)
            });
            self.all_triples_input.from_join(&owl_same_as, &eq_trans_1, |&y, &z, &x| {
                (x, (owlsameas_node, z))
            });


            // prp-fp
            self.prp_fp_1.from_map(&self.spo, |&triple| { has_pred_obj(triple, (rdftype_node, owlfuncprop_node)) });
            self.prp_fp_2.from_join(&self.prp_fp_1, &self.pso, |&p, &(), &(x, y1)| (p, (x, y1)) );
            self.all_triples_input.from_join(&self.prp_fp_2, &self.pso, |&p, &(x1, y1), &(x2, y2)| {
                // if x1 and x2 are the same, then emit y1 and y2 are the same
                match x1 {
                    x2 => (y1, (owlsameas_node, y2)),
                    _ => (0, (0,0)),
                }
            });

            // prp-ifp
            self.prp_ifp_1.from_map(&self.spo, |&triple| { has_pred_obj(triple, (rdftype_node, owlinvfuncprop_node)) });
            self.prp_ifp_2.from_join(&self.prp_ifp_1, &self.pso, |&p, &(), &(x, y1)| (p, (x, y1)) );
            self.all_triples_input.from_join(&self.prp_ifp_2, &self.pso, |&p, &(x1, y1), &(x2, y2)| {
                // if y1 and y2 are the same, then emit x1 and x2 are the same
                match y1 {
                    y2 => (x1, (owlsameas_node, x2)),
                    _ => (0, (0,0)),
                }
            });

            // prp-spo1
            self.prp_spo1_1.from_map(&self.spo, |&triple| has_pred(triple, rdfssubprop_node));
            self.all_triples_input.from_join(&self.prp_spo1_1, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)));

            // cax-sco
            self.cax_sco_1.from_map(&self.spo, |&triple| has_pred(triple, rdfssubclass_node));
            // ?c1, ?x, rdf:type
            self.cax_sco_2.from_map(&self.rdf_type, |&(inst, class)| (class, inst));
            self.all_triples_input.from_join(&self.cax_sco_1, &self.cax_sco_2, |&class, &parent, &inst| (inst, (rdftype_node, parent)));

            // cax-eqc1, cax-eqc2
            // find instances of classes that are equivalent
            self.all_triples_input.from_join(&owl_equivalent_class, &rdf_type_inv, |&c1, &c2, &inst| {
                (inst, (rdftype_node, c2))
            });

            // cax-dw
            cax_dw_1.from_join(&owl_disjoint_with, &rdf_type_inv, |&c1, &c2, &inst| {
                (c2, (c1, inst))
            });
            cax_dw_2.from_join(&cax_dw_1, &rdf_type_inv, |&c2, &(c1, inst1), &inst2| {
                // TODO: how to raise 'false'?
                if inst1 == inst2 {
                    let msg = format!("inst {} is both {} and {} (disjoint classes)", self.to_u(inst1), self.to_u(c1), self.to_u(c2));
                    self.add_error("cax-dw".to_string(), msg.to_string());
                }
                (c2, inst1)
            });

            // prp-inv1
            // T(?p1, owl:inverseOf, ?p2)
            // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
            self.all_triples_input.from_join(&self.owl_inverse_of, &self.pso, |&p1, &p2, &(x, y)| (y, (p2, x)) );

            // prp-inv2
            // T(?p1, owl:inverseOf, ?p2)
            // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
            self.all_triples_input.from_join(&self.owl_inverse_of2, &self.pso, |&p2, &p1, &(x, y)| (x, (p2, y)) );

            // prp-symp
            // T(?p, rdf:type, owl:SymmetricProperty)
            // T(?x, ?p, ?y)
            //  => T(?y, ?p, ?x)
            self.all_triples_input.from_join(&self.symmetric_properties, &self.pso, |&prop, &(), &(x, y)| {
                (y, (prop, x))
            });

            // prp-trp
            // T(?p, rdf:type, owl:TransitiveProperty)
            // T(?x, ?p, ?y)
            // T(?y, ?p, ?z) =>  T(?x, ?p, ?z)
            prp_trp_1.from_join(&self.pso, &transitive_properties, |&p, &(x, y), &()| {
                ((y, p), x)
            });
            prp_trp_2.from_join(&self.pso, &transitive_properties, |&p, &(y, z), &()| {
                ((y, p), z)
            });
            self.all_triples_input.from_join(&prp_trp_1, &prp_trp_2, |&(y, p), &x, &z| {
                (x, (p, z))
            });

            // prp-eqp1
            // T(?p1, owl:equivalentProperty, ?p2)
            // T(?x, ?p1, ?y)
            // => T(?x, ?p2, ?y)
            self.all_triples_input.from_join(&self.equivalent_properties, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );
            // prp-eqp2
            // T(?p1, owl:equivalentProperty, ?p2)
            // T(?x, ?p2, ?y)
            // => T(?x, ?p1, ?y)
            self.all_triples_input.from_join(&self.equivalent_properties_2, &self.pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );

            // cls-int1
            // There's a fair amount of complexity here that we have to manage. The rule we are
            // implementing is cls-int-1:
            //
            //      T(?c owl:intersectionOf ?x), LIST[?x, ?c1...?cn],
            //      T(?y rdf:type ?c_i) for i in range(1,n) =>
            //       T(?y rdf:type ?c)
            //
            // Useful structures:
            // - `owl_intersection_of` is keyed by class and values are the list heads
            // - `ds` gives the list values for the given head (ds.get_list_values(listname))
            //
            // Goal: we need to find instances (?y rdf:type ?class) of all classes given by the
            // list identified by the head for each owl:intersectionOf node.
            //
            // We can get the list of classes easily by iterating over each key of the
            // owl_intersection_of variable. However, we need an efficient way of seeing if there
            // are instances of *each* of those classes (union). This could be a N-way join (where
            // N is the number of classes in the list)
            let mut new_cls_int1_instances = Vec::new();
            for (_intersection_class, _listname) in intersections.iter() {
                let listname = *_listname;
                let intersection_class = *_intersection_class;
                if let Some(values) = ds.get_list_values(listname) {
                    let value_uris: Vec<&String> = values.iter().map(|v| self.index.get(*v).unwrap()).collect();
                    debug!("{} {} (len {}) {} {:?}", listname, self.index.get(intersection_class).unwrap(), values.len(), self.index.get(listname).unwrap(), value_uris);
                    let mut class_counter: HashMap<URI, usize> = HashMap::new();
                    for (_inst, _list_class) in instances.iter() {
                        let inst = *_inst;
                        let list_class = *_list_class;
                        debug!("inst {} values len {}, list class {}", self.index.get(inst).unwrap(), values.len(), list_class);
                        if values.contains(&list_class) {
                            debug!("{} is a {}", inst, list_class);
                            let count = class_counter.entry(inst).or_insert(0);
                            *count += 1;
                        }
                    }
                    for (inst, num_implemented) in class_counter.iter() {
                        if *num_implemented == values.len() {
                            debug!("inferred that {} is a {}", inst, intersection_class);
                            new_cls_int1_instances.push((*inst, (rdftype_node, intersection_class)));
                        }
                    }
                }
            }
            self.all_triples_input.extend(new_cls_int1_instances);

            // cls-int2
            let mut new_cls_int2_instances = Vec::new();
            cls_int_2_1.from_join(&owl_intersection_of, &rdf_type_inv, |&intersection_class, &listname, &inst| {
                if let Some(values) = ds.get_list_values(listname) {
                    for list_class in values {
                        new_cls_int2_instances.push((inst, (rdftype_node, list_class)));
                    }
                }
                (inst, new_cls_int2_instances.len() as URI)
            });
            self.all_triples_input.extend(new_cls_int2_instances);

            // cls-uni  T(?c, owl:unionOf, ?x)
            // LIST[?x, ?c1, ..., ?cn]
            // T(?y, rdf:type, ?ci) (for any i in 1-n) =>  T(?y, rdf:type, ?c)
            let mut new_cls_uni_instances = Vec::new();
            for (_union_class, _listname) in unions.iter() {
                let listname = *_listname;
                let union_class = *_union_class;
                if let Some(values) = ds.get_list_values(listname) {
                    let value_uris: Vec<&String> = values.iter().map(|v| self.index.get(*v).unwrap()).collect();
                    debug!("{} {} (len {}) {} {:?}", listname, self.index.get(union_class).unwrap(), values.len(), self.index.get(listname).unwrap(), value_uris);
                    let mut class_counter: HashMap<URI, usize> = HashMap::new();
                    for (_inst, _list_class) in instances.iter() {
                        let inst = *_inst;
                        let list_class = *_list_class;
                        debug!("inst {} values len {}, list class {}", self.index.get(inst).unwrap(), values.len(), list_class);
                        if values.contains(&list_class) {
                            debug!("{} is a {}", inst, list_class);
                            let count = class_counter.entry(inst).or_insert(0);
                            *count += 1;
                        }
                    }
                    for (inst, num_implemented) in class_counter.iter() {
                        if *num_implemented > 0 { // union instead of union
                            debug!("inferred that {} is a {}", inst, union_class);
                            new_cls_uni_instances.push((*inst, (rdftype_node, union_class)));
                        }
                    }
                }
            }
            self.all_triples_input.extend(new_cls_uni_instances);

            // cls-com
            // T(?c1, owl:complementOf, ?c2)
            // T(?x, rdf:type, ?c1)
            // T(?x, rdf:type, ?c2)  => false
            // TODO: how do we infer instances of classes from owl:complementOf?
            // find instances of owl:Thing (in 'things') that are not instances of c1
            //
            // for each complementary class c1, find all instnaces that AREN't instances of it and
            // make them instances of the complement c2

            // let mut new_complementary_instances: Vec<Triple> = Vec::new();
            // for (c1, c2) in complements.iter() {
            //     let c1u = self.to_u(*c1);
            //     let c2u = self.to_u(*c2);
            //     let mut not_c1: HashSet<URI> = HashSet::new();
            //     let mut not_c2: HashSet<URI> = HashSet::new();
            //     for (inst, class) in instances.iter() {
            //         if !instances.contains(&(*inst, *c1)) {
            //             not_c1.insert(*inst);
            //             not_c2.remove(inst);
            //         }

            //         if !instances.contains(&(*inst, *c2)) {
            //             not_c2.insert(*inst);
            //             not_c1.remove(inst);
            //         }
            //     }
            //     for inst in not_c1.iter() {
            //         let instu = self.to_u(*inst);
            //         new_complementary_instances.push((*inst, (rdftype_node, *c2)));
            //     }
            //     for inst in not_c2.iter() {
            //         let instu = self.to_u(*inst);
            //         new_complementary_instances.push((*inst, (rdftype_node, *c1)));
            //     }
            // }
            // println!("new complementary instances # {}", new_complementary_instances.len());
            // //self.all_triples_input.extend(new_complementary_instances);

            // cls-hv1:
            // T(?x, owl:hasValue, ?y)
            // T(?x, owl:onProperty, ?p)
            // T(?u, rdf:type, ?x) =>  T(?u, ?p, ?y)
            cls_hv1_1.from_join(&owl_has_value, &owl_on_property, |&x, &y, &p| {
                (x, (p, y))
            });
            self.all_triples_input.from_join(&cls_hv1_1, &rdf_type_inv, |&x, &(prop, value), &inst| {
                (inst, (prop, value))
            });

            // cls-hv2:
            // T(?x, owl:hasValue, ?y)
            // T(?x, owl:onProperty, ?p)
            // T(?u, ?p, ?y) =>  T(?u, rdf:type, ?x)
            cls_hv2_1.from_join(&owl_has_value, &owl_on_property, |&x, &y, &p| {
                // format for pso index; needs property key
                (p, (y, x))
            });
            self.all_triples_input.from_join(&cls_hv2_1, &self.pso, |&prop, &(value, anonclass), &(sub, obj)| {
                // if value is correct, then emit the rdf_type
                if value == obj {
                    (sub, (rdftype_node, anonclass))
                } else {
                    (0, (0, 0))
                }
            });

            // cls-avf:
            // T(?x, owl:allValuesFrom, ?y)
            // T(?x, owl:onProperty, ?p)
            // T(?u, rdf:type, ?x)
            // T(?u, ?p, ?v) =>  T(?v, rdf:type, ?y)
            cls_avf_1.from_join(&owl_all_values_from, &owl_on_property, |&x, &y, &p| {
                (x, (y, p))
            });
            cls_avf_2.from_join(&cls_avf_1, &rdf_type_inv, |&x, &(y, p), &u| {
                (u, (p, y))
            });
            self.all_triples_input.from_join(&cls_avf_2, &self.spo, |&u, &(p1, y), &(p2, v)| {
                if p1 == p2 {
                    (v, (rdftype_node, y))
                } else {
                    (0, (0, 0))
                }
            });

            // cls-svf1:
            // T(?x, owl:someValuesFrom, ?y)
            // T(?x, owl:onProperty, ?p)
            // T(?u, ?p, ?v)
            // T(?v, rdf:type, ?y) =>  T(?u, rdf:type, ?x)
            cls_svf1_1.from_join(&owl_some_values_from, &owl_on_property, |&x, &y, &p| {
                (p, (x, y))
            });
            cls_svf1_2.from_join(&cls_svf1_1, &self.pso, |&p, &(x, y), &(u, v)| {
                (v, (x, y, u))
            });
            self.all_triples_input.from_join(&cls_svf1_2, &self.rdf_type, |&v, &(x, y, u), &class| {
                if class == y {
                    (u, (rdftype_node, x))
                } else {
                    (0, (0, 0))
                }
            });

            // cls-svf2:
            //  T(?x, owl:someValuesFrom, owl:Thing)
            //  T(?x, owl:onProperty, ?p)
            //  T(?u, ?p, ?v) =>  T(?u, rdf:type, ?x)
            self.all_triples_input.from_join(&cls_svf1_1, &self.pso, |&p, &(x, y), &(u, v)| {
                if y == owlthing_node {
                    (u, (rdftype_node, x))
                } else {
                    (0, (0, 0))
                }
            });
        }
    }

    fn to_u(&self, u: URI) -> &str {
        self.index.get(u).unwrap()
    }


    pub fn get_triples(&mut self) -> Vec<(String, String, String)> {
        let instances = self.spo.clone().complete();

        instances.iter().filter(|inst| {
            let (_s, (_p, _o)) = inst;
             *_s > 0 && *_p > 0 && *_o > 0
        }).map(|inst| {
            let (_s, (_p, _o)) = inst;
            let s = self.index.get(*_s).unwrap();
            let p = self.index.get(*_p).unwrap();
            let o = self.index.get(*_o).unwrap();
            (s.clone(), p.clone(), o.clone())
        }).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_reasoner() -> Result<(), String> {
        let _r = Reasoner::new();
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

    #[test]
    #[ignore]
    fn test_eq_ref() -> Result<(), String> {
        let mut r = Reasoner::new();
        let trips = vec![
            ("a", "b", "c")
        ];
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
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
        r.load_triples(trips);
        r.reason();
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        assert!(res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), "x".to_string())));
        assert!(!res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), "c".to_string())));
        assert!(!res.contains(&("inst1".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
        assert!(res.contains(&("inst2".to_string(), RDF_TYPE.to_string(), "c2".to_string())));
        Ok(())
    }
}
