use oxigraph::model::{Term, Triple, NamedOrBlankNode};
use crate::error::ReasonableError;

pub type URI = u32;
pub type KeyedTriple = (URI, (URI, URI));

#[macro_export]
macro_rules! uri {
    ($ns:expr, $t:expr) => {
        Term::NamedNode(NamedNode::new(format!($ns, $t)).unwrap())
    };
}


/// Returns the full URI of the concept in the OWL namespace
/// ```
/// # #[macro_use] extern crate reasonable; extern crate oxigraph; use oxigraph::model::{Term, NamedNode}; fn main() {
/// let uri = owl!("Thing");
/// assert_eq!(uri, Term::NamedNode(NamedNode::new_unchecked("http://www.w3.org/2002/07/owl#Thing".to_string())));
/// # }
/// ```
#[macro_export]
macro_rules! owl {
    ($t:expr) => {
        uri!("http://www.w3.org/2002/07/owl#{}", $t)
    };
}

/// Returns the full URI of the concept in the RDF namespace
/// ```
/// # #[macro_use] extern crate reasonable; extern crate oxigraph; use oxigraph::model::{Term, NamedNode}; fn main() {
/// let uri = rdf!("type");
/// assert_eq!(uri, Term::NamedNode(NamedNode::new_unchecked("http://www.w3.org/1999/02/22-rdf-syntax-ns#type".to_string())));
/// # }
/// ```
#[macro_export]
macro_rules! rdf {
    ($t:expr) => {
        uri!("http://www.w3.org/1999/02/22-rdf-syntax-ns#{}", $t)
    };
}

/// Returns the full URI of the concept in the RDFS namespace
/// ```
/// # #[macro_use] extern crate reasonable; extern crate oxigraph; use oxigraph::model::{Term, NamedNode}; fn main() {
/// let uri = rdfs!("subClassOf");
/// assert_eq!(uri, Term::NamedNode(NamedNode::new_unchecked("http://www.w3.org/2000/01/rdf-schema#subClassOf".to_string())));
/// # }
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


pub fn make_triple(s: Term, p: Term, o: Term) -> Result<Triple, ReasonableError> {
    let s = match s {
        Term::NamedNode(n) => NamedOrBlankNode::NamedNode(n),
        Term::BlankNode(b) => NamedOrBlankNode::BlankNode(b),
        _ => return Err(ReasonableError::ManagerError("Cannot have literal as subject".to_string()))
    };
    let p = match p {
        Term::NamedNode(n) => n,
        _ => return Err(ReasonableError::ManagerError("Must have named node as predicate".to_string()))
    };
    Ok(Triple::new(s,p,o))
}
