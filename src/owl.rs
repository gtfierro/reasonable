//! The `owl` module implements the rules necessary for OWL 2 RL reasoning

extern crate datafrog;
use datafrog::{Iteration, Variable, Relation};

use crate::index::URIIndex;
use crate::disjoint_sets::DisjointSets;

use log::{info, debug, error};
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

/// Returns the full URI of the concept in the OWL namespace
/// ```
/// let uri = owl!("Thing");
/// println!(uri);
/// ```
macro_rules! owl {
    ($t:expr) => (format!("http://www.w3.org/2002/07/owl#{}", $t));
}

/// Returns the full URI of the concept in the RDF namespace
/// ```
/// let uri = rdf!("type");
/// println!(uri);
/// ```
macro_rules! rdf {
    ($t:expr) => (format!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{}", $t));
}

/// Returns the full URI of the concept in the RDFS namespace
/// ```
/// let uri = rdfs!("type");
/// println!(uri);
/// ```
macro_rules! rdfs {
    ($t:expr) => (format!("http://www.w3.org/2000/01/rdf-schema#{}", $t));
}

/// Creates a DataFrog variable with the given URI as the only member
macro_rules! node_relation {
    ($self:expr, $uri:expr) => {
        {
            let x = $self.iter1.variable::<(URI, ())>("tmp");
            let v = vec![($self.index.put($uri), ())];
            x.extend(v.iter());
            x
        }
    };
}

/// Structured errors that occur during reasoning
pub struct ReasoningError {
    /// The OWL-RL rule that produced the violation
    rule: String,
    /// A human-readable error message
    message: String,
    // TODO: add a trace of the productions that caused the error
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

/// `Reasoner` is the interface to the reasoning engine. Instances of `Reasoner` maintain the state
/// required to do reasoning.
///
/// ```
/// use reasonable::owl::Reasoner;
/// let mut r = Reasoner::new();
/// // load in an ontology file
/// r.load_file("example_models/ontologies/Brick.n3").unwrap();
/// // load in another ontology file
/// r.load_file("example_models/ontologies/rdfs.ttl").unwrap();
/// // load in more triples
/// r.load_file("example_models/small1.n3").unwrap();
/// // perform reasoning
/// r.reason();
/// // dump to file
/// r.dump_file("output.ttl").unwrap();
/// ```
pub struct Reasoner {
    iter1: Iteration,
    index: URIIndex,
    input: Vec<Triple>,
    errors: Vec<ReasoningError>,

    spo: Variable<Triple>,
    all_triples_input: Variable<Triple>,
}

#[allow(unused)]
impl Reasoner {
    /// Create a new Reasoner instance
    pub fn new() -> Self {
        let mut iter1 = Iteration::new();
        let mut index = URIIndex::new();

        // variables within the iteration
        let spo = iter1.variable::<(URI, (URI, URI))>("spo");
        let all_triples_input = iter1.variable::<(URI, (URI, URI))>("all_triples_input");

        // cls-thing, cls-nothing1
        let u_owl_thing = index.put(owl!("Thing"));
        let u_owl_nothing = index.put(owl!("Nothing"));
        let u_rdf_type = index.put(rdf!("type"));
        let u_owl_class = index.put(owl!("Class"));
        let mut input = Vec::new();
        input.push((u_owl_thing, (u_rdf_type, u_owl_class)));
        input.push((u_owl_nothing, (u_rdf_type, u_owl_class)));

        Reasoner {
            iter1: iter1,
            index: index,
            input: input,
            errors: Vec::new(),
            spo: spo,
            all_triples_input: all_triples_input,
        }
    }

    fn rebuild_with(&mut self, input: Relation<Triple>) {
        // TODO: pull in the existing triples
        self.iter1 = Iteration::new();
        self.input = input.clone().iter().map(|&(x, (y, z))| (x, (y, z))).collect();
        self.all_triples_input = self.iter1.variable::<(URI, (URI, URI))>("all_triples_input");
        self.spo = self.iter1.variable::<(URI, (URI, URI))>("spo");
    }

    /// Load in a vector of triples
    #[allow(dead_code)]
    pub fn load_triples_str(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) {
        let trips: Vec<(URI, (URI, URI))> = triples.iter().map(|trip| {
            (self.index.put_str(trip.0), (self.index.put_str(trip.1), self.index.put_str(trip.2)))
        }).collect();
        self.input.extend(trips);
        // self.all_triples_input.insert(trips.into());
    }

    /// Load in a vector of triples
    #[allow(dead_code)]
    pub fn load_triples(&mut self, triples: Vec<(String, String, String)>) {
        let trips: Vec<(URI, (URI, URI))> = triples.iter().map(|trip| {
            let trip = trip.clone();
            (self.index.put(trip.0), (self.index.put(trip.1), self.index.put(trip.2)))
        }).collect();
        self.input.extend(trips);
        // self.all_triples_input.insert(trips.into());
    }

    fn add_error(&mut self, rule: String, message: String) {
        let error = ReasoningError::new(rule, message);
        error!("Got error {}", error);
        self.errors.push(error);
    }

    /// Dump the contents of the reasoner to the given file.
    /// NOTE: currently `dump_file` prevents the reasoner from being re-used. This is a bug and
    /// will be amended
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

            let subject = add_node_to_graph(&graph, s, false);
            let predicate = add_node_to_graph(&graph, p, false);
            let object = add_node_to_graph(&graph, o, true);

            info!("OUTPUT: {:?} {:?} {:?}", subject, predicate, object);
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


    /// Load the triples in the given file into the Reasoner. This currently accepts
    /// Turtle-formatted (`.ttl`) and NTriples-formatted (`.n3`) files. If you have issues loading
    /// in a Turtle file, try converting it to NTriples
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
            let subject = node_to_string(triple.subject());
            let predicate = node_to_string(triple.predicate());
            let object = node_to_string(triple.object());

            let (s, (p, o)) = (self.index.put(subject.to_string()), (self.index.put(predicate.to_string()), self.index.put(object.to_string())));


            (s, (p,o))

        }).collect();

        //self.all_triples_input.insert(triples.into());
        self.input.extend(triples);

        Ok(())
    }

    /// Perform OWL 2 RL-compatible reasoning on the triples currently loaded into the `Reasoner`
    pub fn reason(&mut self) {
        // TODO: put these URIs inside the index initialization and give easy ways of referring to
        // them

        // RDF nodes
        let rdftype_node = self.index.put(rdf!("type"));
        let rdffirst_node = self.index.put(rdf!("first"));
        let rdfrest_node = self.index.put(rdf!("rest"));
        let rdfnil_node = self.index.put(rdf!("nil"));

        // RDFS nodes
        let rdfsdomain_node = self.index.put(rdfs!("domain"));
        let rdfsrange_node = self.index.put(rdfs!("range"));
        let rdfssubprop_node = self.index.put(rdfs!("subPropertyOf"));
        let rdfssubclass_node = self.index.put(rdfs!("subClassOf"));

        // OWL nodes
        let owlthing_node = self.index.put(owl!("Thing"));
        let owlnothing_node = self.index.put(owl!("Nothing"));
        let owlsameas_node = self.index.put(owl!("sameAs"));
        let owlinverseof_node = self.index.put(owl!("inverseOf"));
        let owlsymmetricprop_node = self.index.put(owl!("SymmetricProperty"));
        let owlirreflexiveprop_node = self.index.put(owl!("IrreflexiveProperty"));
        let owlasymmetricprop_node = self.index.put(owl!("AsymmetricProperty"));
        let owltransitiveprop_node = self.index.put(owl!("TransitiveProperty"));
        let owlequivprop_node = self.index.put(owl!("equivalentProperty"));
        let owlequivclassprop_node = self.index.put(owl!("equivalentClass"));
        let owlfuncprop_node = self.index.put(owl!("FunctionalProperty"));
        let owlinvfuncprop_node = self.index.put(owl!("InverseFunctionalProperty"));
        let owlintersection_node = self.index.put(owl!("intersectionOf"));
        let owlunion_node = self.index.put(owl!("unionOf"));
        let owlhasvalue_node = self.index.put(owl!("hasValue"));
        let owlallvaluesfrom_node = self.index.put(owl!("allValuesFrom"));
        let owlsomevaluesfrom_node = self.index.put(owl!("someValuesFrom"));
        let owldisjointwith_node = self.index.put(owl!("disjointWith"));
        let owlonproperty_node = self.index.put(owl!("onProperty"));
        let owlcomplementof_node = self.index.put(owl!("complementOf"));
        let owl_pdw = self.index.put(owl!("propertyDisjointWith"));


        let rdf_type_relation = node_relation!(self, rdf!("type"));
        let rdf_type = self.iter1.variable::<(URI, URI)>("rdf_type");
        let rdf_type_inv = self.iter1.variable::<(URI, URI)>("rdf_type_inv");

        let prp_fp_isfuncprop = self.iter1.variable::<Triple>("a");
        let prp_fp_hasprop1 = self.iter1.variable::<Triple>("b");

        let rdfs_subclass_relation = node_relation!(self, rdfs!("subClassOf"));
        let owl_inter_relation = node_relation!(self, owl!("intersectionOf"));
        let owl_intersection_of = self.iter1.variable::<(URI, URI)>("owl_intersection_of");
        let owl_union_relation = node_relation!(self, owl!("unionOf"));
        let owl_union_of = self.iter1.variable::<(URI, URI)>("owl_union_of");
        let mut intersections: HashMap<URI, URI> = HashMap::new();
        let mut unions: HashMap<URI, URI> = HashMap::new();
        let mut instances: HashSet<(URI, URI)> = HashSet::new();
        let mut complements: HashMap<URI, URI> = HashMap::new();

        // in-memory indexes
        let pso = self.iter1.variable::<Triple>("pso");
        let osp = self.iter1.variable::<Triple>("osp");

        // prp-dom
        let prp_dom = self.iter1.variable::<(URI, URI)>("prp_dom");
        let rdfs_domain_relation = node_relation!(self, rdfs!("domain"));

        // prp-rng
        let prp_rng = self.iter1.variable::<(URI, URI)>("prp_rng");
        let rdfs_range_relation = node_relation!(self, rdfs!("range"));

        // prp-fp
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
        let owl_funcprop_relation = node_relation!(self, owl!("FunctionalProperty"));
        let prp_fp_1 = self.iter1.variable::<(URI, ())>("prp_fp_1");
        let prp_fp_2 = self.iter1.variable::<Triple>("prp_fp_2");
        // T(?p, ?x, ?y1), T(?p, ?x, ?y2) fulfilled from PSO index

        // prp-ifp
        // T(?p, rdf:type, owl:InverseFunctionalProperty
        // prp-ifp
        //      T(?p, rdf:type, owl:InverseFunctionalProperty) .
        //      T(?p, ?x1, ?y) . (pso)
        //      T(?p, ?x2, ?y) . (pso) =>
        //          T(?x1, owl:sameAs, ?x2) .
        let owl_invfuncprop_relation = node_relation!(self, owl!("InverseFunctionalProperty"));
        let prp_ifp_1 = self.iter1.variable::<(URI, ())>("prp_ifp_1");
        let prp_ifp_2 = self.iter1.variable::<Triple>("prp_ifp_2");
        // T(?p, ?x1, ?y), T(?p, ?x2, ?y) fulfilled from PSO index

        // prp-spo1
        // T(?p1, rdfs:subPropertyOf, ?p2) .
        // T(?p1, ?x, ?y) (pso) =>
        //  T(?x, ?p2, ?y)
        // IMPLS
        // T(?p1, rdfs:subPropertyOf, ?p2)
        let rdfs_subprop_relation = node_relation!(self, rdfs!("subPropertyOf"));
        let prp_spo1_1 = self.iter1.variable::<(URI, URI)>("prp_spo1_1");

        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        let owl_inv_relation = node_relation!(self, owl!("inverseOf"));
        let owl_inverse_of = self.iter1.variable::<(URI, URI)>("owl_inverseOf");
        let owl_inverse_of2 = self.iter1.variable::<(URI, URI)>("owl_inverse_of2");

        // eq-ref
        //  T(?s, ?p, ?o) =>
        //  T(?s, owl:sameAs, ?s)
        //  T(?p, owl:sameAs, ?p)
        //  T(?o, owl:sameAs, ?o)

        // eq-sym
        //  T(?x, owl:sameAs, ?y)  =>  T(?y, owl:sameAs, ?x)
        let owl_sameas_relation = node_relation!(self, owl!("sameAs"));
        let owl_same_as = self.iter1.variable::<(URI, URI)>("owl_same_as");

        // eq-trans
        // T(?x, owl:sameAs, ?y)
        // T(?y, owl:sameAs, ?z)  =>  T(?x, owl:sameAs, ?z)
        let eq_trans_1 = self.iter1.variable::<(URI, URI)>("eq_trans_1");

        // eq-rep-s, eq-rep-o, eq-rep-p
        // T(?s, owl:sameAs, ?s')
        // TODO: make more efficient
        // T(?s, ?p, ?o) => T(?s', ?p, ?o) (and then for p' and o')

        // prp-irp
        // T(?p, rdf:type, owl:IrreflexiveProperty)
        // T(?x, ?p, ?x) => false
        let owl_irreflex_relation = node_relation!(self, owl!("IrreflexiveProperty"));
        let owl_irreflexive = self.iter1.variable::<(URI, ())>("owl_irreflexive");
        let prp_irp_1 = self.iter1.variable::<(URI, URI)>("prp_irp_1");

        // prp-asyp
        //  T(?p, rdf:type, owl:AsymmetricProperty)
        //  T(?x, ?p, ?y)
        //  T(?y, ?p, ?x)  => false
        let owl_asymm_relation = node_relation!(self, owl!("AsymmetricProperty"));
        let owl_asymmetric = self.iter1.variable::<(URI, ())>("owl_asymmetric");
        let prp_asyp_1 = self.iter1.variable::<((URI, URI, URI), ())>("prp_asyp_1");
        let prp_asyp_2 = self.iter1.variable::<((URI, URI, URI), ())>("prp_asyp_2");
        let prp_asyp_3 = self.iter1.variable::<((URI, URI, URI), ())>("prp_asyp_3");

        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        //
        // (p1, p2)
        let prp_inv1 = self.iter1.variable::<(URI, URI)>("prp_inv1");

        // prp-pdw
        // T(?p1, owl:propertyDisjointWith, ?p2)
        // T(?x, ?p1, ?y)
        // T(?x, ?p2, ?y) => false
        // pairs of disjoint properties
        let owl_propdisjoint_relation = node_relation!(self, owl!("propertyDisjointWith"));
        let owl_propertydisjointwith = self.iter1.variable::<(URI, URI)>("owl_propertydisjointwith");
        // store the inverse; p2 pdw p1
        let owl_propertydisjointwith2 = self.iter1.variable::<(URI, URI)>("owl_propertydisjointwith2");
        let prp_pdw_1 = self.iter1.variable::<((URI, URI, URI), URI)>("prp_pdw_1");
        let prp_pdw_2 = self.iter1.variable::<((URI, URI, URI), URI)>("prp_pdw_2");
        let prp_pdw_3 = self.iter1.variable::<((URI, URI, URI), URI)>("prp_pdw_3");

        // prp-trp
        // T(?p, rdf:type, owl:TransitiveProperty)
        // T(?x, ?p, ?y)
        // T(?y, ?p, ?z) =>  T(?x, ?p, ?z)
        let owl_transitive_relation = node_relation!(self, owl!("TransitiveProperty"));
        let transitive_properties = self.iter1.variable::<(URI, ())>("transitive_properties");
        let prp_trp_1 = self.iter1.variable::<((URI, URI), URI)>("prp_trp_1");
        let prp_trp_2 = self.iter1.variable::<((URI, URI), URI)>("prp_trp_2");

        // prp-symp
        //      T(?p, rdf:type, owl:SymmetricProperty)
        //      T(?x, ?p, ?y)
        //      => T(?y, ?p, ?x)
        let owl_symprop_relation = node_relation!(self, owl!("SymmetricProperty"));
        let symmetric_properties = self.iter1.variable::<(URI, ())>("symmetric_properties");

        // prp-eqp1
        // T(?p1, owl:equivalentProperty, ?p2)
        // T(?x, ?p1, ?y)
        // => T(?x, ?p2, ?y)
        //
        // prp-eqp2
        // T(?p1, owl:equivalentProperty, ?p2)
        // T(?x, ?p2, ?y)
        // => T(?x, ?p1, ?y)
        let owl_equivprop_relation = node_relation!(self, owl!("equivalentProperty"));
        let equivalent_properties = self.iter1.variable::<(URI, URI)>("equivalent_properties");
        let equivalent_properties_2 = self.iter1.variable::<(URI, URI)>("equivalent_properties_2");


        // cls-nothing2
        //  T(?x, rdf:type, owl:Nothing)  => false
        let cls_nothing2 = self.iter1.variable::<(URI, ())>("cls_nothing2");
        let owl_nothing = node_relation!(self, owl!("Nothing"));


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
        let owl_hasvalue_relation = node_relation!(self, owl!("hasValue"));
        let owl_onprop_relation = node_relation!(self, owl!("onProperty"));
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
        let owl_allvalues_relation = node_relation!(self, owl!("allValuesFrom"));
        let owl_all_values_from = self.iter1.variable::<(URI, URI)>("owl_all_values_from");
        let cls_avf_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_avf_1");
        let cls_avf_2 = self.iter1.variable::<(URI, (URI, URI))>("cls_avf_2");

        // cls-svf1
        // T(?x, owl:someValuesFrom, ?y)
        // T(?x, owl:onProperty, ?p)
        // T(?u, ?p, ?v)
        // T(?v, rdf:type, ?y) =>  T(?u, rdf:type, ?x)
        let owl_somevalues_relation = node_relation!(self, owl!("someValuesFrom"));
        let owl_some_values_from = self.iter1.variable::<(URI, URI)>("owl_some_values_from");
        let cls_svf1_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_svf1_1");
        let cls_svf1_2 = self.iter1.variable::<(URI, (URI, URI, URI))>("cls_svf1_2");

        // cls-com
        let owl_complement_relation = node_relation!(self, owl!("complementOf"));
        let owl_complement_of = self.iter1.variable::<(URI, URI)>("owl_complement_of");
        let things = self.iter1.variable::<(URI, ())>("things");
        let cls_com_1 = self.iter1.variable::<(URI, (URI, URI))>("cls_com_1");
        let cls_com_2 = self.iter1.variable::<(URI, URI)>("cls_com_2");

        // cax-sco
        //  T(?c1, rdfs:subClassOf, ?c2)
        //  T(?c1, ?x, rdf:type) (osp) => T(?x, rdf:type, ?c2)
        //
        //  T(?c1, rdfs:subClassOf, ?c2)
        let cax_sco_1 = self.iter1.variable::<(URI, URI)>("cax_sco_1");
        //  T(?c1, ?x, rdf:type)
        let cax_sco_2 = self.iter1.variable::<(URI, URI)>("cax_sco_2");

        // cax-eqc1
        // T(?c1, owl:equivalentClass, ?c2), T(?x, rdf:type, ?c1)  =>
        //  T(?x, rdf:type, ?c2)
        // cax-eqc2
        // T(?c1, owl:equivalentClass, ?c2), T(?x, rdf:type, ?c2)  =>
        //  T(?x, rdf:type, ?c1)
        let owl_equivalent_relation = node_relation!(self, owl!("equivalentClass"));
        let owl_equivalent_class = self.iter1.variable::<(URI, URI)>("owl_equivalent_class");

        // cax-dw
        // T(?c1, owl:disjointWith, ?c2)
        // T(?x, rdf:type, ?c1)
        // T(?x, rdf:type, ?c2) => false
        let owl_disjointwith_relation = node_relation!(self, owl!("disjointWith"));
        let owl_disjoint_with = self.iter1.variable::<(URI, URI)>("owl_disjoint_with");
        let cax_dw_1 = self.iter1.variable::<(URI, (URI, URI))>("cax_dw_1");
        let cax_dw_2 = self.iter1.variable::<(URI, URI)>("cax_dw_2");


        let ds = DisjointSets::new(&self.input);

        self.all_triples_input.extend(self.input.iter().cloned());
        let mut changed = true;
        let mut established_complementary_instances: HashSet<Triple> = HashSet::new();
        let mut new_complementary_instances: HashSet<Triple> = HashSet::new();
        while changed {
            self.all_triples_input.extend(new_complementary_instances.drain());

            while self.iter1.changed() {
                // all individuals are owl:Things
                self.all_triples_input.from_map(&self.spo, |&(s, (_p, _o))| {
                    (s, (rdftype_node, owlthing_node))
                });

                self.spo.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (sub, (pred, obj)));
                pso.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (pred, (sub, obj)));
                osp.from_map(&self.all_triples_input, |&(sub, (pred, obj))| (obj, (sub, pred)));

                rdf_type.from_join(&pso, &rdf_type_relation, |&_, &tup, &()| {
                    instances.insert(tup);
                    tup
                });
                rdf_type_inv.from_map(&rdf_type, |&(inst, class)| { (class, inst) });

                // prp-dom
                prp_dom.from_join(&pso, &rdfs_domain_relation, |&_, &(pred, domain_class), &()| {
                    (pred, domain_class)
                });
                self.all_triples_input.from_join(&prp_dom, &pso, |&tpred, &class, &(sub, obj)| {
                    (sub, (rdftype_node, class))
                });

                // prp-rng
                prp_rng.from_join(&pso, &rdfs_range_relation, |&_, &(pred, domain_class), &()| {
                    (pred, domain_class)
                });
                self.all_triples_input.from_join(&prp_rng, &pso, |&tpred, &class, &(sub, obj)| {
                    (obj, (rdftype_node, class))
                });

                owl_inverse_of.from_join(&pso, &owl_inv_relation, |&_, &(p1, p2), &()| (p1, p2));
                owl_inverse_of2.from_map(&owl_inverse_of, |&(p1, p2)| (p2, p1) );

                owl_intersection_of.from_join(&pso, &owl_inter_relation, |&_, &(a, b), &()| {
                    intersections.insert(a, b);
                    (a, b)
                });

                owl_union_of.from_join(&pso, &owl_union_relation, |&_, &(a, b), &()| {
                    unions.insert(a, b);
                    (a, b)
                });

                owl_irreflexive.from_join(&rdf_type_inv, &owl_irreflex_relation, |&_, &inst, &()| (inst, ()));
                owl_asymmetric.from_join(&rdf_type_inv, &owl_asymm_relation, |&_, &inst, &()| (inst, ()));
                owl_propertydisjointwith.from_join(&pso, &owl_propdisjoint_relation, |&_, &(p1, p2), &()| (p1, p2));
                owl_propertydisjointwith2.from_map(&owl_propertydisjointwith, |&(p1, p2)| (p2, p1));

                owl_has_value.from_join(&pso, &owl_hasvalue_relation, |&_, &tup, &()| tup);
                owl_on_property.from_join(&pso, &owl_onprop_relation, |&_, &tup, &()| tup);
                owl_all_values_from.from_join(&pso, &owl_allvalues_relation, |&_, &tup, &()| tup);
                owl_some_values_from.from_join(&pso, &owl_somevalues_relation, |&_, &tup, &()| tup);
                owl_disjoint_with.from_join(&pso, &owl_disjointwith_relation, |&_, &tup, &()| tup);
                owl_same_as.from_join(&pso, &owl_sameas_relation, |&_, &tup, &()| tup);
                owl_complement_of.from_join(&pso, &owl_complement_relation, |&_, &(a, b), &()| {
                    complements.insert(a, b);
                    complements.insert(b, a);
                    (a, b)
                });
                owl_complement_of.from_join(&pso, &owl_complement_relation, |&_, &(a, b), &()| (b, a));
                owl_equivalent_class.from_join(&pso, &owl_equivalent_relation, |&_, &(c1, c2), &()| (c1, c2));
                owl_equivalent_class.from_join(&pso, &owl_equivalent_relation, |&_, &(c1, c2), &()| (c2, c1));
                symmetric_properties.from_join(&rdf_type_inv, &owl_symprop_relation, |&_, &inst, &()| (inst, ()));
                transitive_properties.from_join(&rdf_type_inv, &owl_transitive_relation, |&_, &inst, &()| (inst, ()));
                equivalent_properties.from_join(&pso, &owl_equivprop_relation, |&_, &(p1, p2), &()| (p1, p2));
                equivalent_properties_2.from_join(&pso, &owl_equivprop_relation, |&_, &(p1, p2), &()| (p2, p1));



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
                self.all_triples_input.from_join(&pso, &owl_same_as, |&p1, &(s, o), &p2| {
                    (s, (p2, o))
                });
                // eq-rep-o
                self.all_triples_input.from_join(&osp, &owl_same_as, |&o1, &(s, p), &o2| {
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

                // prp-irp
                // T(?p, rdf:type, owl:IrreflexiveProperty)
                // T(?x, ?p, ?x) => false
                prp_irp_1.from_join(&owl_irreflexive, &pso, |&p, &(), &(s, o)| {
                    if s == o && s > 0 && o > 0 {
                        let msg = format!("property {} of {} is irreflexive", self.to_u(p), self.to_u(s));
                        self.add_error("prp-irp".to_string(), msg.to_string());
                    }
                    (p, s)
                });

                // prp-asyp
                //  T(?p, rdf:type, owl:AsymmetricProperty)
                //  T(?x, ?p, ?y)
                //  T(?y, ?p, ?x) => false
                prp_asyp_1.from_join(&owl_asymmetric, &pso, |&p, &(), &(x, y)| ((x, y, p), ()) );
                prp_asyp_2.from_join(&owl_asymmetric, &pso, |&p, &(), &(x, y)| ((y, x, p), ()) );
                prp_asyp_3.from_join(&prp_asyp_1, &prp_asyp_2, |&(x, y, p), &(), &()| {
                    if x > 0 && y > 0 && p > 0 {
                        let msg = format!("property {} of {} and {} is asymmetric", self.to_u(p), self.to_u(x), self.to_u(y));
                        self.add_error("prp-asyp".to_string(), msg.to_string());
                    }
                    ((x, y, p), ())
                });




                // prp-fp
                prp_fp_1.from_join(&rdf_type_inv, &owl_funcprop_relation, |&_, &p, &()| (p, ()));
                prp_fp_2.from_join(&prp_fp_1, &pso, |&p, &(), &(x, y1)| (p, (x, y1)) );
                self.all_triples_input.from_join(&prp_fp_2, &pso, |&p, &(x1, y1), &(x2, y2)| {
                    // if x1 and x2 are the same, then emit y1 and y2 are the same
                    match x1 {
                        x2 => (y1, (owlsameas_node, y2)),
                        _ => (0, (0,0)),
                    }
                });

                // prp-ifp
                prp_ifp_1.from_join(&rdf_type_inv, &owl_invfuncprop_relation, |&_, &p, &()| (p, ()));
                prp_ifp_2.from_join(&prp_ifp_1, &pso, |&p, &(), &(x, y1)| (p, (x, y1)) );
                self.all_triples_input.from_join(&prp_ifp_2, &pso, |&p, &(x1, y1), &(x2, y2)| {
                    // if y1 and y2 are the same, then emit x1 and x2 are the same
                    match y1 {
                        y2 => (x1, (owlsameas_node, x2)),
                        _ => (0, (0,0)),
                    }
                });

                // prp-spo1
                prp_spo1_1.from_join(&pso, &rdfs_subprop_relation, |&_, &(p1, p2), &()| (p1, p2));
                self.all_triples_input.from_join(&prp_spo1_1, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)));

                // prp-inv1
                // T(?p1, owl:inverseOf, ?p2)
                // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
                self.all_triples_input.from_join(&owl_inverse_of, &pso, |&p1, &p2, &(x, y)| (y, (p2, x)) );

                // prp-inv2
                // T(?p1, owl:inverseOf, ?p2)
                // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
                self.all_triples_input.from_join(&owl_inverse_of2, &pso, |&p2, &p1, &(x, y)| (x, (p2, y)) );

                // cax-sco
                cax_sco_1.from_join(&pso, &rdfs_subclass_relation, |&_, &(c1, c2), &()| (c1, c2));
                // ?c1, ?x, rdf:type
                cax_sco_2.from_map(&rdf_type, |&(inst, class)| (class, inst));
                self.all_triples_input.from_join(&cax_sco_1, &cax_sco_2, |&class, &parent, &inst| (inst, (rdftype_node, parent)));

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
                    if inst1 == inst2 && inst1 > 0 && inst2 > 0 {
                        let msg = format!("inst {} is both {} and {} (disjoint classes)", self.to_u(inst1), self.to_u(c1), self.to_u(c2));
                        self.add_error("cax-dw".to_string(), msg.to_string());
                    }
                    (c2, inst1)
                });

                // prp-pdw
                // T(?p1, owl:propertyDisjointWith, ?p2)
                // T(?x, ?p1, ?y)
                // T(?x, ?p2, ?y) => false
                // returns pairs of (x,y) that should NOT exist for p2 because they exist for p1
                prp_pdw_1.from_join(&owl_propertydisjointwith, &pso, |&p1, &p2, &(x, y)| ((x, y, p2), p1));
                // returns pairs of (x,y) that do have p2
                prp_pdw_2.from_join(&owl_propertydisjointwith2, &pso, |&p2, &p1, &(x, y)| ((x, y, p2), p1));
                // join on (x,y) to find pairs in violation
                prp_pdw_3.from_join(&prp_pdw_1, &prp_pdw_2, |&(x, y, p2), &p1, &_p1| {
                    if x > 0 && y > 0 && p2 > 0 && p1 > 0 {
                        let msg = format!("property {} is disjoint with {} for pair ({} {} {})", self.to_u(p1), self.to_u(p2), self.to_u(x), self.to_u(p1), self.to_u(y));
                        self.add_error("prp-pdw".to_string(), msg.to_string());
                    }
                    ((x, y, p2), p1)
                });

                // prp-symp
                // T(?p, rdf:type, owl:SymmetricProperty)
                // T(?x, ?p, ?y)
                //  => T(?y, ?p, ?x)
                self.all_triples_input.from_join(&symmetric_properties, &pso, |&prop, &(), &(x, y)| {
                    (y, (prop, x))
                });

                // prp-trp
                // T(?p, rdf:type, owl:TransitiveProperty)
                // T(?x, ?p, ?y)
                // T(?y, ?p, ?z) =>  T(?x, ?p, ?z)
                prp_trp_1.from_join(&pso, &transitive_properties, |&p, &(x, y), &()| {
                    ((y, p), x)
                });
                prp_trp_2.from_join(&pso, &transitive_properties, |&p, &(y, z), &()| {
                    ((y, p), z)
                });
                self.all_triples_input.from_join(&prp_trp_1, &prp_trp_2, |&(y, p), &x, &z| {
                    (x, (p, z))
                });

                // prp-eqp1
                // T(?p1, owl:equivalentProperty, ?p2)
                // T(?x, ?p1, ?y)
                // => T(?x, ?p2, ?y)
                self.all_triples_input.from_join(&equivalent_properties, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );
                // prp-eqp2
                // T(?p1, owl:equivalentProperty, ?p2)
                // T(?x, ?p2, ?y)
                // => T(?x, ?p1, ?y)
                self.all_triples_input.from_join(&equivalent_properties_2, &pso, |&p1, &p2, &(x, y)| (x, (p2, y)) );

                // cls-nothing2
                //  T(?x, rdf:type, owl:Nothing) => false
                cls_nothing2.from_join(&rdf_type_inv, &owl_nothing, |&_nothing, &x, &()| {
                    if x > 0 {
                        let msg = format!("Instance {} is owl:Nothing (suggests unsatisfiability)", self.to_u(x));
                        self.add_error("cls-nothing2".to_string(), msg.to_string());
                    }
                    (x, ())
                });


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
                cls_com_1.from_join(&owl_complement_of, &rdf_type_inv, |&c1, &c2, &x| (c2, (x, c1)));
                cls_com_2.from_join(&rdf_type_inv, &cls_com_1, |&c2, &x_exists, &(x_bad, c1)| {
                    if x_exists == x_bad && x_exists > 0 && x_bad > 0 {
                        let msg = format!("Individual {} has type {} and {} which are complements", self.to_u(x_exists), self.to_u(c2), self.to_u(c1));
                        self.add_error("cls-com".to_string(), msg.to_string());
                    }
                    (x_bad, c1)
                });



                // Algorithm:
                // - for all pairs of complementary classes (c1, c2) where c1 owl:complementOf c2, find
                //   pairs where either c1 or c2 is an owl:Restriction
                // - if c1 is a Restriction and its complement c2 is not, then all individuals that are
                //   NOT of c1 should be instantiated as c2
                // - vice-versa if c2 is a Restriction and c1 is not
                // - if c1 and c2 are both Restrictions, this should probably be a warning
                // - if neither c1 or c2 are Restrictions, then anything not a c1 is a c2
                //
                // Key aspect: given an instance, how can I tell which class it *isn't*?
                //
                // A bad heuristic would be to assume that of a pair (c1, c2) where one is an
                // owl:Restriction and there isn't, the instance is of the non-Restriction class.
                // This is under the assumption that the definition of the restriction would have told
                // us if the instance met the requirements. However, in the middle of the reasoning
                // process, we can't really be sure if the instance meets the requirements. Thus, we
                // will need to handle the computation of this AFTER the reasoner has produced all
                // triples it can given the information available.
                //
                // This means we have 2 loops going. The "inner" loop is the reasoning implementation
                // *without* any owl:complementOf stuff. When that is done, we compute complements, add
                // these to the set of triples, and then re-run the inner loop. This "outer" loop runs
                // until no new triples are produced.

                // things.from_map(&rdf_type, |&(inst, class)| {
                //     if class == owlthing_node {
                //         (inst, ())
                //     } else {
                //         (0, ())
                //     }
                // });

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
                self.all_triples_input.from_join(&cls_hv2_1, &pso, |&prop, &(value, anonclass), &(sub, obj)| {
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
                cls_svf1_2.from_join(&cls_svf1_1, &pso, |&p, &(x, y), &(u, v)| {
                    (v, (x, y, u))
                });
                self.all_triples_input.from_join(&cls_svf1_2, &rdf_type, |&v, &(x, y, u), &class| {
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
                self.all_triples_input.from_join(&cls_svf1_1, &pso, |&p, &(x, y), &(u, v)| {
                    if y == owlthing_node {
                        (u, (rdftype_node, x))
                    } else {
                        (0, (0, 0))
                    }
                });
            }

            // Now that the inference stage has finished, we will compute the sets of instances for
            // complementary classes
            changed = false;
            for (c1, c2) in complements.iter() {
                // get all instances of NOT c1
                let c1_instances: HashSet<URI> = instances.iter().filter_map(|(inst, class)| {
                    if class == c1 {
                        Some(*inst)
                    } else {
                        None
                    }
                }).collect();
                let not_c1_instances: Vec<Triple> = instances.iter().filter_map(|(inst, class)| {
                    let triple = (*inst, (rdftype_node, *c2));
                    if c1_instances.contains(&inst) {
                        None
                    } else if established_complementary_instances.insert(triple) {
                        Some(triple)
                    } else {
                        None
                    }

                }).collect();
                if not_c1_instances.len() > 0 {
                    new_complementary_instances.extend(not_c1_instances);
                    changed = true;
                }
            }
        }
    }

    fn to_u(&self, u: URI) -> &str {
        self.index.get(u).unwrap()
    }


    /// Returns the vec of triples currently contained in the Reasoner
    pub fn get_triples(&mut self) -> Vec<(String, String, String)> {
        let instances = self.spo.clone().complete();

        self.rebuild_with(instances.clone());

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

fn node_to_string(n: &Node) -> String {
    match n {
        Node::UriNode{uri} => uri.to_string().clone(),
        Node::LiteralNode{literal, data_type: _, language: _} => literal.to_string(),
        Node::BlankNode{id} => format!("_:{}", id.to_string())
    }
}

fn add_node_to_graph(graph: &Graph, s: String, is_object: bool) -> Node {
    if s.starts_with("_:") {
        graph.create_blank_node_with_id(s.replacen("_:", "", 1))
    } else if is_object && (s.contains(" ") || suggests_literal(&s)) {
        graph.create_literal_node(escape_literal(&s))
    } else {
        graph.create_uri_node(&Uri::new(s))
    }
}

fn suggests_literal(pred: &String) -> bool {
    *pred == "http://www.w3.org/2000/01/rdf-schema#label".to_string() ||
    *pred == "http://www.w3.org/2000/01/rdf-schema#comment".to_string() ||
    *pred == "http://schema.org#email".to_string() ||
    *pred == "http://schema.org#name".to_string() ||
    *pred == "http://www.w3.org/2004/02/skos/core#definition".to_string() ||
    *pred == "http://purl.org/dc/elements/1.1/title"
}

fn escape_literal(literal: &str) -> String {
    literal.to_string().replace("\n", "\\n")
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
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
        r.load_triples_str(trips);
        r.reason();
        let res = r.get_triples();
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
        let res = r.get_triples();
        for i in res.iter() {
            let (s, p, o) = i;
            println!("{} {} {}", s, p, o);
        }
        // assert!(res.errors.len() > 0);
        Ok(())
    }
}
