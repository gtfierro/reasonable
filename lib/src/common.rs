use oxrdf::{
    BlankNode, Literal, NamedNode, NamedOrBlankNode, Subject, SubjectRef, Term, TermRef, Triple,
    TripleRef,
};
mod rio {
    pub use rio_api::model::*;
}

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
/// # #[macro_use] extern crate reasonable; extern crate oxrdf; use oxrdf::{Term, NamedNode}; fn main() {
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
/// # #[macro_use] extern crate reasonable; extern crate oxrdf; use oxrdf::{Term, NamedNode}; fn main() {
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
/// # #[macro_use] extern crate reasonable; extern crate oxrdf; use oxrdf::{Term, NamedNode}; fn main() {
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
        _ => {
            return Err(ReasonableError::ManagerError(
                "Cannot have literal as subject".to_string(),
            ))
        }
    };
    let p = match p {
        Term::NamedNode(n) => n,
        _ => {
            return Err(ReasonableError::ManagerError(
                "Must have named node as predicate".to_string(),
            ))
        }
    };
    Ok(Triple::new(s, p, o))
}

pub fn rio_to_oxrdf(t: rio::Triple) -> Triple {
    let s: Subject = match t.subject {
        rio::Subject::NamedNode(rio::NamedNode { iri }) => {
            Subject::NamedNode(NamedNode::new_unchecked(iri))
        }
        rio::Subject::BlankNode(rio::BlankNode { id }) => {
            Subject::BlankNode(BlankNode::new_unchecked(id))
        }
        _ => panic!("no rdf*"),
    };
    let rio::NamedNode { iri } = t.predicate;
    let p: NamedNode = NamedNode::new_unchecked(iri);
    let o = match t.object {
        rio::Term::NamedNode(rio::NamedNode { iri }) => {
            Term::NamedNode(NamedNode::new_unchecked(iri))
        }
        rio::Term::BlankNode(rio::BlankNode { id }) => {
            Term::BlankNode(BlankNode::new_unchecked(id))
        }
        rio::Term::Literal(lit) => match lit {
            rio::Literal::Simple { value } => Term::Literal(Literal::new_simple_literal(value)),
            rio::Literal::LanguageTaggedString { value, language } => Term::Literal(
                Literal::new_language_tagged_literal_unchecked(value, language),
            ),
            rio::Literal::Typed { value, datatype } => {
                let rio::NamedNode { iri } = datatype;
                Term::Literal(Literal::new_typed_literal(
                    value,
                    NamedNode::new_unchecked(iri),
                ))
            }
        },
        _ => panic!("no rdf*"),
    };
    Triple::new(s, p, o)
}

pub fn oxrdf_to_rio<'a>(t: TripleRef<'a>) -> rio::Triple<'a> {
    let s: rio::Subject = match t.subject {
        SubjectRef::NamedNode(nn) => rio::Subject::NamedNode(rio::NamedNode { iri: nn.as_str() }),
        SubjectRef::BlankNode(bn) => rio::Subject::BlankNode(rio::BlankNode { id: bn.as_str() }),
    };
    let p = rio::NamedNode {
        iri: t.predicate.as_str(),
    };
    let o: rio::Term = match t.object {
        TermRef::NamedNode(nn) => rio::Term::NamedNode(rio::NamedNode { iri: nn.as_str() }),
        TermRef::BlankNode(bn) => rio::Term::BlankNode(rio::BlankNode { id: bn.as_str() }),
        TermRef::Literal(lit) => {
            let value = lit.value();
            let datatype = lit.datatype();
            let language = lit.language();
            if datatype.as_str() != "http://www.w3.org/2001/XMLSchema#string" {
                rio::Term::Literal(rio::Literal::Typed {
                    value,
                    datatype: rio::NamedNode {
                        iri: datatype.as_str(),
                    },
                })
            } else {
                match language {
                    None => rio::Term::Literal(rio::Literal::Simple { value }),
                    Some(l) => rio::Term::Literal(rio::Literal::LanguageTaggedString {
                        value,
                        language: l,
                    }),
                }
            }
        }
    };
    rio::Triple {
        subject: s,
        predicate: p,
        object: o,
    }
}
