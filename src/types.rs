pub type URI = u64;

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

