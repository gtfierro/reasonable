use std::collections::{HashMap, HashSet};
use std::hash::Hash;

use datafrog::{Relation, Variable};
use oxrdf::{NamedNode, Term};

use crate::common::{KeyedTriple, URI};
use crate::index::URIIndex;

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, Ord, PartialOrd)]
pub(crate) enum ListViolation {
    AllDifferent {
        rule: &'static str,
        axiom: URI,
        left: URI,
        right: URI,
    },
    DisjointClasses {
        axiom: URI,
        instance: URI,
        left: URI,
        right: URI,
    },
    DisjointProperties {
        axiom: URI,
        subject: URI,
        object: URI,
        left: URI,
        right: URI,
    },
}

#[derive(Default)]
struct RdfLists {
    first: HashMap<URI, HashSet<URI>>,
    rest: HashMap<URI, HashSet<URI>>,
    predecessors: HashMap<URI, HashSet<URI>>,
    terminating: HashSet<URI>,
}

impl RdfLists {
    fn extend_terminating(&mut self, mut changed_cells: Vec<URI>) {
        // A cell forms a complete list if it has a value and a tail that
        // already reaches rdf:nil. Work backward so incomplete cycles stay out.
        while let Some(cell) = changed_cells.pop() {
            if self.first.contains_key(&cell)
                && self
                    .rest
                    .get(&cell)
                    .is_some_and(|tails| tails.iter().any(|tail| self.terminating.contains(tail)))
                && self.terminating.insert(cell)
            {
                changed_cells.extend(self.predecessors.get(&cell).into_iter().flatten().copied());
            }
        }
    }

    fn reachable(&self, heads: impl IntoIterator<Item = URI>) -> HashSet<URI> {
        let mut pending: Vec<_> = heads.into_iter().collect();
        let mut cells = HashSet::new();
        while let Some(cell) = pending.pop() {
            if self.terminating.contains(&cell) && cells.insert(cell) {
                pending.extend(self.rest.get(&cell).into_iter().flatten().copied());
            }
        }
        cells
    }

    fn members(&self, head: URI) -> ListMembers<'_> {
        let mut positions: HashMap<URI, Vec<URI>> = HashMap::new();
        let mut linear = true;
        for cell in self.reachable([head]) {
            if let Some(values) = self.first.get(&cell) {
                for &value in values {
                    positions.entry(value).or_default().push(cell);
                }
                linear &= self.rest[&cell]
                    .iter()
                    .filter(|tail| self.terminating.contains(tail))
                    .count()
                    == 1;
            }
        }
        ListMembers {
            lists: self,
            positions,
            linear,
        }
    }
}

struct ListMembers<'a> {
    lists: &'a RdfLists,
    positions: HashMap<URI, Vec<URI>>,
    linear: bool,
}

impl ListMembers<'_> {
    fn cooccur(&self, left: URI, right: URI) -> bool {
        let Some(right_cells) = self.positions.get(&right) else {
            return false;
        };
        let left_cells = &self.positions[&left];
        if left == right && left_cells.len() == 1 {
            return false;
        }
        if self.linear {
            return left_cells.len() > 1 || right_cells.len() > 1 || left_cells != right_cells;
        }
        // Alternatives in a branched list count together only when two
        // distinct cells lie on the same terminating path.
        self.follows(left_cells, right_cells) || self.follows(right_cells, left_cells)
    }

    fn follows(&self, before: &[URI], after: &[URI]) -> bool {
        before.iter().any(|cell| {
            let suffix = self.lists.reachable(self.lists.rest[cell].iter().copied());
            after
                .iter()
                .any(|later| later != cell && suffix.contains(later))
        })
    }

    fn check_disjoint<V: Copy + Ord + Hash>(
        &self,
        index: &[Relation<(URI, V)>],
        mut report: impl FnMut(V, URI, URI),
    ) {
        // Only compare members that share an instance or an assertion.
        let mut members_by_witness: HashMap<V, Vec<URI>> = HashMap::new();
        for &member in self.positions.keys() {
            for batch in index {
                for &(_, witness) in with_key(batch, member) {
                    members_by_witness.entry(witness).or_default().push(member);
                }
            }
        }
        for (witness, mut members) in members_by_witness {
            members.sort_unstable();
            for (i, &left) in members.iter().enumerate() {
                for &right in &members[i..] {
                    if self.cooccur(left, right) {
                        report(witness, left, right);
                    }
                }
            }
        }
    }

    fn check_all_different(
        &self,
        same_as: &[Relation<(URI, URI)>],
        mut report: impl FnMut(URI, URI),
    ) {
        for &left in self.positions.keys() {
            if self.cooccur(left, left) {
                report(left, left);
            }
            for batch in same_as {
                for &(_, right) in with_key(batch, left) {
                    if left < right && self.cooccur(left, right) {
                        report(left, right);
                    }
                }
            }
        }
    }
}

pub(crate) struct ListRules {
    rdf_type: URI,
    rdf_first: URI,
    rdf_rest: URI,
    rdf_nil: URI,
    one_of: URI,
    members: URI,
    distinct_members: URI,
    all_different: URI,
    all_disjoint_classes: URI,
    all_disjoint_properties: URI,
    lists: RdfLists,
    enumerations: HashSet<(URI, URI)>,
    groups: HashSet<(URI, URI)>,
    member_heads: HashMap<URI, HashSet<URI>>,
    distinct_member_heads: HashMap<URI, HashSet<URI>>,
    reported: HashSet<ListViolation>,
}

impl ListRules {
    pub(crate) fn new(
        index: &mut URIIndex,
        rdf_type: URI,
        rdf_first: URI,
        rdf_rest: URI,
        rdf_nil: URI,
    ) -> Self {
        Self {
            rdf_type,
            rdf_first,
            rdf_rest,
            rdf_nil,
            one_of: index.put(owl!("oneOf")),
            members: index.put(owl!("members")),
            distinct_members: index.put(owl!("distinctMembers")),
            all_different: index.put(owl!("AllDifferent")),
            all_disjoint_classes: index.put(owl!("AllDisjointClasses")),
            all_disjoint_properties: index.put(owl!("AllDisjointProperties")),
            lists: RdfLists {
                terminating: HashSet::from([rdf_nil]),
                ..RdfLists::default()
            },
            enumerations: HashSet::new(),
            groups: HashSet::new(),
            member_heads: HashMap::new(),
            distinct_member_heads: HashMap::new(),
            reported: HashSet::new(),
        }
    }

    pub(crate) fn clear(&mut self) {
        self.lists = RdfLists {
            terminating: HashSet::from([self.rdf_nil]),
            ..RdfLists::default()
        };
        self.enumerations.clear();
        self.groups.clear();
        self.member_heads.clear();
        self.distinct_member_heads.clear();
        self.reported.clear();
    }

    pub(crate) fn update(
        &mut self,
        by_predicate: &[KeyedTriple],
        by_class: &[(URI, URI)],
    ) -> Vec<KeyedTriple> {
        let mut changed_cells = Vec::new();
        for &(_, (cell, value)) in with_key(by_predicate, self.rdf_first) {
            if cell != self.rdf_nil {
                self.lists.first.entry(cell).or_default().insert(value);
                changed_cells.push(cell);
            }
        }
        for &(_, (cell, tail)) in with_key(by_predicate, self.rdf_rest) {
            if cell != self.rdf_nil {
                self.lists.rest.entry(cell).or_default().insert(tail);
                self.lists
                    .predecessors
                    .entry(tail)
                    .or_default()
                    .insert(cell);
                changed_cells.push(cell);
            }
        }
        let previous_count = self.enumerations.len();
        self.enumerations.extend(
            with_key(by_predicate, self.one_of)
                .iter()
                .map(|&(_, enumeration)| enumeration),
        );
        for kind in [
            self.all_different,
            self.all_disjoint_classes,
            self.all_disjoint_properties,
        ] {
            self.groups.extend(
                with_key(by_class, kind)
                    .iter()
                    .map(|&(_, axiom)| (axiom, kind)),
            );
        }
        for &(_, (axiom, head)) in with_key(by_predicate, self.members) {
            self.member_heads.entry(axiom).or_default().insert(head);
        }
        for &(_, (axiom, head)) in with_key(by_predicate, self.distinct_members) {
            self.distinct_member_heads
                .entry(axiom)
                .or_default()
                .insert(head);
        }

        let mut inferred = Vec::new();
        if self.enumerations.len() != previous_count || !changed_cells.is_empty() {
            self.lists.extend_terminating(changed_cells);
            for &(class, head) in &self.enumerations {
                for cell in self.lists.reachable([head]) {
                    for &member in self.lists.first.get(&cell).into_iter().flatten() {
                        inferred.push((member, (self.rdf_type, class)));
                    }
                }
            }
        }
        inferred
    }

    pub(crate) fn violations(
        &mut self,
        by_predicate: &Variable<KeyedTriple>,
        by_class: &Variable<(URI, URI)>,
        same_as: &Variable<(URI, URI)>,
    ) -> Vec<ListViolation> {
        let mut violations = Vec::new();
        let mut report = |violation| {
            if self.reported.insert(violation) {
                violations.push(violation);
            }
        };
        let by_predicate = by_predicate.stable.borrow();
        let by_class = by_class.stable.borrow();
        let same_as = same_as.stable.borrow();
        for &(axiom, kind) in &self.groups {
            let member_heads = self.member_heads.get(&axiom).into_iter().flatten();
            if kind == self.all_different {
                let distinct_member_heads =
                    self.distinct_member_heads.get(&axiom).into_iter().flatten();
                let heads = member_heads
                    .map(|&head| (head, "eq-diff2"))
                    .chain(distinct_member_heads.map(|&head| (head, "eq-diff3")));
                for (head, rule) in heads {
                    self.lists
                        .members(head)
                        .check_all_different(&same_as, |left, right| {
                            report(ListViolation::AllDifferent {
                                rule,
                                axiom,
                                left,
                                right,
                            });
                        });
                }
            } else if kind == self.all_disjoint_classes {
                for &head in member_heads {
                    self.lists
                        .members(head)
                        .check_disjoint(&by_class, |instance, left, right| {
                            report(ListViolation::DisjointClasses {
                                axiom,
                                instance,
                                left,
                                right,
                            });
                        });
                }
            } else {
                for &head in member_heads {
                    self.lists.members(head).check_disjoint(
                        &by_predicate,
                        |(subject, object), left, right| {
                            report(ListViolation::DisjointProperties {
                                axiom,
                                subject,
                                object,
                                left,
                                right,
                            });
                        },
                    );
                }
            }
        }
        violations.sort_unstable();
        violations
    }
}

fn with_key<V>(sorted: &[(URI, V)], key: URI) -> &[(URI, V)] {
    let start = sorted.partition_point(|(k, _)| *k < key);
    let end = start + sorted[start..].partition_point(|(k, _)| *k == key);
    &sorted[start..end]
}
