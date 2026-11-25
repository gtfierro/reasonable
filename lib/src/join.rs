//! Joins over datafrog inputs for rules that only want some of the joined pairs.
//!
//! `Variable::from_join` materialises one output tuple per joined pair, so a rule
//! that only cares about some pairs (or only reports violations) must emit filler
//! tuples that are then sorted and stored. These helpers visit the same pairs
//! without storing anything the rule did not ask for.

use datafrog::{JoinInput, Relation, Variable};

/// Calls `logic` on the pairs of tuples, one from each input, that share a key and
/// that `Variable::from_join` would join in this round (at least one is recent).
pub(crate) fn join_each<'me, K: Ord, V1: Ord, V2: Ord>(
    input1: impl JoinInput<'me, (K, V1)>,
    input2: impl JoinInput<'me, (K, V2)>,
    mut logic: impl FnMut(&K, &V1, &V2),
) {
    let recent1 = input1.recent();
    let recent2 = input2.recent();
    for batch2 in input2.stable().iter() {
        join_helper(&recent1, batch2, &mut logic);
    }
    for batch1 in input1.stable().iter() {
        join_helper(batch1, &recent2, &mut logic);
    }
    join_helper(&recent1, &recent2, &mut logic);
}

/// Like `Variable::from_join`, but `logic` returns `None` for pairs that produce nothing.
pub(crate) fn join_filter_into<'me, K: Ord, V1: Ord, V2: Ord, R: Ord>(
    input1: impl JoinInput<'me, (K, V1)>,
    input2: impl JoinInput<'me, (K, V2)>,
    output: &Variable<R>,
    mut logic: impl FnMut(&K, &V1, &V2) -> Option<R>,
) {
    let mut results = Vec::new();
    join_each(input1, input2, |k, v1, v2| results.extend(logic(k, v1, v2)));
    output.insert(Relation::from_vec(results));
}

// From datafrog 2.0.1 (MIT/Apache-2.0), where it is private.
fn join_helper<K: Ord, V1, V2>(
    mut slice1: &[(K, V1)],
    mut slice2: &[(K, V2)],
    mut result: impl FnMut(&K, &V1, &V2),
) {
    while !slice1.is_empty() && !slice2.is_empty() {
        use std::cmp::Ordering;

        // If the keys match produce tuples, else advance the smaller key until they might.
        match slice1[0].0.cmp(&slice2[0].0) {
            Ordering::Less => {
                slice1 = gallop(slice1, |x| x.0 < slice2[0].0);
            }
            Ordering::Equal => {
                // Determine the number of matching keys in each slice.
                let count1 = slice1.iter().take_while(|x| x.0 == slice1[0].0).count();
                let count2 = slice2.iter().take_while(|x| x.0 == slice2[0].0).count();

                // Produce results from the cross-product of matches.
                for index1 in 0..count1 {
                    for s2 in slice2[..count2].iter() {
                        result(&slice1[0].0, &slice1[index1].1, &s2.1);
                    }
                }

                // Advance slices past this key.
                slice1 = &slice1[count1..];
                slice2 = &slice2[count2..];
            }
            Ordering::Greater => {
                slice2 = gallop(slice2, |x| x.0 < slice1[0].0);
            }
        }
    }
}

// From datafrog 2.0.1 (MIT/Apache-2.0), where it is private.
fn gallop<T>(mut slice: &[T], mut cmp: impl FnMut(&T) -> bool) -> &[T] {
    // if empty slice, or already >= element, return
    if !slice.is_empty() && cmp(&slice[0]) {
        let mut step = 1;
        while step < slice.len() && cmp(&slice[step]) {
            slice = &slice[step..];
            step <<= 1;
        }

        step >>= 1;
        while step > 0 {
            if step < slice.len() && cmp(&slice[step]) {
                slice = &slice[step..];
            }
            step >>= 1;
        }

        slice = &slice[1..]; // advance one, as we always stayed < value
    }

    slice
}
