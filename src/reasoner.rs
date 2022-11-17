//! The `owl` module implements the rules necessary for OWL 2 RL reasoning

extern crate datafrog;
use datafrog::{Iteration, Variable};

use crate::disjoint_sets::DisjointSets;
use crate::index::URIIndex;

use crate::common::*;
use crate::{node_relation, owl};
use log::{debug, error, info};
use oxrdf::{Graph, NamedNode, Term, Triple};
use rio_api::formatter::TriplesFormatter;
use rio_api::parser::TriplesParser;
use rio_turtle::TurtleFormatter;
use rio_turtle::{NTriplesParser, TurtleError, TurtleParser};
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::BufReader;
use std::io::{Error, ErrorKind};
use std::rc::Rc;

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
        ReasoningError { rule, message }
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
/// use reasonable::reasoner::Reasoner;
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
    input: Vec<KeyedTriple>,
    base: Vec<KeyedTriple>,
    errors: Vec<ReasoningError>,
    output: Vec<Triple>,

    spo: Variable<KeyedTriple>,
    pso: Variable<KeyedTriple>,
    osp: Variable<KeyedTriple>,
    all_triples_input: Variable<KeyedTriple>,
    rdf_type_inv: Rc<RefCell<Variable<(URI, URI)>>>,
    owl_intersection_of: Variable<(URI, URI)>,
    prp_dom: Variable<(URI, URI)>,
    prp_rng: Variable<(URI, URI)>,
    prp_fp_1: Variable<(URI, ())>,
    prp_fp_2: Variable<KeyedTriple>,
    prp_ifp_1: Variable<(URI, ())>,
    prp_ifp_2: Variable<KeyedTriple>,
    prp_spo1_1: Variable<(URI, URI)>,
    owl_inv1: Variable<(URI, URI)>,
    owl_inv2: Variable<(URI, URI)>,
    owl_same_as: Variable<(URI, URI)>,

    // list stuff
    established_complementary_instances: Rc<RefCell<HashSet<KeyedTriple>>>,
    intersections: Rc<RefCell<HashMap<URI, URI>>>,
    unions: Rc<RefCell<HashMap<URI, URI>>>,
    instances: Rc<RefCell<HashSet<(URI, URI)>>>,
    complements: Rc<RefCell<HashMap<URI, URI>>>,
}

#[allow(unused)]
impl Reasoner {
    /// Create a new Reasoner instance
    pub fn new() -> Self {
        let mut iter1 = Iteration::new();
        let mut index = URIIndex::new();

        // variables within the iteration
        let spo = iter1.variable::<(URI, (URI, URI))>("spo");
        let pso = iter1.variable::<(URI, (URI, URI))>("pso");
        let osp = iter1.variable::<(URI, (URI, URI))>("pso");
        let all_triples_input = iter1.variable::<(URI, (URI, URI))>("all_triples_input");

        // cls-thing, cls-nothing1
        let u_owl_thing = index.put(owl!("Thing"));
        let u_owl_nothing = index.put(owl!("Nothing"));
        let u_rdf_type = index.put(rdf!("type"));
        let u_owl_class = index.put(owl!("Class"));
        let mut input = vec![
            (u_owl_thing, (u_rdf_type, u_owl_class)),
            (u_owl_nothing, (u_rdf_type, u_owl_class)),
        ];

        let rdf_type_inv = Rc::new(RefCell::new(iter1.variable("rdf_type_inv")));
        let owl_intersection_of = iter1.variable::<(URI, URI)>("owl_intersection_of");
        let prp_dom = iter1.variable::<(URI, URI)>("prp_dom");
        let prp_rng = iter1.variable::<(URI, URI)>("prp_rng");
        let prp_fp_1 = iter1.variable::<(URI, ())>("prp_fp_1");
        let prp_fp_2 = iter1.variable::<KeyedTriple>("prp_fp_2");
        let prp_ifp_1 = iter1.variable::<(URI, ())>("prp_ifp_1");
        let prp_ifp_2 = iter1.variable::<KeyedTriple>("prp_ifp_2");
        let prp_spo1_1 = iter1.variable::<(URI, URI)>("prp_spo1_1");
        let owl_inv1 = iter1.variable::<(URI, URI)>("owl_inverseOf");
        let owl_inv2 = iter1.variable::<(URI, URI)>("owl_inverse_of2");
        let owl_same_as = iter1.variable::<(URI, URI)>("owl_same_as");
        let base = input.clone();
        Reasoner {
            iter1,
            index,
            input,
            base,
            errors: Vec::new(),
            output: Vec::new(),
            spo,
            pso,
            osp,
            all_triples_input,
            rdf_type_inv,
            owl_intersection_of,
            prp_dom,
            prp_rng,
            prp_fp_1,
            prp_fp_2,
            prp_ifp_1,
            prp_ifp_2,
            prp_spo1_1,
            owl_inv1,
            owl_inv2,
            owl_same_as,
            established_complementary_instances: Rc::new(RefCell::new(HashSet::new())),
            intersections: Rc::new(RefCell::new(HashMap::new())),
            unions: Rc::new(RefCell::new(HashMap::new())),
            instances: Rc::new(RefCell::new(HashSet::new())),
            complements: Rc::new(RefCell::new(HashMap::new())),
        }
    }

    /// Clears the state and uses the base triples (non-inferred)
    pub fn clear(&mut self) {
        self.input = self.base.clone();
    }

    fn rebuild(&mut self, output: Vec<KeyedTriple>) {
        // TODO: pull in the existing triples
        //self.iter1 = Iteration::new();
        self.input = output; //self.spo.clone().complete().iter().map(|&(x, (y, z))| (x, (y, z))).collect();
        self.all_triples_input = self
            .iter1
            .variable::<(URI, (URI, URI))>("all_triples_input");
        self.spo = self.iter1.variable::<(URI, (URI, URI))>("spo");
    }

    fn add_base_triples(&mut self, input: Vec<KeyedTriple>) {
        self.base.extend(input.clone());
        self.input.extend(input);
    }

    /// Load in a vector of triples
    #[allow(dead_code)]
    pub fn load_triples_str(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) {
        let mut trips: Vec<(URI, (URI, URI))> = triples
            .iter()
            .map(|trip| {
                (
                    self.index.put_str(trip.0).unwrap(),
                    (
                        self.index.put_str(trip.1).unwrap(),
                        self.index.put_str(trip.2).unwrap(),
                    ),
                )
            })
            .collect();
        trips.sort();
        get_unique(&self.input, &mut trips);
        self.add_base_triples(trips);
    }

    /// Load in a vector of triples
    pub fn load_triples(&mut self, mut triples: Vec<Triple>) {
        self.input.sort();
        let mut trips: Vec<(URI, (URI, URI))> = triples
            .iter()
            .map(|trip| {
                let (s, p, o) = (
                    trip.subject.clone(),
                    trip.predicate.clone(),
                    trip.object.clone(),
                );
                (
                    self.index.put(s.into()),
                    (self.index.put(p.into()), self.index.put(o)),
                )
            })
            .collect();
        trips.sort();
        get_unique(&self.input, &mut trips);
        self.add_base_triples(trips);
    }

    fn add_error(&mut self, rule: String, message: String) {
        let error = ReasoningError::new(rule, message);
        error!("Got error {}", error);
        self.errors.push(error);
    }

    /// Dump the contents of the reasoner to the given file.
    pub fn dump_file(&mut self, filename: &str) -> Result<(), Error> {
        // let mut abbrevs: HashMap<String, Uri> = HashMap::new();
        let mut output = fs::File::create(filename)?;
        let mut formatter = TurtleFormatter::new(output);
        for t in self.view_output() {
            formatter.format(&oxrdf_to_rio(t.into()))?;
        }

        //let mut writer =
        //    GraphSerializer::from_format(GraphFormat::Turtle).triple_writer(&output)?;
        //let mut graph = Graph::new(None);
        //graph.add_namespace(&Namespace::new(
        //    "owl".to_string(),
        //    Uri::new("http://www.w3.org/2002/07/owl#".to_string()),
        //));
        //graph.add_namespace(&Namespace::new(
        //    "rdf".to_string(),
        //    Uri::new("http://www.w3.org/1999/02/22-rdf-syntax-ns#".to_string()),
        //));
        //graph.add_namespace(&Namespace::new(
        //    "rdfs".to_string(),
        //    Uri::new("http://www.w3.org/2000/01/rdf-schema#".to_string()),
        //));
        //graph.add_namespace(&Namespace::new(
        //    "brick".to_string(),
        //    Uri::new("https://brickschema.org/schema/Brick#".to_string()),
        //));
        //graph.add_namespace(&Namespace::new(
        //    "tag".to_string(),
        //    Uri::new("https://brickschema.org/schema/BrickTag#".to_string()),
        //));
        //for t in self.view_output() {
        //    writer.write(t)?;
        //}

        //writer.finish()?;
        //info!("Wrote {} triples to {}", graph.count(), filename);
        //Ok(())
        formatter.finish()?;
        Ok(())
    }

    /// Load the triples in the given file into the Reasoner. This currently accepts
    /// Turtle-formatted (`.ttl`) and NTriples-formatted (`.n3`) files. If you have issues loading
    /// in a Turtle file, try converting it to NTriples
    pub fn load_file(&mut self, filename: &str) -> Result<(), Error> {
        let mut f = BufReader::new(fs::File::open(filename)?);
        let mut graph = Graph::new();
        if filename.ends_with(".ttl") {
            TurtleParser::new(f, None).parse_all(&mut |t| {
                graph.insert(&rio_to_oxrdf(t));
                Ok(()) as Result<(), TurtleError>
            })?;
        } else if filename.ends_with(".n3") {
            NTriplesParser::new(f).parse_all(&mut |t| {
                graph.insert(&rio_to_oxrdf(t));
                Ok(()) as Result<(), TurtleError>
            })?;
        } else {
            return Err(Error::new(
                ErrorKind::Other,
                "no parser for file (only ttl and n3)",
            ));
        }
        println!("filename {}", filename);

        //let graph = parser.read_triples(f)?.collect::<Result<Vec<_>,_>>()?;

        let mut triples: Vec<(URI, (URI, URI))> = graph
            .iter()
            .map(|_triple| {
                let triple = _triple;
                let (s, (p, o)) = (
                    self.index.put(triple.subject.clone().into()),
                    (
                        self.index.put(triple.predicate.clone().into()),
                        self.index.put(triple.object.clone().into()),
                    ),
                );
                (s, (p, o))
            })
            .collect();
        info!("Loaded {} triples from file {}", triples.len(), filename);

        triples.sort();
        get_unique(&self.input, &mut triples);

        //self.all_triples_input.insert(triples.into());
        self.add_base_triples(triples);

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

        //TODO: need to keep the variables persistent in the reasoner so they last between changes
        //to the input

        let rdf_type_relation = node_relation!(self, rdf!("type"));
        let rdf_type = self.iter1.variable::<(URI, URI)>("rdf_type");
        //let rdf_type_inv = self.iter1.variable::<(URI, URI)>("rdf_type_inv");

        let rdfs_subclass_relation = node_relation!(self, rdfs!("subClassOf"));
        let owl_inter_relation = node_relation!(self, owl!("intersectionOf"));
        //let owl_intersection_of = self.iter1.variable::<(URI, URI)>("owl_intersection_of");
        let owl_union_relation = node_relation!(self, owl!("unionOf"));
        let owl_union_of = self.iter1.variable::<(URI, URI)>("owl_union_of");
        //let mut intersections: HashMap<URI, URI> = HashMap::new();
        //let mut unions: HashMap<URI, URI> = HashMap::new();
        //let mut instances: HashSet<(URI, URI)> = HashSet::new();
        //let mut complements: HashMap<URI, URI> = HashMap::new();

        // in-memory indexes
        //let pso = self.iter1.variable::<Triple>("pso");
        //let osp = self.iter1.variable::<Triple>("osp");

        // prp-dom
        let rdfs_domain_relation = node_relation!(self, rdfs!("domain"));

        // prp-rng
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
        // T(?p, ?x, ?y1), T(?p, ?x, ?y2) fulfilled from PSO index

        // prp-ifp
        // T(?p, rdf:type, owl:InverseFunctionalProperty
        // prp-ifp
        //      T(?p, rdf:type, owl:InverseFunctionalProperty) .
        //      T(?p, ?x1, ?y) . (pso)
        //      T(?p, ?x2, ?y) . (pso) =>
        //          T(?x1, owl:sameAs, ?x2) .
        let owl_invfuncprop_relation = node_relation!(self, owl!("InverseFunctionalProperty"));
        // T(?p, ?x1, ?y), T(?p, ?x2, ?y) fulfilled from PSO index

        // prp-spo1
        // T(?p1, rdfs:subPropertyOf, ?p2) .
        // T(?p1, ?x, ?y) (pso) =>
        //  T(?x, ?p2, ?y)
        // IMPLS
        // T(?p1, rdfs:subPropertyOf, ?p2)
        let rdfs_subprop_relation = node_relation!(self, rdfs!("subPropertyOf"));

        // prp-inv1
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
        // prp-inv2
        // T(?p1, owl:inverseOf, ?p2)
        // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
        let owl_inv_relation = node_relation!(self, owl!("inverseOf"));

        // eq-ref
        //  T(?s, ?p, ?o) =>
        //  T(?s, owl:sameAs, ?s)
        //  T(?p, owl:sameAs, ?p)
        //  T(?o, owl:sameAs, ?o)

        // eq-sym
        //  T(?x, owl:sameAs, ?y)  =>  T(?y, owl:sameAs, ?x)
        let owl_sameas_relation = node_relation!(self, owl!("sameAs"));

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
        let owl_propertydisjointwith = self
            .iter1
            .variable::<(URI, URI)>("owl_propertydisjointwith");
        // store the inverse; p2 pdw p1
        let owl_propertydisjointwith2 = self
            .iter1
            .variable::<(URI, URI)>("owl_propertydisjointwith2");
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
        //let mut established_complementary_instances: HashSet<Triple> = HashSet::new();
        let mut new_complementary_instances: HashSet<KeyedTriple> = HashSet::new();
        while changed {
            self.all_triples_input
                .extend(new_complementary_instances.drain());

            while self.iter1.changed() {
                // all individuals are owl:Things
                self.all_triples_input
                    .from_map(&self.spo, |&(s, (_p, _o))| {
                        (s, (rdftype_node, owlthing_node))
                    });

                self.spo
                    .from_map(&self.all_triples_input, |&(sub, (pred, obj))| {
                        (sub, (pred, obj))
                    });
                self.pso
                    .from_map(&self.all_triples_input, |&(sub, (pred, obj))| {
                        (pred, (sub, obj))
                    });
                self.osp
                    .from_map(&self.all_triples_input, |&(sub, (pred, obj))| {
                        (obj, (sub, pred))
                    });

                rdf_type.from_join(&self.pso, &rdf_type_relation, |&_, &tup, &()| {
                    self.instances.borrow_mut().insert(tup);
                    tup
                });
                self.rdf_type_inv
                    .borrow_mut()
                    .from_map(&rdf_type, |&(inst, class)| (class, inst));

                // prp-dom
                self.prp_dom.from_join(
                    &self.pso,
                    &rdfs_domain_relation,
                    |&_, &(pred, domain_class), &()| (pred, domain_class),
                );
                self.all_triples_input.from_join(
                    &self.prp_dom,
                    &self.pso,
                    |&tpred, &class, &(sub, obj)| (sub, (rdftype_node, class)),
                );

                // prp-rng
                self.prp_rng.from_join(
                    &self.pso,
                    &rdfs_range_relation,
                    |&_, &(pred, domain_class), &()| (pred, domain_class),
                );
                self.all_triples_input.from_join(
                    &self.prp_rng,
                    &self.pso,
                    |&tpred, &class, &(sub, obj)| (obj, (rdftype_node, class)),
                );

                self.owl_inv1
                    .from_join(&self.pso, &owl_inv_relation, |&_, &(p1, p2), &()| (p1, p2));
                self.owl_inv2.from_map(&self.owl_inv1, |&(p1, p2)| (p2, p1));

                self.owl_intersection_of.from_join(
                    &self.pso,
                    &owl_inter_relation,
                    |&_, &(a, b), &()| {
                        self.intersections.borrow_mut().insert(a, b);
                        (a, b)
                    },
                );

                owl_union_of.from_join(&self.pso, &owl_union_relation, |&_, &(a, b), &()| {
                    self.unions.borrow_mut().insert(a, b);
                    (a, b)
                });

                owl_irreflexive.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_irreflex_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                owl_asymmetric.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_asymm_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                owl_propertydisjointwith.from_join(
                    &self.pso,
                    &owl_propdisjoint_relation,
                    |&_, &(p1, p2), &()| (p1, p2),
                );
                owl_propertydisjointwith2.from_map(&owl_propertydisjointwith, |&(p1, p2)| (p2, p1));

                owl_has_value.from_join(&self.pso, &owl_hasvalue_relation, |&_, &tup, &()| tup);
                owl_on_property.from_join(&self.pso, &owl_onprop_relation, |&_, &tup, &()| tup);
                owl_all_values_from.from_join(
                    &self.pso,
                    &owl_allvalues_relation,
                    |&_, &tup, &()| tup,
                );
                owl_some_values_from.from_join(
                    &self.pso,
                    &owl_somevalues_relation,
                    |&_, &tup, &()| tup,
                );
                owl_disjoint_with.from_join(
                    &self.pso,
                    &owl_disjointwith_relation,
                    |&_, &tup, &()| tup,
                );
                self.owl_same_as
                    .from_join(&self.pso, &owl_sameas_relation, |&_, &tup, &()| tup);
                owl_complement_of.from_join(
                    &self.pso,
                    &owl_complement_relation,
                    |&_, &(a, b), &()| {
                        self.complements.borrow_mut().insert(a, b);
                        self.complements.borrow_mut().insert(b, a);
                        (a, b)
                    },
                );
                owl_complement_of.from_join(
                    &self.pso,
                    &owl_complement_relation,
                    |&_, &(a, b), &()| (b, a),
                );
                owl_equivalent_class.from_join(
                    &self.pso,
                    &owl_equivalent_relation,
                    |&_, &(c1, c2), &()| (c1, c2),
                );
                owl_equivalent_class.from_join(
                    &self.pso,
                    &owl_equivalent_relation,
                    |&_, &(c1, c2), &()| (c2, c1),
                );
                symmetric_properties.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_symprop_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                transitive_properties.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_transitive_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                equivalent_properties.from_join(
                    &self.pso,
                    &owl_equivprop_relation,
                    |&_, &(p1, p2), &()| (p1, p2),
                );
                equivalent_properties_2.from_join(
                    &self.pso,
                    &owl_equivprop_relation,
                    |&_, &(p1, p2), &()| (p2, p1),
                );

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
                self.all_triples_input.from_join(
                    &self.spo,
                    &self.owl_same_as,
                    |&x, &(p, o), &y| (y, (owlsameas_node, x)),
                );

                // eq-rep-s
                self.all_triples_input.from_join(
                    &self.spo,
                    &self.owl_same_as,
                    |&s1, &(p, o), &s2| (s2, (p, o)),
                );
                // eq-rep-p
                self.all_triples_input.from_join(
                    &self.pso,
                    &self.owl_same_as,
                    |&p1, &(s, o), &p2| (s, (p2, o)),
                );
                // eq-rep-o
                self.all_triples_input.from_join(
                    &self.osp,
                    &self.owl_same_as,
                    |&o1, &(s, p), &o2| (s, (p, o2)),
                );

                // eq-trans
                // T(?x, owl:sameAs, ?y)
                // T(?y, owl:sameAs, ?z)  =>  T(?x, owl:sameAs, ?z)
                self.all_triples_input.from_join(
                    &self.owl_same_as,
                    &self.owl_same_as,
                    |&y, &z, &x| (x, (owlsameas_node, z)),
                );

                // prp-irp
                // T(?p, rdf:type, owl:IrreflexiveProperty)
                // T(?x, ?p, ?x) => false
                prp_irp_1.from_join(&owl_irreflexive, &self.pso, |&p, &(), &(s, o)| {
                    //if s == o && s > 0 && o > 0 {
                    //    let msg = format!(
                    //        "property {} of {} is irreflexive",
                    //        self.to_u(p),
                    //        self.to_u(s)
                    //    );
                    //    self.add_error("prp-irp".to_string(), msg);
                    //}
                    (p, s)
                });

                // prp-asyp
                //  T(?p, rdf:type, owl:AsymmetricProperty)
                //  T(?x, ?p, ?y)
                //  T(?y, ?p, ?x) => false
                prp_asyp_1.from_join(&owl_asymmetric, &self.pso, |&p, &(), &(x, y)| {
                    ((x, y, p), ())
                });
                prp_asyp_2.from_join(&owl_asymmetric, &self.pso, |&p, &(), &(x, y)| {
                    ((y, x, p), ())
                });
                prp_asyp_3.from_join(&prp_asyp_1, &prp_asyp_2, |&(x, y, p), &(), &()| {
                    if x > 0 && y > 0 && p > 0 {
                        let msg = format!(
                            "property {} of {} and {} is asymmetric",
                            self.to_u(p),
                            self.to_u(x),
                            self.to_u(y)
                        );
                        self.add_error("prp-asyp".to_string(), msg);
                    }
                    ((x, y, p), ())
                });

                // prp-fp
                self.prp_fp_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_funcprop_relation,
                    |&_, &p, &()| (p, ()),
                );
                self.prp_fp_2
                    .from_join(&self.prp_fp_1, &self.pso, |&p, &(), &(x, y1)| (p, (x, y1)));
                self.all_triples_input.from_join(
                    &self.prp_fp_2,
                    &self.pso,
                    |&p, &(x1, y1), &(x2, y2)| {
                        // if x1 and x2 are the same, then emit y1 and y2 are the same
                        if (x1 == x2) {
                            (y1, (owlsameas_node, y2))
                        } else {
                            (0, (0, 0))
                        }
                    },
                );

                // prp-ifp
                self.prp_ifp_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_invfuncprop_relation,
                    |&_, &p, &()| (p, ()),
                );
                self.prp_ifp_2
                    .from_join(&self.prp_ifp_1, &self.pso, |&p, &(), &(x, y1)| (p, (x, y1)));
                self.all_triples_input.from_join(
                    &self.prp_ifp_2,
                    &self.pso,
                    |&p, &(x1, y1), &(x2, y2)| {
                        // if y1 and y2 are the same, then emit x1 and x2 are the same
                        match y1 {
                            y2 => (x1, (owlsameas_node, x2)),
                            _ => (0, (0, 0)),
                        }
                    },
                );

                // prp-spo1
                self.prp_spo1_1.from_join(
                    &self.pso,
                    &rdfs_subprop_relation,
                    |&_, &(p1, p2), &()| (p1, p2),
                );
                self.all_triples_input.from_join(
                    &self.prp_spo1_1,
                    &self.pso,
                    |&p1, &p2, &(x, y)| (x, (p2, y)),
                );

                // prp-inv1
                // T(?p1, owl:inverseOf, ?p2)
                // T(?x, ?p1, ?y) => T(?y, ?p2, ?x)
                self.all_triples_input
                    .from_join(&self.owl_inv1, &self.pso, |&p1, &p2, &(x, y)| (y, (p2, x)));

                // prp-inv2
                // T(?p1, owl:inverseOf, ?p2)
                // T(?x, ?p2, ?y) => T(?y, ?p1, ?x)
                self.all_triples_input
                    .from_join(&self.owl_inv2, &self.pso, |&p2, &p1, &(x, y)| (x, (p2, y)));

                // cax-sco
                cax_sco_1.from_join(&self.pso, &rdfs_subclass_relation, |&_, &(c1, c2), &()| {
                    (c1, c2)
                });
                // ?c1, ?x, rdf:type
                cax_sco_2.from_map(&rdf_type, |&(inst, class)| (class, inst));

                self.all_triples_input.from_join(
                    &cax_sco_1,
                    &cax_sco_2,
                    |&class, &parent, &inst| {
                        //println!("instance: {:?} {:?} {:?}", self.to_u(inst), self.to_u(parent), self.to_u(class));
                        (inst, (rdftype_node, parent))
                    },
                );

                // cax-eqc1, cax-eqc2
                // find instances of classes that are equivalent
                self.all_triples_input.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_equivalent_class,
                    |&c1, &inst, &c2| (inst, (rdftype_node, c2)),
                );

                // cax-dw
                cax_dw_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_disjoint_with,
                    |&c1, &inst, &c2| (c2, (c1, inst)),
                );
                cax_dw_2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &cax_dw_1,
                    |&c2, &inst2, &(c1, inst1)| {
                        //if inst1 == inst2 && inst1 > 0 && inst2 > 0 {
                        //    let msg = format!(
                        //        "inst {} is both {} and {} (disjoint classes)",
                        //        self.to_u(inst1),
                        //        self.to_u(c1),
                        //        self.to_u(c2)
                        //    );
                        //    self.add_error("cax-dw".to_string(), msg);
                        //}
                        (c2, inst1)
                    },
                );

                // prp-pdw
                // T(?p1, owl:propertyDisjointWith, ?p2)
                // T(?x, ?p1, ?y)
                // T(?x, ?p2, ?y) => false
                // returns pairs of (x,y) that should NOT exist for p2 because they exist for p1
                prp_pdw_1.from_join(&owl_propertydisjointwith, &self.pso, |&p1, &p2, &(x, y)| {
                    ((x, y, p2), p1)
                });
                // returns pairs of (x,y) that do have p2
                prp_pdw_2.from_join(
                    &owl_propertydisjointwith2,
                    &self.pso,
                    |&p2, &p1, &(x, y)| ((x, y, p2), p1),
                );
                // join on (x,y) to find pairs in violation
                prp_pdw_3.from_join(&prp_pdw_1, &prp_pdw_2, |&(x, y, p2), &p1, &_p1| {
                    if x > 0 && y > 0 && p2 > 0 && p1 > 0 {
                        let msg = format!(
                            "property {} is disjoint with {} for pair ({} {} {})",
                            self.to_u(p1),
                            self.to_u(p2),
                            self.to_u(x),
                            self.to_u(p1),
                            self.to_u(y)
                        );
                        self.add_error("prp-pdw".to_string(), msg);
                    }
                    ((x, y, p2), p1)
                });

                // prp-symp
                // T(?p, rdf:type, owl:SymmetricProperty)
                // T(?x, ?p, ?y)
                //  => T(?y, ?p, ?x)
                self.all_triples_input.from_join(
                    &symmetric_properties,
                    &self.pso,
                    |&prop, &(), &(x, y)| (y, (prop, x)),
                );

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
                self.all_triples_input
                    .from_join(&prp_trp_1, &prp_trp_2, |&(y, p), &x, &z| (x, (p, z)));

                // prp-eqp1
                // T(?p1, owl:equivalentProperty, ?p2)
                // T(?x, ?p1, ?y)
                // => T(?x, ?p2, ?y)
                self.all_triples_input.from_join(
                    &equivalent_properties,
                    &self.pso,
                    |&p1, &p2, &(x, y)| (x, (p2, y)),
                );
                // prp-eqp2
                // T(?p1, owl:equivalentProperty, ?p2)
                // T(?x, ?p2, ?y)
                // => T(?x, ?p1, ?y)
                self.all_triples_input.from_join(
                    &equivalent_properties_2,
                    &self.pso,
                    |&p1, &p2, &(x, y)| (x, (p2, y)),
                );

                // cls-nothing2
                //  T(?x, rdf:type, owl:Nothing) => false
                cls_nothing2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_nothing,
                    |&_nothing, &x, &()| {
                        //if x > 0 {
                        //    let msg = format!(
                        //        "Instance {} is owl:Nothing (suggests unsatisfiability)",
                        //        self.to_u(x)
                        //    );
                        //    self.add_error("cls-nothing2".to_string(), msg);
                        //}
                        (x, ())
                    },
                );

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
                for (_intersection_class, _listname) in self.intersections.borrow().iter() {
                    let listname = *_listname;
                    let intersection_class = *_intersection_class;
                    if let Some(values) = ds.get_list_values(listname) {
                        // let value_uris: Vec<String> = values.iter().map(|v| node_to_string(self.index.get(*v).unwrap())).collect();
                        // debug!("{} {} (len {}) {} {:?}", listname, self.index.get(intersection_class).unwrap(), values.len(), self.index.get(listname).unwrap(), value_uris);
                        let mut class_counter: HashMap<URI, usize> = HashMap::new();
                        for (_inst, _list_class) in self.instances.borrow().iter() {
                            let inst = *_inst;
                            let list_class = *_list_class;
                            // debug!("inst {} values len {}, list class {}", self.index.get(inst).unwrap(), values.len(), list_class);
                            if values.contains(&list_class) {
                                // debug!("{:?} is a {:?}", inst, list_class);
                                let count = class_counter.entry(inst).or_insert(0);
                                *count += 1;
                            }
                        }
                        for (inst, num_implemented) in class_counter.iter() {
                            if *num_implemented == values.len() {
                                // debug!("inferred that {:?} is a {:?}", inst, intersection_class);
                                // println!("inferred {:?} is a {:?}", self.to_u(*inst), self.to_u(intersection_class));
                                new_cls_int1_instances
                                    .push((*inst, (rdftype_node, intersection_class)));
                            }
                        }
                    }
                }
                self.all_triples_input.extend(new_cls_int1_instances);

                // cls-int2
                let mut new_cls_int2_instances = Vec::new();
                cls_int_2_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_intersection_of,
                    |&intersection_class, &inst, &listname| {
                        if let Some(values) = ds.get_list_values(listname) {
                            for list_class in values {
                                new_cls_int2_instances.push((inst, (rdftype_node, list_class)));
                            }
                        }
                        (inst, new_cls_int2_instances.len() as URI)
                    },
                );
                self.all_triples_input.extend(new_cls_int2_instances);

                // cls-uni  T(?c, owl:unionOf, ?x)
                // LIST[?x, ?c1, ..., ?cn]
                // T(?y, rdf:type, ?ci) (for any i in 1-n) =>  T(?y, rdf:type, ?c)
                let mut new_cls_uni_instances = Vec::new();
                for (_union_class, _listname) in self.unions.borrow().iter() {
                    let listname = *_listname;
                    let union_class = *_union_class;
                    if let Some(values) = ds.get_list_values(listname) {
                        // let value_uris: Vec<String> = values.iter().map(|v| node_to_string(self.index.get(*v).unwrap())).collect();
                        // debug!("{} {} (len {}) {} {:?}", listname, self.index.get(union_class).unwrap(), values.len(), self.index.get(listname).unwrap(), value_uris);
                        let mut class_counter: HashMap<URI, usize> = HashMap::new();
                        for (_inst, _list_class) in self.instances.borrow().iter() {
                            let inst = *_inst;
                            let list_class = *_list_class;
                            // debug!("inst {} values len {}, list class {}", self.index.get(inst).unwrap(), values.len(), list_class);
                            if values.contains(&list_class) {
                                debug!("{} is a {}", inst, list_class);
                                let count = class_counter.entry(inst).or_insert(0);
                                *count += 1;
                            }
                        }
                        for (inst, num_implemented) in class_counter.iter() {
                            if *num_implemented > 0 {
                                // union instead of union
                                // debug!("inferred that {} is a {}", inst, union_class);
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
                cls_com_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &owl_complement_of,
                    |&c1, &x, &c2| (c2, (x, c1)),
                );
                cls_com_2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &cls_com_1,
                    |&c2, &x_exists, &(x_bad, c1)| {
                        //if x_exists == x_bad && x_exists > 0 && x_bad > 0 {
                        //    let msg = format!(
                        //        "Individual {} has type {} and {} which are complements",
                        //        self.to_u(x_exists),
                        //        self.to_u(c2),
                        //        self.to_u(c1)
                        //    );
                        //    self.add_error("cls-com".to_string(), msg);
                        //}
                        (x_bad, c1)
                    },
                );

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
                cls_hv1_1.from_join(&owl_has_value, &owl_on_property, |&x, &y, &p| (x, (p, y)));
                self.all_triples_input.from_join(
                    &self.rdf_type_inv.borrow(),
                    &cls_hv1_1,
                    |&x, &inst, &(prop, value)| (inst, (prop, value)),
                );

                // cls-hv2:
                // T(?x, owl:hasValue, ?y)
                // T(?x, owl:onProperty, ?p)
                // T(?u, ?p, ?y) =>  T(?u, rdf:type, ?x)
                cls_hv2_1.from_join(&owl_has_value, &owl_on_property, |&x, &y, &p| {
                    // format for pso index; needs property key
                    (p, (y, x))
                });
                self.all_triples_input.from_join(
                    &cls_hv2_1,
                    &self.pso,
                    |&prop, &(value, anonclass), &(sub, obj)| {
                        // if value is correct, then emit the rdf_type
                        if value == obj {
                            (sub, (rdftype_node, anonclass))
                        } else {
                            (0, (0, 0))
                        }
                    },
                );

                // cls-avf:
                // T(?x, owl:allValuesFrom, ?y)
                // T(?x, owl:onProperty, ?p)
                // T(?u, rdf:type, ?x)
                // T(?u, ?p, ?v) =>  T(?v, rdf:type, ?y)
                cls_avf_1.from_join(&owl_all_values_from, &owl_on_property, |&x, &y, &p| {
                    (x, (y, p))
                });
                cls_avf_2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &cls_avf_1,
                    |&x, &u, &(y, p)| (u, (p, y)),
                );
                self.all_triples_input.from_join(
                    &cls_avf_2,
                    &self.spo,
                    |&u, &(p1, y), &(p2, v)| {
                        if p1 == p2 {
                            (v, (rdftype_node, y))
                        } else {
                            (0, (0, 0))
                        }
                    },
                );

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
                self.all_triples_input.from_join(
                    &cls_svf1_2,
                    &rdf_type,
                    |&v, &(x, y, u), &class| {
                        if class == y {
                            (u, (rdftype_node, x))
                        } else {
                            (0, (0, 0))
                        }
                    },
                );

                // cls-svf2:
                //  T(?x, owl:someValuesFrom, owl:Thing)
                //  T(?x, owl:onProperty, ?p)
                //  T(?u, ?p, ?v) =>  T(?u, rdf:type, ?x)
                self.all_triples_input
                    .from_join(&cls_svf1_1, &self.pso, |&p, &(x, y), &(u, v)| {
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
            let mut est = self.established_complementary_instances.borrow_mut();
            for (c1, c2) in self.complements.borrow().iter() {
                // get all instances of NOT c1
                let c1_instances: HashSet<URI> = self
                    .instances
                    .borrow()
                    .iter()
                    .filter_map(|(inst, class)| if class == c1 { Some(*inst) } else { None })
                    .collect();
                let not_c1_instances: Vec<KeyedTriple> = self
                    .instances
                    .borrow()
                    .iter()
                    .filter_map(|(inst, class)| {
                        let triple = (*inst, (rdftype_node, *c2));
                        if c1_instances.contains(inst) {
                            None
                        } else if est.insert(triple) {
                            Some(triple)
                        } else {
                            None
                        }
                    })
                    .collect();
                if !not_c1_instances.is_empty() {
                    new_complementary_instances.extend(not_c1_instances);
                    changed = true;
                }
            }
        }

        let output: Vec<KeyedTriple> = self
            .spo
            .clone()
            .complete()
            .iter()
            .filter(|inst| {
                let (_s, (_p, _o)) = inst;
                *_s > 0 && *_p > 0 && *_o > 0
            })
            .cloned()
            .collect();
        self.output = output
            .iter()
            .map(|inst| {
                let (_s, (_p, _o)) = inst;
                let s = self.index.get(*_s).unwrap().clone();
                let p = self.index.get(*_p).unwrap().clone();
                let o = self.index.get(*_o).unwrap().clone();
                make_triple(s, p, o)
            })
            .filter_map(|t| match t {
                Ok(t) => Some(t),
                Err(e) => {
                    error!("Got error {:?}", e);
                    None
                }
            })
            .collect();
        self.rebuild(output);
    }

    fn to_u(&self, u: URI) -> String {
        self.index.get(u).unwrap().to_string()
    }

    /// Returns the vec of triples currently contained in the Reasoner
    pub fn get_triples(&self) -> Vec<Triple> {
        self.output.clone()
    }

    /// Returns the vec of triples currently contained in the Reasoner
    pub fn view_output(&self) -> &[Triple] {
        &self.output
    }

    pub fn get_input(&self) -> Vec<Triple> {
        self.base
            .iter()
            .map(|inst| {
                let (_s, (_p, _o)) = inst;
                let s = self.index.get(*_s).unwrap().clone();
                let p = self.index.get(*_p).unwrap().clone();
                let o = self.index.get(*_o).unwrap().clone();
                make_triple(s, p, o)
            })
            .filter_map(|t| match t {
                Ok(t) => Some(t),
                Err(e) => {
                    error!("Got error {:?}", e);
                    None
                }
            })
            .collect()
    }

    /// Returns the vec of triples currently contained in the Reasoner
    pub fn get_triples_string(&mut self) -> Vec<(String, String, String)> {
        self.view_output()
            .iter()
            .map(|inst| {
                (
                    inst.subject.to_string(),
                    inst.predicate.to_string(),
                    inst.object.to_string(),
                )
            })
            .collect()
    }
}

/// removes from rv the triples that are in src. src is sorted
pub fn get_unique(src: &[KeyedTriple], rv: &mut Vec<KeyedTriple>) {
    rv.retain(|t| !src.contains(t))
}
