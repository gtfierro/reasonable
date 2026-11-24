use hashbrown::{HashMap, HashSet};
use std::hash::Hash;

use datafrog::{Iteration, Relation, Variable};
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
                    .filter(|&tail| self.terminating.contains(tail))
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

struct IntersectionVars {
    /// List head -> intersection class.
    heads: Variable<(URI, URI)>,
    /// Intersection class -> member class (cls-int2).
    members: Variable<(URI, URI)>,
    /// Member class -> final cell.
    last: Variable<(URI, URI)>,
    /// Tail -> (cell, member class).
    links: Variable<(URI, (URI, URI))>,
    /// (Instance, member class) -> cell, where the instance matches the cell's tail.
    candidates: Variable<((URI, URI), URI)>,
    /// (Cell, instance) for each complete list suffix the instance matches.
    matches: Variable<((URI, URI), ())>,
    /// Cell -> instance for emitting cls-int1.
    matches_by_head: Variable<(URI, URI)>,
}

impl IntersectionVars {
    fn new(iter: &mut Iteration) -> Self {
        Self {
            heads: iter.variable("intersection_heads"),
            members: iter.variable("intersection_members"),
            last: iter.variable("intersection_last"),
            links: iter.variable("intersection_links"),
            candidates: iter.variable("intersection_candidates"),
            matches: iter.variable("intersection_matches"),
            matches_by_head: iter.variable("intersection_matches_by_head"),
        }
    }

    fn infer(
        &self,
        by_class: &Variable<(URI, URI)>,
        type_pairs: &Variable<((URI, URI), ())>,
        rdf_type: URI,
        triples: &Variable<KeyedTriple>,
    ) {
        // cls-int1: the member of the final cell must match, then walk
        // backward one cell at a time. Each match follows one complete path,
        // even when a list cell has alternative first or rest triples.
        //
        // Only instances that already match a suffix of the list are carried
        // back to the cell before it and checked against that cell's member,
        // so the work follows the instances that can still match rather than
        // every instance of every member class.
        self.matches
            .from_join(&self.last, by_class, |&_, &cell, &instance| {
                ((cell, instance), ())
            });
        self.matches_by_head
            .from_map(&self.matches, |&((cell, instance), ())| (cell, instance));
        self.candidates.from_join(
            &self.links,
            &self.matches_by_head,
            |&_, &(cell, member), &instance| ((instance, member), cell),
        );
        self.matches.from_join(
            &self.candidates,
            type_pairs,
            |&(instance, _), &cell, &()| ((cell, instance), ()),
        );
        triples.from_join(
            &self.heads,
            &self.matches_by_head,
            |&_, &class, &instance| (instance, (rdf_type, class)),
        );

        // cls-int2: instance types propagate to every member of a complete list.
        triples.from_join(&self.members, by_class, |&_, &member, &instance| {
            (instance, (rdf_type, member))
        });
    }
}

pub(crate) struct ListRules {
    rdf_type: URI,
    rdf_first: URI,
    rdf_rest: URI,
    rdf_nil: URI,
    one_of: URI,
    intersection_of: URI,
    union_of: URI,
    members: URI,
    distinct_members: URI,
    all_different: URI,
    all_disjoint_classes: URI,
    all_disjoint_properties: URI,
    lists: RdfLists,
    enumerations: HashSet<(URI, URI)>,
    intersection_heads: HashSet<(URI, URI)>,
    union_heads: HashSet<(URI, URI)>,
    intersection: IntersectionVars,
    union_members: Variable<(URI, URI)>,
    groups: HashSet<(URI, URI)>,
    member_heads: HashMap<URI, HashSet<URI>>,
    distinct_member_heads: HashMap<URI, HashSet<URI>>,
    reported: HashSet<ListViolation>,
}

impl ListRules {
    pub(crate) fn new(
        index: &mut URIIndex,
        iter: &mut Iteration,
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
            intersection_of: index.put(owl!("intersectionOf")),
            union_of: index.put(owl!("unionOf")),
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
            intersection_heads: HashSet::new(),
            union_heads: HashSet::new(),
            intersection: IntersectionVars::new(iter),
            union_members: iter.variable("union_members"),
            groups: HashSet::new(),
            member_heads: HashMap::new(),
            distinct_member_heads: HashMap::new(),
            reported: HashSet::new(),
        }
    }

    pub(crate) fn clear(&mut self, iter: &mut Iteration) {
        self.lists = RdfLists {
            terminating: HashSet::from([self.rdf_nil]),
            ..RdfLists::default()
        };
        self.enumerations.clear();
        self.intersection_heads.clear();
        self.union_heads.clear();
        self.intersection = IntersectionVars::new(iter);
        self.union_members = iter.variable("union_members");
        self.groups.clear();
        self.member_heads.clear();
        self.distinct_member_heads.clear();
        self.reported.clear();
    }

    /// `type_pairs` holds each type assertion as ((instance, class), ()).
    pub(crate) fn update(
        &mut self,
        by_predicate: &[KeyedTriple],
        by_class: &Variable<(URI, URI)>,
        type_pairs: &Variable<((URI, URI), ())>,
        triples: &Variable<KeyedTriple>,
    ) {
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
        let mut class_lists_changed = !changed_cells.is_empty();
        for (predicate, axioms) in [
            (self.one_of, &mut self.enumerations),
            (self.intersection_of, &mut self.intersection_heads),
            (self.union_of, &mut self.union_heads),
        ] {
            let previous_count = axioms.len();
            axioms.extend(
                with_key(by_predicate, predicate)
                    .iter()
                    .map(|&(_, axiom)| axiom),
            );
            class_lists_changed |= axioms.len() != previous_count;
        }
        let recent_types = by_class.recent.borrow();
        for kind in [
            self.all_different,
            self.all_disjoint_classes,
            self.all_disjoint_properties,
        ] {
            self.groups.extend(
                with_key(&recent_types, kind)
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

        if class_lists_changed {
            self.lists.extend_terminating(changed_cells);
            self.update_class_lists(triples);
        }

        self.intersection
            .infer(by_class, type_pairs, self.rdf_type, triples);
        // cls-uni: membership in any member class entails membership in the union.
        triples.from_join(&self.union_members, by_class, |&_, &class, &instance| {
            (instance, (self.rdf_type, class))
        });
    }

    fn update_class_lists(&self, triples: &Variable<KeyedTriple>) {
        let values = |head| {
            self.lists
                .reachable([head])
                .into_iter()
                .flat_map(|cell| self.lists.first.get(&cell).into_iter().flatten().copied())
        };

        // cls-oo
        triples.extend(self.enumerations.iter().flat_map(|&(class, head)| {
            values(head).map(move |member| (member, (self.rdf_type, class)))
        }));
        self.union_members.extend(
            self.union_heads
                .iter()
                .flat_map(|&(class, head)| values(head).map(move |member| (member, class))),
        );
        self.intersection.heads.extend(
            self.intersection_heads
                .iter()
                .map(|&(class, head)| (head, class)),
        );

        let mut intersection_members = Vec::new();
        let mut cells = HashSet::new();
        for &(class, head) in &self.intersection_heads {
            for cell in self.lists.reachable([head]) {
                cells.insert(cell);
                intersection_members.extend(
                    self.lists
                        .first
                        .get(&cell)
                        .into_iter()
                        .flatten()
                        .map(|&member| (class, member)),
                );
            }
        }
        self.intersection.members.extend(intersection_members);

        let mut last = Vec::new();
        let mut links = Vec::new();
        for cell in cells {
            let Some(members) = self.lists.first.get(&cell) else {
                continue; // rdf:nil
            };
            for &tail in &self.lists.rest[&cell] {
                if tail == self.rdf_nil {
                    last.extend(members.iter().map(|&member| (member, cell)));
                } else if self.lists.terminating.contains(&tail) {
                    links.extend(members.iter().map(|&member| (tail, (cell, member))));
                }
            }
        }
        self.intersection.last.extend(last);
        self.intersection.links.extend(links);
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
