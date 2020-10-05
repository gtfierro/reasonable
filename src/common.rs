pub const RDFS_SUBCLASSOF: &str = "http://www.w3.org/2000/01/rdf-schema#subClassOf";
pub const RDFS_DOMAIN: &str = "http://www.w3.org/2000/01/rdf-schema#domain";
pub const RDFS_RANGE: &str = "http://www.w3.org/2000/01/rdf-schema#range";
pub const RDFS_LITERAL: &str = "http://www.w3.org/2000/01/rdf-schema#Literal";
pub const RDFS_RESOURCE: &str = "http://www.w3.org/2000/01/rdf-schema#Resource";
pub const RDFS_SUBPROP: &str = "http://www.w3.org/2000/01/rdf-schema#subPropertyOf";
pub const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
pub const RDF_FIRST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#first";
pub const RDF_REST: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#rest";
pub const RDF_NIL: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#nil";
pub const OWL_SAMEAS: &str = "http://www.w3.org/2002/07/owl#sameAs";
pub const OWL_EQUIVALENTCLASS: &str = "http://www.w3.org/2002/07/owl#equivalentClass";
pub const OWL_HASVALUE: &str = "http://www.w3.org/2002/07/owl#hasValue";
pub const OWL_ALLVALUESFROM: &str = "http://www.w3.org/2002/07/owl#allValuesFrom";
pub const OWL_SOMEVALUESFROM: &str = "http://www.w3.org/2002/07/owl#someValuesFrom";
pub const OWL_ONPROPERTY: &str = "http://www.w3.org/2002/07/owl#onProperty";
pub const OWL_INVERSEOF: &str = "http://www.w3.org/2002/07/owl#inverseOf";
pub const OWL_SYMMETRICPROP: &str = "http://www.w3.org/2002/07/owl#SymmetricProperty";
pub const OWL_EQUIVPROP: &str = "http://www.w3.org/2002/07/owl#equivalentProperty";
pub const OWL_FUNCPROP: &str = "http://www.w3.org/2002/07/owl#FunctionalProperty";
pub const OWL_INVFUNCPROP: &str = "http://www.w3.org/2002/07/owl#InverseFunctionalProperty";
pub const OWL_TRANSPROP: &str = "http://www.w3.org/2002/07/owl#TransitiveProperty";
pub const OWL_INTERSECTION: &str = "http://www.w3.org/2002/07/owl#intersectionOf";
pub const OWL_UNION: &str = "http://www.w3.org/2002/07/owl#unionOf";
pub const OWL_CLASS: &str = "http://www.w3.org/2002/07/owl#Class";
pub const OWL_THING: &str = "http://www.w3.org/2002/07/owl#Thing";
pub const OWL_NOTHING: &str = "http://www.w3.org/2002/07/owl#Nothing";
pub const OWL_COMPLEMENT: &str = "http://www.w3.org/2002/07/owl#complementOf";
pub const OWL_RESTRICTION: &str = "http://www.w3.org/2002/07/owl#Restriction";
pub const OWL_ASYMMETRICPROP: &str = "http://www.w3.org/2002/07/owl#AsymmetricProperty";

pub type URI = u32;
pub type Triple = (URI, (URI, URI));

#[macro_export]
macro_rules! uri {
    ($ns:expr, $t:expr) => {
        Node::UriNode {
            uri: Uri::new(format!($ns, $t)),
        }
    };
}

/// Returns the full URI of the concept in the OWL namespace
/// ```
/// let uri = owl!("Thing");
/// println!(uri);
/// ```
#[macro_export]
macro_rules! owl {
    ($t:expr) => {
        uri!("http://www.w3.org/2002/07/owl#{}", $t)
    };
}

/// Returns the full URI of the concept in the RDF namespace
/// ```
/// let uri = rdf!("type");
/// println!(uri);
/// ```
#[macro_export]
macro_rules! rdf {
    ($t:expr) => {
        uri!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{}", $t)
    };
}

/// Returns the full URI of the concept in the RDFS namespace
/// ```
/// let uri = rdfs!("type");
/// println!(uri);
/// ```
#[macro_export]
macro_rules! rdfs {
    ($t:expr) => {
        uri!("http://www.w3.org/2000/01/rdf-schema#{}", $t)
    };
}

/// Creates a DataFrog variable with the given URI as the only member
#[macro_export]
macro_rules! node_relation {
    ($self:expr, $uri:expr) => {{
        let x = $self.iter1.variable::<(URI, ())>("tmp");
        let v = vec![($self.index.put($uri), ())];
        x.extend(v.iter());
        x
    }};
}
