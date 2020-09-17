// use serde_lexpr::error;
// use serde_lexpr::{from_str, to_string};
use regex::Regex;
use serde::de::DeserializeOwned;
use serde_sexpr;
// use serde_sexpr::{from_str, to_string, Error};
use serde_derive::{Serialize, Deserialize};
//use lexpr::value::Value;
//

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum Atom {
    iri(String),
    var(String)
}
//TODO: to_atom constructor from query.rs

type Triple = (Atom, PropertyPathExpression, Atom);

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
#[allow(non_camel_case_types)]
enum PropertyPathExpression {
    link(Atom),
    inv(Box<PropertyPathExpression>),
    seq(Box<PropertyPathExpression>, Box<PropertyPathExpression>),
    alt(Box<PropertyPathExpression>, Box<PropertyPathExpression>),
    ZeroOrMorePath(Box<PropertyPathExpression>),
    OneOrMorePath(Box<PropertyPathExpression>),
    ZeroOrOnePath(Box<PropertyPathExpression>)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
enum GraphPattern {
    union(Box<GraphPattern>, Box<GraphPattern>),
    optional(Box<GraphPattern>),
    bgp(BasicGraphPattern)
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
struct Filter {
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "kebab-case")]
struct BasicGraphPattern {
    filter: Option<Filter>,
    triples: Vec<Triple>
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
struct Query {
    select(GraphPattern),
    count(GraphPattern),
}

pub fn try_it() {
    let bgp = BasicGraphPattern {
        filter: None,
        triples: vec![
            (Atom::var("?ahu".to_string()), PropertyPathExpression::OneOrMorePath(Box::new(PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())))), Atom::var("?ds".to_string())),
            (Atom::var("?ds".to_string()), PropertyPathExpression::link(Atom::iri("rdf:type".to_string())), Atom::iri("brick:Equipment".to_string())),
        ]
    };
}

pub fn parse<T: DeserializeOwned>(q: &str) -> serde_sexpr::Result<T> {
    let re = Regex::new(r"([\t\n\r]+)|(  +)").unwrap();
    serde_sexpr::from_str(&re.replace_all(q, ""))
}

#[cfg(test)]
mod algebra_tests {
    use super::*;

    #[test]
    fn test_algebra() -> Result<(), serde_sexpr::Error> {
        let q = "(var ?ahu)";
        let parsed: Atom = parse(q)?;
        assert_eq!(Atom::var("?ahu".to_string()), parsed);

        let q = "(iri brick:VAV)";
        let parsed: Atom = parse(q)?;
        assert_eq!(Atom::iri("brick:VAV".to_string()), parsed);

        let q = "(link (iri brick:feeds))";
        let parsed: PropertyPathExpression = parse(q)?;
        assert_eq!(PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())), parsed);

        let q = "(zero-or-more-path (link (iri brick:feeds)))";
        let parsed: PropertyPathExpression = parse(q)?;
        assert_eq!(PropertyPathExpression::ZeroOrMorePath(Box::new(PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())))), parsed);

        let q = r#"((var ?ahu) (link (iri brick:feeds)) (var ?equip))"#;
        let parsed: Triple = parse(q)?;
        assert_eq!((Atom::var("?ahu".to_string()), PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())), Atom::var("?equip".to_string())), parsed);

        let q = r#"(
            ((var ?ahu) (link (iri brick:feeds)) (var ?equip))
        )"#;
        let parsed: Vec<Triple> = parse(q)?;
        assert_eq!(vec![(Atom::var("?ahu".to_string()), PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())), Atom::var("?equip".to_string()))], parsed);

        let q = r#"((triples (
            ((var ?ahu) (link (iri brick:feeds)) (var ?equip))
        )))"#;
        let parsed: BasicGraphPattern = parse(q)?;
        let expected = BasicGraphPattern {
            filter: None,
            triples: vec![(Atom::var("?ahu".to_string()), PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())), Atom::var("?equip".to_string()))] 
        };
        assert_eq!(expected, parsed);

        let q = r#"(bgp (
            (triples (
                ((var ?ahu) (link (iri brick:feeds)) (var ?equip))
            ))
        ))"#;
        let parsed: GraphPattern = parse(q)?;
        let expected = GraphPattern::bgp(BasicGraphPattern {
            filter: None,
            triples: vec![(Atom::var("?ahu".to_string()), PropertyPathExpression::link(Atom::iri("brick:feeds".to_string())), Atom::var("?equip".to_string()))] 
        });
        assert_eq!(expected, parsed);
        Ok(())
    }
}
