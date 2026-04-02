//! The `owl` module implements the rules necessary for OWL 2 RL reasoning

extern crate datafrog;
use datafrog::{Iteration, Relation, Variable};

use crate::disjoint_sets::DisjointSets;
use crate::index::URIIndex;

use crate::common::*;
use log::{error, info};
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
use std::io::ErrorKind;
use std::rc::Rc;

/// Severity of a diagnostic produced during reasoning
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Info,
}

impl fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Info => write!(f, "info"),
        }
    }
}

/// Structured diagnostics that occur during reasoning
pub struct ReasoningError {
    /// Stable diagnostic code (e.g., OWLRL.CAX_DW)
    code: String,
    /// The OWL-RL rule that produced the violation (e.g., cax-dw)
    rule: String,
    /// Severity of the diagnostic
    severity: DiagnosticSeverity,
    /// A human-readable error message
    message: String,
    // TODO: add a trace of the productions that caused the error
}

impl ReasoningError {
    pub fn new(rule: String, message: String) -> Self {
        let code = match rule.as_str() {
            "cax-dw" => "OWLRL.CAX_DW",
            "prp-pdw" => "OWLRL.PRP_PDW",
            "cls-nothing2" => "OWLRL.CLS_NOTHING",
            "prp-asyp" => "OWLRL.PRP_ASYP",
            "prp-irp" => "OWLRL.PRP_IRP",
            "cls-com" => "OWLRL.CLS_COM",
            _ => "OWLRL.UNKNOWN",
        }
        .to_string();
        ReasoningError {
            code,
            rule,
            severity: DiagnosticSeverity::Error,
            message,
        }
    }

    pub fn code(&self) -> &str {
        &self.code
    }
    pub fn rule(&self) -> &str {
        &self.rule
    }
    pub fn severity(&self) -> &DiagnosticSeverity {
        &self.severity
    }
    pub fn message(&self) -> &str {
        &self.message
    }
}

impl fmt::Display for ReasoningError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}[{}]: {}",
            self.severity, self.rule, self.message
        )
    }
}

#[derive(Clone)]
pub struct ReasonerOptions {
    /// Whether to collect diagnostics during reasoning
    pub collect_diagnostics: bool,
    /// Maximum number of diagnostics to retain (None = unlimited)
    pub max_diagnostics: Option<usize>,
    /// Deduplicate diagnostics by (code, message)
    pub dedupe: bool,
}

impl Default for ReasonerOptions {
    fn default() -> Self {
        Self {
            collect_diagnostics: true,
            max_diagnostics: None,
            dedupe: true,
        }
    }
}

/// A convenience builder for constructing a `Reasoner` preloaded with files and/or triples.
#[derive(Default)]
pub struct ReasonerBuilder {
    files: Vec<String>,
    triples: Vec<Triple>,
    triples_str: Vec<(&'static str, &'static str, &'static str)>,
}

impl ReasonerBuilder {
    /// Creates a new `ReasonerBuilder`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds a file to be loaded during `build()`.
    pub fn with_file(mut self, path: impl Into<String>) -> Self {
        self.files.push(path.into());
        self
    }

    /// Adds triples to be loaded during `build()`.
    pub fn with_triples(mut self, triples: Vec<Triple>) -> Self {
        self.triples.extend(triples);
        self
    }

    /// Adds string-based triples to be loaded during `build()`.
    pub fn with_triples_str(mut self, triples: Vec<(&'static str, &'static str, &'static str)>) -> Self {
        self.triples_str.extend(triples);
        self
    }

    /// Builds a `Reasoner` and preloads configured files and triples.
    pub fn build(self) -> crate::error::Result<Reasoner> {
        let mut r = Reasoner::new();
        for f in self.files {
            r.load_file(&f)?;
        }
        if !self.triples_str.is_empty() {
            r.load_triples_str(self.triples_str);
        }
        if !self.triples.is_empty() {
            r.load_triples(self.triples);
        }
        Ok(r)
    }
}

/**
`Reasoner` is the interface to the reasoning engine. Instances of `Reasoner` maintain the state
required to do reasoning.

Basic usage:

```
use reasonable::reasoner::Reasoner;

let mut r = Reasoner::new();
// load files
r.load_file("../example_models/ontologies/Brick.n3")?;
r.load_file("../example_models/ontologies/rdfs.ttl")?;
// run reasoning
r.reason();
// inspect results
for t in r.view_output() {
    // do something with each triple
}
# Ok::<(), reasonable::error::ReasonableError>(())
```

Or use the builder for convenience:

```
use reasonable::reasoner::ReasonerBuilder;

let r = ReasonerBuilder::new()
    .with_file("../example_models/ontologies/Brick.n3")
    .with_file("../example_models/ontologies/rdfs.ttl")
    .with_triples_str(vec![
        ("urn:a", "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", "urn:SomeClass")
    ])
    .build()?;
// Reason over preloaded data
let mut r = r;
r.reason();
# Ok::<(), reasonable::error::ReasonableError>(())
```
*/
pub struct Reasoner {
    iter1: Iteration,
    index: URIIndex,
    input: Vec<KeyedTriple>,
    base: Vec<KeyedTriple>,
    errors: Vec<ReasoningError>,
    options: ReasonerOptions,
    seen_diags: HashSet<(String, String)>,
    output: Vec<Triple>,

    // --- URI constant nodes ---
    rdftype_node: URI,
    rdffirst_node: URI,
    rdfrest_node: URI,
    rdfnil_node: URI,
    rdfsdomain_node: URI,
    rdfsrange_node: URI,
    rdfssubprop_node: URI,
    rdfssubclass_node: URI,
    owlthing_node: URI,
    owlnothing_node: URI,
    owlsameas_node: URI,
    owlinverseof_node: URI,
    owlsymmetricprop_node: URI,
    owlirreflexiveprop_node: URI,
    owlasymmetricprop_node: URI,
    owltransitiveprop_node: URI,
    owlequivprop_node: URI,
    owlequivclassprop_node: URI,
    owlfuncprop_node: URI,
    owlinvfuncprop_node: URI,
    owlintersection_node: URI,
    owlunion_node: URI,
    owlhasvalue_node: URI,
    owlallvaluesfrom_node: URI,
    owlsomevaluesfrom_node: URI,
    owldisjointwith_node: URI,
    owlonproperty_node: URI,
    owlcomplementof_node: URI,
    owl_pdw_node: URI,

    // --- Static node relations (Relation, not Variable — not tracked by Iteration) ---
    rdf_type_relation: Relation<(URI, ())>,
    rdfs_subclass_relation: Relation<(URI, ())>,
    owl_inter_relation: Relation<(URI, ())>,
    owl_union_relation: Relation<(URI, ())>,
    rdfs_domain_relation: Relation<(URI, ())>,
    rdfs_range_relation: Relation<(URI, ())>,
    owl_funcprop_relation: Relation<(URI, ())>,
    owl_invfuncprop_relation: Relation<(URI, ())>,
    rdfs_subprop_relation: Relation<(URI, ())>,
    owl_inv_relation: Relation<(URI, ())>,
    owl_sameas_relation: Relation<(URI, ())>,
    owl_irreflex_relation: Relation<(URI, ())>,
    owl_asymm_relation: Relation<(URI, ())>,
    owl_propdisjoint_relation: Relation<(URI, ())>,
    owl_transitive_relation: Relation<(URI, ())>,
    owl_symprop_relation: Relation<(URI, ())>,
    owl_equivprop_relation: Relation<(URI, ())>,
    owl_nothing_relation: Relation<(URI, ())>,
    owl_hasvalue_relation: Relation<(URI, ())>,
    owl_onprop_relation: Relation<(URI, ())>,
    owl_allvalues_relation: Relation<(URI, ())>,
    owl_somevalues_relation: Relation<(URI, ())>,
    owl_complement_relation: Relation<(URI, ())>,
    owl_equivalent_relation: Relation<(URI, ())>,
    owl_disjointwith_relation: Relation<(URI, ())>,

    // --- Triple index variables ---
    spo: Variable<KeyedTriple>,
    pso: Variable<KeyedTriple>,
    osp: Variable<KeyedTriple>,
    all_triples_input: Variable<KeyedTriple>,

    // --- Rule-specific variables (persistent across reason() calls) ---
    rdf_type_inv: Rc<RefCell<Variable<(URI, URI)>>>,
    rdf_type: Variable<(URI, URI)>,
    owl_intersection_of: Variable<(URI, URI)>,
    owl_union_of: Variable<(URI, URI)>,
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
    owl_irreflexive: Variable<(URI, ())>,
    prp_irp_1: Variable<(URI, URI)>,
    owl_asymmetric: Variable<(URI, ())>,
    prp_asyp_1: Variable<((URI, URI, URI), ())>,
    prp_asyp_2: Variable<((URI, URI, URI), ())>,
    prp_asyp_3: Variable<((URI, URI, URI), ())>,
    prp_inv1: Variable<(URI, URI)>,
    owl_propertydisjointwith: Variable<(URI, URI)>,
    owl_propertydisjointwith2: Variable<(URI, URI)>,
    prp_pdw_1: Variable<((URI, URI, URI), URI)>,
    prp_pdw_2: Variable<((URI, URI, URI), URI)>,
    prp_pdw_3: Variable<((URI, URI, URI), URI)>,
    transitive_properties: Variable<(URI, ())>,
    prp_trp_1: Variable<((URI, URI), URI)>,
    prp_trp_2: Variable<((URI, URI), URI)>,
    symmetric_properties: Variable<(URI, ())>,
    equivalent_properties: Variable<(URI, URI)>,
    equivalent_properties_2: Variable<(URI, URI)>,
    cls_nothing2: Variable<(URI, ())>,
    cls_int_2_1: Variable<(URI, URI)>,
    owl_has_value: Variable<(URI, URI)>,
    owl_on_property: Variable<(URI, URI)>,
    cls_hv1_1: Variable<(URI, (URI, URI))>,
    cls_hv2_1: Variable<(URI, (URI, URI))>,
    owl_all_values_from: Variable<(URI, URI)>,
    cls_avf_1: Variable<(URI, (URI, URI))>,
    cls_avf_2: Variable<(URI, (URI, URI))>,
    owl_some_values_from: Variable<(URI, URI)>,
    cls_svf1_1: Variable<(URI, (URI, URI))>,
    cls_svf1_2: Variable<(URI, (URI, URI, URI))>,
    owl_complement_of: Variable<(URI, URI)>,
    things: Variable<(URI, ())>,
    cls_com_1: Variable<(URI, (URI, URI))>,
    cls_com_2: Variable<(URI, URI)>,
    cax_sco_1: Variable<(URI, URI)>,
    cax_sco_2: Variable<(URI, URI)>,
    owl_equivalent_class: Variable<(URI, URI)>,
    owl_disjoint_with: Variable<(URI, URI)>,
    cax_dw_1: Variable<(URI, (URI, URI))>,
    cax_dw_2: Variable<(URI, URI)>,
    union_mem_var: Variable<(URI, URI)>,

    // --- Auxiliary state ---
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

        // --- URI constant nodes ---
        let rdftype_node = index.put(rdf!("type"));
        let rdffirst_node = index.put(rdf!("first"));
        let rdfrest_node = index.put(rdf!("rest"));
        let rdfnil_node = index.put(rdf!("nil"));
        let rdfsdomain_node = index.put(rdfs!("domain"));
        let rdfsrange_node = index.put(rdfs!("range"));
        let rdfssubprop_node = index.put(rdfs!("subPropertyOf"));
        let rdfssubclass_node = index.put(rdfs!("subClassOf"));
        let owlthing_node = index.put(owl!("Thing"));
        let owlnothing_node = index.put(owl!("Nothing"));
        let owlsameas_node = index.put(owl!("sameAs"));
        let owlinverseof_node = index.put(owl!("inverseOf"));
        let owlsymmetricprop_node = index.put(owl!("SymmetricProperty"));
        let owlirreflexiveprop_node = index.put(owl!("IrreflexiveProperty"));
        let owlasymmetricprop_node = index.put(owl!("AsymmetricProperty"));
        let owltransitiveprop_node = index.put(owl!("TransitiveProperty"));
        let owlequivprop_node = index.put(owl!("equivalentProperty"));
        let owlequivclassprop_node = index.put(owl!("equivalentClass"));
        let owlfuncprop_node = index.put(owl!("FunctionalProperty"));
        let owlinvfuncprop_node = index.put(owl!("InverseFunctionalProperty"));
        let owlintersection_node = index.put(owl!("intersectionOf"));
        let owlunion_node = index.put(owl!("unionOf"));
        let owlhasvalue_node = index.put(owl!("hasValue"));
        let owlallvaluesfrom_node = index.put(owl!("allValuesFrom"));
        let owlsomevaluesfrom_node = index.put(owl!("someValuesFrom"));
        let owldisjointwith_node = index.put(owl!("disjointWith"));
        let owlonproperty_node = index.put(owl!("onProperty"));
        let owlcomplementof_node = index.put(owl!("complementOf"));
        let owl_pdw_node = index.put(owl!("propertyDisjointWith"));
        let u_owl_class = index.put(owl!("Class"));

        // Helper to create a single-element Relation for a URI node
        let node_rel = |uri: URI| -> Relation<(URI, ())> {
            Relation::from_vec(vec![(uri, ())])
        };

        // --- Static node relations (Relation type — not tracked by Iteration) ---
        let rdf_type_relation = node_rel(rdftype_node);
        let rdfs_subclass_relation = node_rel(rdfssubclass_node);
        let owl_inter_relation = node_rel(owlintersection_node);
        let owl_union_relation = node_rel(owlunion_node);
        let rdfs_domain_relation = node_rel(rdfsdomain_node);
        let rdfs_range_relation = node_rel(rdfsrange_node);
        let owl_funcprop_relation = node_rel(owlfuncprop_node);
        let owl_invfuncprop_relation = node_rel(owlinvfuncprop_node);
        let rdfs_subprop_relation = node_rel(rdfssubprop_node);
        let owl_inv_relation = node_rel(owlinverseof_node);
        let owl_sameas_relation = node_rel(owlsameas_node);
        let owl_irreflex_relation = node_rel(owlirreflexiveprop_node);
        let owl_asymm_relation = node_rel(owlasymmetricprop_node);
        let owl_propdisjoint_relation = node_rel(owl_pdw_node);
        let owl_transitive_relation = node_rel(owltransitiveprop_node);
        let owl_symprop_relation = node_rel(owlsymmetricprop_node);
        let owl_equivprop_relation = node_rel(owlequivprop_node);
        let owl_nothing_relation = node_rel(owlnothing_node);
        let owl_hasvalue_relation = node_rel(owlhasvalue_node);
        let owl_onprop_relation = node_rel(owlonproperty_node);
        let owl_allvalues_relation = node_rel(owlallvaluesfrom_node);
        let owl_somevalues_relation = node_rel(owlsomevaluesfrom_node);
        let owl_complement_relation = node_rel(owlcomplementof_node);
        let owl_equivalent_relation = node_rel(owlequivclassprop_node);
        let owl_disjointwith_relation = node_rel(owldisjointwith_node);

        // --- Triple index variables ---
        let spo = iter1.variable::<KeyedTriple>("spo");
        let pso = iter1.variable::<KeyedTriple>("pso");
        let osp = iter1.variable::<KeyedTriple>("osp");
        let all_triples_input = iter1.variable::<KeyedTriple>("all_triples_input");

        // --- Rule-specific variables ---
        let rdf_type_inv = Rc::new(RefCell::new(iter1.variable("rdf_type_inv")));
        let rdf_type = iter1.variable::<(URI, URI)>("rdf_type");
        let owl_intersection_of = iter1.variable::<(URI, URI)>("owl_intersection_of");
        let owl_union_of = iter1.variable::<(URI, URI)>("owl_union_of");
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
        let owl_irreflexive = iter1.variable::<(URI, ())>("owl_irreflexive");
        let prp_irp_1 = iter1.variable::<(URI, URI)>("prp_irp_1");
        let owl_asymmetric = iter1.variable::<(URI, ())>("owl_asymmetric");
        let prp_asyp_1 = iter1.variable::<((URI, URI, URI), ())>("prp_asyp_1");
        let prp_asyp_2 = iter1.variable::<((URI, URI, URI), ())>("prp_asyp_2");
        let prp_asyp_3 = iter1.variable::<((URI, URI, URI), ())>("prp_asyp_3");
        let prp_inv1 = iter1.variable::<(URI, URI)>("prp_inv1");
        let owl_propertydisjointwith = iter1.variable::<(URI, URI)>("owl_propertydisjointwith");
        let owl_propertydisjointwith2 = iter1.variable::<(URI, URI)>("owl_propertydisjointwith2");
        let prp_pdw_1 = iter1.variable::<((URI, URI, URI), URI)>("prp_pdw_1");
        let prp_pdw_2 = iter1.variable::<((URI, URI, URI), URI)>("prp_pdw_2");
        let prp_pdw_3 = iter1.variable::<((URI, URI, URI), URI)>("prp_pdw_3");
        let transitive_properties = iter1.variable::<(URI, ())>("transitive_properties");
        let prp_trp_1 = iter1.variable::<((URI, URI), URI)>("prp_trp_1");
        let prp_trp_2 = iter1.variable::<((URI, URI), URI)>("prp_trp_2");
        let symmetric_properties = iter1.variable::<(URI, ())>("symmetric_properties");
        let equivalent_properties = iter1.variable::<(URI, URI)>("equivalent_properties");
        let equivalent_properties_2 = iter1.variable::<(URI, URI)>("equivalent_properties_2");
        let cls_nothing2 = iter1.variable::<(URI, ())>("cls_nothing2");
        let cls_int_2_1 = iter1.variable::<(URI, URI)>("cls_int_2_1");
        let owl_has_value = iter1.variable::<(URI, URI)>("owl_has_value");
        let owl_on_property = iter1.variable::<(URI, URI)>("owl_on_property");
        let cls_hv1_1 = iter1.variable::<(URI, (URI, URI))>("cls_hv1_1");
        let cls_hv2_1 = iter1.variable::<(URI, (URI, URI))>("cls_hv2_1");
        let owl_all_values_from = iter1.variable::<(URI, URI)>("owl_all_values_from");
        let cls_avf_1 = iter1.variable::<(URI, (URI, URI))>("cls_avf_1");
        let cls_avf_2 = iter1.variable::<(URI, (URI, URI))>("cls_avf_2");
        let owl_some_values_from = iter1.variable::<(URI, URI)>("owl_some_values_from");
        let cls_svf1_1 = iter1.variable::<(URI, (URI, URI))>("cls_svf1_1");
        let cls_svf1_2 = iter1.variable::<(URI, (URI, URI, URI))>("cls_svf1_2");
        let owl_complement_of = iter1.variable::<(URI, URI)>("owl_complement_of");
        let things = iter1.variable::<(URI, ())>("things");
        let cls_com_1 = iter1.variable::<(URI, (URI, URI))>("cls_com_1");
        let cls_com_2 = iter1.variable::<(URI, URI)>("cls_com_2");
        let cax_sco_1 = iter1.variable::<(URI, URI)>("cax_sco_1");
        let cax_sco_2 = iter1.variable::<(URI, URI)>("cax_sco_2");
        let owl_equivalent_class = iter1.variable::<(URI, URI)>("owl_equivalent_class");
        let owl_disjoint_with = iter1.variable::<(URI, URI)>("owl_disjoint_with");
        let cax_dw_1 = iter1.variable::<(URI, (URI, URI))>("cax_dw_1");
        let cax_dw_2 = iter1.variable::<(URI, URI)>("cax_dw_2");
        let union_mem_var = iter1.variable::<(URI, URI)>("union_memberships");

        // cls-thing, cls-nothing1 — seed triples
        let input = vec![
            (owlthing_node, (rdftype_node, u_owl_class)),
            (owlnothing_node, (rdftype_node, u_owl_class)),
        ];
        let base = input.clone();

        Reasoner {
            iter1,
            index,
            input,
            base,
            errors: Vec::new(),
            options: ReasonerOptions::default(),
            seen_diags: HashSet::new(),
            output: Vec::new(),
            // URI constants
            rdftype_node,
            rdffirst_node,
            rdfrest_node,
            rdfnil_node,
            rdfsdomain_node,
            rdfsrange_node,
            rdfssubprop_node,
            rdfssubclass_node,
            owlthing_node,
            owlnothing_node,
            owlsameas_node,
            owlinverseof_node,
            owlsymmetricprop_node,
            owlirreflexiveprop_node,
            owlasymmetricprop_node,
            owltransitiveprop_node,
            owlequivprop_node,
            owlequivclassprop_node,
            owlfuncprop_node,
            owlinvfuncprop_node,
            owlintersection_node,
            owlunion_node,
            owlhasvalue_node,
            owlallvaluesfrom_node,
            owlsomevaluesfrom_node,
            owldisjointwith_node,
            owlonproperty_node,
            owlcomplementof_node,
            owl_pdw_node,
            // Static relations
            rdf_type_relation,
            rdfs_subclass_relation,
            owl_inter_relation,
            owl_union_relation,
            rdfs_domain_relation,
            rdfs_range_relation,
            owl_funcprop_relation,
            owl_invfuncprop_relation,
            rdfs_subprop_relation,
            owl_inv_relation,
            owl_sameas_relation,
            owl_irreflex_relation,
            owl_asymm_relation,
            owl_propdisjoint_relation,
            owl_transitive_relation,
            owl_symprop_relation,
            owl_equivprop_relation,
            owl_nothing_relation,
            owl_hasvalue_relation,
            owl_onprop_relation,
            owl_allvalues_relation,
            owl_somevalues_relation,
            owl_complement_relation,
            owl_equivalent_relation,
            owl_disjointwith_relation,
            // Index variables
            spo,
            pso,
            osp,
            all_triples_input,
            // Rule variables
            rdf_type_inv,
            rdf_type,
            owl_intersection_of,
            owl_union_of,
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
            owl_irreflexive,
            prp_irp_1,
            owl_asymmetric,
            prp_asyp_1,
            prp_asyp_2,
            prp_asyp_3,
            prp_inv1,
            owl_propertydisjointwith,
            owl_propertydisjointwith2,
            prp_pdw_1,
            prp_pdw_2,
            prp_pdw_3,
            transitive_properties,
            prp_trp_1,
            prp_trp_2,
            symmetric_properties,
            equivalent_properties,
            equivalent_properties_2,
            cls_nothing2,
            cls_int_2_1,
            owl_has_value,
            owl_on_property,
            cls_hv1_1,
            cls_hv2_1,
            owl_all_values_from,
            cls_avf_1,
            cls_avf_2,
            owl_some_values_from,
            cls_svf1_1,
            cls_svf1_2,
            owl_complement_of,
            things,
            cls_com_1,
            cls_com_2,
            cax_sco_1,
            cax_sco_2,
            owl_equivalent_class,
            owl_disjoint_with,
            cax_dw_1,
            cax_dw_2,
            union_mem_var,
            // Auxiliary state
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

    fn add_base_triples(&mut self, input: Vec<KeyedTriple>) {
        self.base.extend(input.clone());
        self.input.extend(input);
    }

    /// Loads a vector of triples given as string URIs.
    ///
    /// Example:
    /// ```
    /// # use reasonable::reasoner::Reasoner;
    /// let mut r = Reasoner::new();
    /// r.load_triples_str(vec![
    ///   ("urn:a", "http://www.w3.org/1999/02/22-rdf-syntax-ns#type", "urn:SomeClass")
    /// ]);
    /// ```
    #[allow(dead_code)]
    pub fn load_triples_str(&mut self, triples: Vec<(&'static str, &'static str, &'static str)>) {
        let mut trips: Vec<(URI, (URI, URI))> = Vec::with_capacity(triples.len());
        for trip in triples.iter() {
            if let (Ok(s), Ok(p), Ok(o)) = (
                self.index.put_str(trip.0),
                self.index.put_str(trip.1),
                self.index.put_str(trip.2),
            ) {
                trips.push((s, (p, o)));
            }
        }
        trips.sort_unstable();
        // Ensure src is sorted for linear merge
        self.input.sort_unstable();
        get_unique(&self.input, &mut trips);
        self.add_base_triples(trips);
    }

    /// Loads a vector of triples.
    ///
    /// Example:
    /// ```
    /// # use reasonable::reasoner::Reasoner;
    /// # use oxrdf::{NamedNode, Triple, Subject, Term};
    /// # fn build_triple() -> Triple {
    /// #   let nn = NamedNode::new_unchecked("urn:a".to_string());
    /// #   Triple::new(Subject::NamedNode(nn.clone()), nn.clone(), Term::NamedNode(nn))
    /// # }
    /// let mut r = Reasoner::new();
    /// r.load_triples(vec![build_triple()]);
    /// ```
    pub fn load_triples(&mut self, mut triples: Vec<Triple>) {
        // Ensure src is sorted for linear merge
        self.input.sort_unstable();
        let mut trips: Vec<(URI, (URI, URI))> = Vec::with_capacity(triples.len());
        for trip in triples.iter() {
            let s = self.index.put(trip.subject.clone().into());
            let p = self.index.put(trip.predicate.clone().into());
            let o = self.index.put(trip.object.clone().into());
            trips.push((s, (p, o)));
        }
        trips.sort_unstable();
        get_unique(&self.input, &mut trips);
        self.add_base_triples(trips);
    }

    fn add_error(&mut self, rule: String, message: String) {
        if !self.options.collect_diagnostics {
            return;
        }
        let error = ReasoningError::new(rule, message);
        if self.options.dedupe {
            let key = (error.code.clone(), error.message.clone());
            if !self.seen_diags.insert(key) {
                return;
            }
        }
        if let Some(max) = self.options.max_diagnostics {
            if self.errors.len() >= max {
                return;
            }
        }
        // Do not log each diagnostic via log to avoid noise; tests and CLI will consume via API
        self.errors.push(error);
    }

    /// Returns a read-only view of errors detected during reasoning (e.g., disjointness violations).
    pub fn errors(&self) -> &[ReasoningError] {
        &self.errors
    }

    /// Returns diagnostics (alias of errors for backward compatibility)
    pub fn diagnostics(&self) -> &[ReasoningError] {
        &self.errors
    }

    /// Updates the reasoning options
    pub fn set_options(&mut self, opts: ReasonerOptions) {
        self.options = opts;
    }

    /// Dumps the current inferred triples to a Turtle file.
    ///
    /// The file will contain the current output of the reasoner (post `reason()`).
    pub fn dump_file(&mut self, filename: &str) -> crate::error::Result<()> {
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

    /// Loads triples from a file into the Reasoner.
    ///
    /// Supports:
    /// - Turtle files: ".ttl"
    /// - N-Triples files: ".n3"
    ///
    /// Returns an error for unsupported extensions.
    pub fn load_file(&mut self, filename: &str) -> crate::error::Result<()> {
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
            return Err(std::io::Error::new(
                ErrorKind::Other,
                "no parser for file (only ttl and n3)",
            )
            .into());
        }

        //let graph = parser.read_triples(f)?.collect::<Result<Vec<_>,_>>()?;

        // Build new triples with capacity hints
        let mut triples: Vec<(URI, (URI, URI))> = Vec::with_capacity(graph.len());
        for triple in graph.iter() {
            let s = self.index.put(triple.subject.clone().into());
            let p = self.index.put(triple.predicate.clone().into());
            let o = self.index.put(triple.object.clone().into());
            triples.push((s, (p, o)));
        }
        info!("Loaded {} triples from file {}", triples.len(), filename);

        triples.sort_unstable();
        // Ensure src is sorted for linear merge
        self.input.sort_unstable();
        get_unique(&self.input, &mut triples);

        self.add_base_triples(triples);

        Ok(())
    }

    /// Performs OWL 2 RL-compatible reasoning on the triples currently loaded into the `Reasoner`.
    ///
    /// The inferred closure is preserved in the internal state and subsequent calls seed from the
    /// previously materialized closure unless `clear()` is called.
    pub fn reason(&mut self) {
        // Copy URI constants to local variables for use in closures
        let rdftype_node = self.rdftype_node;
        let owlthing_node = self.owlthing_node;
        let owlsameas_node = self.owlsameas_node;

        let ds = DisjointSets::new(&self.input, self.rdffirst_node, self.rdfrest_node, self.rdfnil_node);

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

                self.rdf_type.from_join(&self.pso, &self.rdf_type_relation, |&_, &tup, &()| {
                    self.instances.borrow_mut().insert(tup);
                    tup
                });
                self.rdf_type_inv
                    .borrow_mut()
                    .from_map(&self.rdf_type, |&(inst, class)| (class, inst));

                // prp-dom
                self.prp_dom.from_join(
                    &self.pso,
                    &self.rdfs_domain_relation,
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
                    &self.rdfs_range_relation,
                    |&_, &(pred, domain_class), &()| (pred, domain_class),
                );
                self.all_triples_input.from_join(
                    &self.prp_rng,
                    &self.pso,
                    |&tpred, &class, &(sub, obj)| (obj, (rdftype_node, class)),
                );

                self.owl_inv1
                    .from_join(&self.pso, &self.owl_inv_relation, |&_, &(p1, p2), &()| (p1, p2));
                self.owl_inv2.from_map(&self.owl_inv1, |&(p1, p2)| (p2, p1));

                self.owl_intersection_of.from_join(
                    &self.pso,
                    &self.owl_inter_relation,
                    |&_, &(a, b), &()| {
                        self.intersections.borrow_mut().insert(a, b);
                        (a, b)
                    },
                );

                self.owl_union_of.from_join(&self.pso, &self.owl_union_relation, |&_, &(a, b), &()| {
                    self.unions.borrow_mut().insert(a, b);
                    (a, b)
                });

                self.owl_irreflexive.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_irreflex_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                self.owl_asymmetric.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_asymm_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                self.owl_propertydisjointwith.from_join(
                    &self.pso,
                    &self.owl_propdisjoint_relation,
                    |&_, &(p1, p2), &()| (p1, p2),
                );
                self.owl_propertydisjointwith2.from_map(&self.owl_propertydisjointwith, |&(p1, p2)| (p2, p1));

                self.owl_has_value.from_join(&self.pso, &self.owl_hasvalue_relation, |&_, &tup, &()| tup);
                self.owl_on_property.from_join(&self.pso, &self.owl_onprop_relation, |&_, &tup, &()| tup);
                self.owl_all_values_from.from_join(
                    &self.pso,
                    &self.owl_allvalues_relation,
                    |&_, &tup, &()| tup,
                );
                self.owl_some_values_from.from_join(
                    &self.pso,
                    &self.owl_somevalues_relation,
                    |&_, &tup, &()| tup,
                );
                self.owl_disjoint_with.from_join(
                    &self.pso,
                    &self.owl_disjointwith_relation,
                    |&_, &tup, &()| tup,
                );
                self.owl_same_as
                    .from_join(&self.pso, &self.owl_sameas_relation, |&_, &tup, &()| tup);
                self.owl_complement_of.from_join(
                    &self.pso,
                    &self.owl_complement_relation,
                    |&_, &(a, b), &()| {
                        self.complements.borrow_mut().insert(a, b);
                        self.complements.borrow_mut().insert(b, a);
                        (a, b)
                    },
                );
                self.owl_complement_of.from_join(
                    &self.pso,
                    &self.owl_complement_relation,
                    |&_, &(a, b), &()| (b, a),
                );
                self.owl_equivalent_class.from_join(
                    &self.pso,
                    &self.owl_equivalent_relation,
                    |&_, &(c1, c2), &()| (c1, c2),
                );
                self.owl_equivalent_class.from_join(
                    &self.pso,
                    &self.owl_equivalent_relation,
                    |&_, &(c1, c2), &()| (c2, c1),
                );
                self.symmetric_properties.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_symprop_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                self.transitive_properties.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_transitive_relation,
                    |&_, &inst, &()| (inst, ()),
                );
                self.equivalent_properties.from_join(
                    &self.pso,
                    &self.owl_equivprop_relation,
                    |&_, &(p1, p2), &()| (p1, p2),
                );
                self.equivalent_properties_2.from_join(
                    &self.pso,
                    &self.owl_equivprop_relation,
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
                // Collect irreflexive property violations: T(p rdf:type IrreflexiveProperty), T(x p x)
                let mut prp_irp_violations: Vec<(URI, URI)> = Vec::new();
                self.prp_irp_1.from_join(&self.owl_irreflexive, &self.pso, |&p, &(), &(s, o)| {
                    if s == o && s > 0 {
                        prp_irp_violations.push((p, s));
                    }
                    (p, s)
                });

                // prp-asyp
                //  T(?p, rdf:type, owl:AsymmetricProperty)
                //  T(?x, ?p, ?y)
                //  T(?y, ?p, ?x) => false
                self.prp_asyp_1.from_join(&self.owl_asymmetric, &self.pso, |&p, &(), &(x, y)| {
                    ((x, y, p), ())
                });
                self.prp_asyp_2.from_join(&self.owl_asymmetric, &self.pso, |&p, &(), &(x, y)| {
                    ((y, x, p), ())
                });
                let mut prp_asyp_violations: Vec<(URI, URI, URI)> = Vec::new();
                self.prp_asyp_3.from_join(&self.prp_asyp_1, &self.prp_asyp_2, |&(x, y, p), &(), &()| {
                    if x > 0 && y > 0 && p > 0 {
                        prp_asyp_violations.push((p, x, y));
                    }
                    ((x, y, p), ())
                });

                // prp-fp
                self.prp_fp_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_funcprop_relation,
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
                    &self.owl_invfuncprop_relation,
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
                    &self.rdfs_subprop_relation,
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
                    .from_join(&self.owl_inv2, &self.pso, |&p2, &p1, &(x, y)| (y, (p1, x)));

                // cax-sco
                self.cax_sco_1.from_join(&self.pso, &self.rdfs_subclass_relation, |&_, &(c1, c2), &()| {
                    (c1, c2)
                });
                // ?c1, ?x, rdf:type
                self.cax_sco_2.from_map(&self.rdf_type, |&(inst, class)| (class, inst));

                self.all_triples_input.from_join(
                    &self.cax_sco_1,
                    &self.cax_sco_2,
                    |&class, &parent, &inst| {
                        //println!("instance: {:?} {:?} {:?}", self.to_u(inst), self.to_u(parent), self.to_u(class));
                        (inst, (rdftype_node, parent))
                    },
                );

                // cax-eqc1, cax-eqc2
                // find instances of classes that are equivalent
                self.all_triples_input.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_equivalent_class,
                    |&c1, &inst, &c2| (inst, (rdftype_node, c2)),
                );

                // cax-dw
                self.cax_dw_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_disjoint_with,
                    |&c1, &inst, &c2| (c2, (c1, inst)),
                );
                // Collect disjointness violations without borrowing &mut self inside the closure
                let mut cax_dw_violations: Vec<(URI, URI, URI)> = Vec::new();
                self.cax_dw_2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.cax_dw_1,
                    |&c2, &inst2, &(c1, inst1)| {
                        if inst1 == inst2 && inst1 > 0 {
                            cax_dw_violations.push((inst1, c1, c2));
                        }
                        (c2, inst1)
                    },
                );
                for (inst, c1, c2) in cax_dw_violations.drain(..) {
                    let msg = format!(
                        "inst {} is both {} and {} (disjoint classes)",
                        self.to_u(inst),
                        self.to_u(c1),
                        self.to_u(c2)
                    );
                    self.add_error("cax-dw".to_string(), msg);
                }

                // prp-pdw
                // T(?p1, owl:propertyDisjointWith, ?p2)
                // T(?x, ?p1, ?y)
                // T(?x, ?p2, ?y) => false
                // returns pairs of (x,y) that should NOT exist for p2 because they exist for p1
                self.prp_pdw_1.from_join(&self.owl_propertydisjointwith, &self.pso, |&p1, &p2, &(x, y)| {
                    ((x, y, p2), p1)
                });
                // returns pairs of (x,y) that do have p2
                self.prp_pdw_2.from_join(
                    &self.owl_propertydisjointwith2,
                    &self.pso,
                    |&p2, &p1, &(x, y)| ((x, y, p2), p1),
                );
                // join on (x,y) to find pairs in violation
                let mut prp_pdw_violations: Vec<(URI, URI, URI, URI)> = Vec::new();
                self.prp_pdw_3.from_join(&self.prp_pdw_1, &self.prp_pdw_2, |&(x, y, p2), &p1, &_p1| {
                    if x > 0 && y > 0 && p2 > 0 && p1 > 0 {
                        prp_pdw_violations.push((x, y, p1, p2));
                    }
                    ((x, y, p2), p1)
                });
                for (p, s) in prp_irp_violations.drain(..) {
                    let msg = format!(
                        "property {} of {} is irreflexive",
                        self.to_u(p),
                        self.to_u(s)
                    );
                    self.add_error("prp-irp".to_string(), msg);
                }
                for (p, x, y) in prp_asyp_violations.drain(..) {
                    let msg = format!(
                        "property {} of {} and {} is asymmetric",
                        self.to_u(p),
                        self.to_u(x),
                        self.to_u(y)
                    );
                    self.add_error("prp-asyp".to_string(), msg);
                }
                for (x, y, p1, p2) in prp_pdw_violations.drain(..) {
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

                // prp-symp
                // T(?p, rdf:type, owl:SymmetricProperty)
                // T(?x, ?p, ?y)
                //  => T(?y, ?p, ?x)
                self.all_triples_input.from_join(
                    &self.symmetric_properties,
                    &self.pso,
                    |&prop, &(), &(x, y)| (y, (prop, x)),
                );

                // prp-trp
                // T(?p, rdf:type, owl:TransitiveProperty)
                // T(?x, ?p, ?y)
                // T(?y, ?p, ?z) =>  T(?x, ?p, ?z)
                self.prp_trp_1.from_join(&self.pso, &self.transitive_properties, |&p, &(x, y), &()| {
                    ((y, p), x)
                });
                self.prp_trp_2.from_join(&self.pso, &self.transitive_properties, |&p, &(y, z), &()| {
                    ((y, p), z)
                });
                self.all_triples_input
                    .from_join(&self.prp_trp_1, &self.prp_trp_2, |&(y, p), &x, &z| (x, (p, z)));

                // prp-eqp1
                // T(?p1, owl:equivalentProperty, ?p2)
                // T(?x, ?p1, ?y)
                // => T(?x, ?p2, ?y)
                self.all_triples_input.from_join(
                    &self.equivalent_properties,
                    &self.pso,
                    |&p1, &p2, &(x, y)| (x, (p2, y)),
                );
                // prp-eqp2
                // T(?p1, owl:equivalentProperty, ?p2)
                // T(?x, ?p2, ?y)
                // => T(?x, ?p1, ?y)
                self.all_triples_input.from_join(
                    &self.equivalent_properties_2,
                    &self.pso,
                    |&p1, &p2, &(x, y)| (x, (p2, y)),
                );

                // cls-nothing2
                //  T(?x, rdf:type, owl:Nothing) => false
                let mut cls_nothing2_violations: Vec<URI> = Vec::new();
                self.cls_nothing2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_nothing_relation,
                    |&_nothing, &x, &()| {
                        if x > 0 {
                            cls_nothing2_violations.push(x);
                        }
                        (x, ())
                    },
                );
                for x in cls_nothing2_violations.drain(..) {
                    let msg = format!(
                        "instance {} is owl:Nothing (unsatisfiable)",
                        self.to_u(x)
                    );
                    self.add_error("cls-nothing2".to_string(), msg);
                }

                // cls-int1
                // Optimized implementation:
                // 1. Build a temporary index `class -> [instances]` from `self.instances`.
                // 2. For each intersection list:
                //    a. Select the class in the list with the fewest instances to minimize candidates.
                //    b. Verify if these candidate instances belong to all other classes in the list.
                let mut class_instances: HashMap<URI, Vec<URI>> = HashMap::new();
                for (inst, class) in self.instances.borrow().iter() {
                    class_instances.entry(*class).or_default().push(*inst);
                }

                let mut new_cls_int1_instances = Vec::new();
                for (intersection_class, listname) in self.intersections.borrow().iter() {
                    let listname = *listname;
                    let intersection_class = *intersection_class;
                    if let Some(values) = ds.get_list_values(listname) {
                        if values.is_empty() {
                            continue;
                        }

                        // Optimization: Start with the class having the fewest instances
                        let mut best_class = values[0];
                        let mut min_len = usize::MAX;
                        let mut empty_intersection = false;

                        for &cls in values {
                            match class_instances.get(&cls) {
                                Some(insts) => {
                                    if insts.len() < min_len {
                                        min_len = insts.len();
                                        best_class = cls;
                                    }
                                }
                                None => {
                                    // One of the classes has no instances, so intersection is empty
                                    empty_intersection = true;
                                    break;
                                }
                            }
                        }

                        if empty_intersection {
                            continue;
                        }

                        if let Some(candidates) = class_instances.get(&best_class) {
                            for &inst in candidates {
                                let mut all_match = true;
                                for &cls in values {
                                    if cls == best_class {
                                        continue;
                                    }
                                    if !self.instances.borrow().contains(&(inst, cls)) {
                                        all_match = false;
                                        break;
                                    }
                                }
                                if all_match {
                                    new_cls_int1_instances
                                        .push((inst, (rdftype_node, intersection_class)));
                                }
                            }
                        }
                    }
                }
                self.all_triples_input.extend(new_cls_int1_instances);

                // cls-int2
                let mut new_cls_int2_instances = Vec::new();
                self.cls_int_2_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_intersection_of,
                    |&intersection_class, &inst, &listname| {
                        if let Some(values) = ds.get_list_values(listname) {
                            for &list_class in values {
                                new_cls_int2_instances.push((inst, (rdftype_node, list_class)));
                            }
                        }
                        (inst, new_cls_int2_instances.len() as URI) // Keeping return type consistent, value ignored
                    },
                );
                self.all_triples_input.extend(new_cls_int2_instances);

                // cls-uni  T(?c, owl:unionOf, ?x)
                // LIST[?x, ?c1, ..., ?cn]
                // T(?y, rdf:type, ?ci) (for any i in 1-n) =>  T(?y, rdf:type, ?c)
                // Optimized: Join rdf_type_inv(Class, Inst) with UnionMap(Class, Union)
                let mut union_memberships: Vec<(URI, URI)> = Vec::new();
                for (union_class, listname) in self.unions.borrow().iter() {
                    if let Some(values) = ds.get_list_values(*listname) {
                        for &member_class in values {
                            union_memberships.push((member_class, *union_class));
                        }
                    }
                }
                self.union_mem_var.extend(union_memberships);

                self.all_triples_input.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.union_mem_var,
                    |&_member_class, &inst, &union_class| (inst, (rdftype_node, union_class)),
                );

                // cls-com
                // T(?c1, owl:complementOf, ?c2)
                // T(?x, rdf:type, ?c1)
                // T(?x, rdf:type, ?c2)  => false
                // TODO: how do we infer instances of classes from owl:complementOf?
                self.cls_com_1.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.owl_complement_of,
                    |&c1, &x, &c2| (c2, (x, c1)),
                );
                self.cls_com_2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.cls_com_1,
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
                self.cls_hv1_1.from_join(&self.owl_has_value, &self.owl_on_property, |&x, &y, &p| (x, (p, y)));
                self.all_triples_input.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.cls_hv1_1,
                    |&x, &inst, &(prop, value)| (inst, (prop, value)),
                );

                // cls-hv2:
                // T(?x, owl:hasValue, ?y)
                // T(?x, owl:onProperty, ?p)
                // T(?u, ?p, ?y) =>  T(?u, rdf:type, ?x)
                self.cls_hv2_1.from_join(&self.owl_has_value, &self.owl_on_property, |&x, &y, &p| {
                    // format for pso index; needs property key
                    (p, (y, x))
                });
                self.all_triples_input.from_join(
                    &self.cls_hv2_1,
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
                self.cls_avf_1.from_join(&self.owl_all_values_from, &self.owl_on_property, |&x, &y, &p| {
                    (x, (y, p))
                });
                self.cls_avf_2.from_join(
                    &self.rdf_type_inv.borrow(),
                    &self.cls_avf_1,
                    |&x, &u, &(y, p)| (u, (p, y)),
                );
                self.all_triples_input.from_join(
                    &self.cls_avf_2,
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
                self.cls_svf1_1.from_join(&self.owl_some_values_from, &self.owl_on_property, |&x, &y, &p| {
                    (p, (x, y))
                });
                self.cls_svf1_2.from_join(&self.cls_svf1_1, &self.pso, |&p, &(x, y), &(u, v)| {
                    (v, (x, y, u))
                });
                self.all_triples_input.from_join(
                    &self.cls_svf1_2,
                    &self.rdf_type,
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
                    .from_join(&self.cls_svf1_1, &self.pso, |&p, &(x, y), &(u, v)| {
                        if y == owlthing_node {
                            (u, (rdftype_node, x))
                        } else {
                            (0, (0, 0))
                        }
                    });
            }

            // Now that the inference stage has finished, we will compute the sets of instances for
            // complementary classes
            // OWL 2 RL does not materialize class complements; only inconsistency checks apply.
            // The previous implementation attempted to assert membership in the complement
            // for any individual not known to be in the class, which is unsound under
            // open-world semantics and led to spurious types. We therefore skip generating
            // such triples here and leave only the (optional) inconsistency checks above.
            changed = false;
        }

        // Non-destructive read of stable partitions (preserves Variable state for incremental use)
        let stable = self.spo.stable.borrow();
        let mut output: Vec<KeyedTriple> = Vec::new();
        for batch in stable.iter() {
            output.extend(
                batch.iter()
                    .filter(|(s, (p, o))| *s > 0 && *p > 0 && *o > 0)
            );
        }
        // Build oxrdf output
        let mut out_triples: Vec<Triple> = Vec::with_capacity(output.len());
        for &(_s, (_p, _o)) in output.iter() {
            let (Some(s), Some(p), Some(o)) =
                (self.index.get(_s), self.index.get(_p), self.index.get(_o))
            else {
                error!(
                    "Index lookup failed for triple IDs: ({}, {}, {})",
                    _s, _p, _o
                );
                continue;
            };
            match make_triple(s.clone(), p.clone(), o.clone()) {
                Ok(t) => out_triples.push(t),
                Err(e) => error!("Got error {:?}", e),
            }
        }
        self.output = out_triples;
        // Store materialized triples for incremental use (sorted for dedup)
        output.sort_unstable();
        self.input = output;
    }

    fn to_u(&self, u: URI) -> String {
        match self.index.get(u) {
            Some(t) => t.to_string(),
            None => "<unknown>".to_string(),
        }
    }

    /// Returns the vec of triples currently contained in the Reasoner
    pub fn get_triples(&self) -> Vec<Triple> {
        self.output.clone()
    }

    /// Returns a read-only view of the inferred triples from the last `reason()` run.
    pub fn view_output(&self) -> &[Triple] {
        &self.output
    }

    pub fn get_input(&self) -> Vec<Triple> {
        self.base
            .iter()
            .filter_map(|inst| {
                let (_s, (_p, _o)) = inst;
                let (Some(s), Some(p), Some(o)) =
                    (self.index.get(*_s), self.index.get(*_p), self.index.get(*_o))
                else {
                    error!(
                        "Index lookup failed for base triple IDs: ({}, {}, {})",
                        _s, _p, _o
                    );
                    return None;
                };
                match make_triple(s.clone(), p.clone(), o.clone()) {
                    Ok(t) => Some(t),
                    Err(e) => {
                        error!("Got error {:?}", e);
                        None
                    }
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

/**
Removes from rv the triples that are in src using a linear merge.
Both src and rv must be sorted ascending.
On return, rv contains only elements not present in src.
*/
pub fn get_unique(src: &[KeyedTriple], rv: &mut Vec<KeyedTriple>) {
    let n = src.len();
    let m = rv.len();
    if n == 0 || m == 0 {
        return;
    }
    let mut i = 0usize; // index into src
    let mut j = 0usize; // index into rv
    let mut out = Vec::with_capacity(m);
    while j < m {
        let b = rv[j];
        // Advance src until src[i] >= b
        while i < n && src[i] < b {
            i += 1;
        }
        if i == n || src[i] != b {
            out.push(b);
        }
        j += 1;
    }
    *rv = out;
}
