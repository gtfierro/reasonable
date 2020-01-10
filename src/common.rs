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

pub type URI = u32;
pub type Triple = (URI, (URI, URI));

pub const _NONE : (URI, ())= (0, ());
pub const _NONE_TUP : (URI, URI) = (0,0);
pub const _NONE_TRIP : (URI, (URI, URI)) = (0,(0,0));

pub fn has_pred(triple: (URI, (URI, URI)), pred: URI) -> (URI, URI) {
    let (s, (p, o)) = triple;
    if p == pred {
        (s, o)
    } else {
        _NONE_TUP
    }
}

pub fn has_obj(triple: (URI, (URI, URI)), obj: URI) -> (URI, URI) {
    let (s, (p, o)) = triple;
    if o == obj {
        (s, p)
    } else {
        _NONE_TUP
    }
}

pub fn has_pred_obj(triple: (URI, (URI, URI)), predobj: (URI, URI)) -> (URI, ()) {
    let (s, (p, o)) = triple;
    let (pred, obj) = predobj;
    if p == pred && o == obj{
        (s, ())
    } else {
        _NONE
    }
}
