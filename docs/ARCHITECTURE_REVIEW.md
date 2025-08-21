# Architecture review and idiomatic Rust improvement plan

This document critiques the current structure and outlines concrete, incremental improvements to make the codebase more idiomatic Rust, more robust in error handling, and easier to observe via logging/tracing.

The feedback is based on the following modules: lib/src/common.rs, lib/src/error.rs, lib/src/index.rs, lib/src/disjoint_sets.rs, lib/src/reasoner.rs, lib/src/query.rs, lib/src/tests.rs.

## High-level structure and separation of concerns

Observations:
- Reasoning engine concerns (datafrog iteration, rule application, runtime state) are interleaved with parsing/serialization and indexing concerns.
- Two different RDF ecosystems are used: oxrdf/rio in core and rdf in query.rs. This increases cognitive load and maintenance costs.
- The runtime state mixes datafrog `Variable`s, shared maps/sets via `Rc<RefCell<_>>`, and manual vectors of triples.

Recommendations:
- Create clearer modules:
  - io: file parsing (rio_turtle), dumping (formatter), conversion glue (already largely in common.rs).
  - index: URI interning/indexing (see “Indexing and ID stability”).
  - engine: datafrog iteration and rule execution (most of reasoner.rs).
  - types: public newtypes and core aliases (URI, KeyedTriple, etc.), so they don’t live in multiple modules.
- Consolidate on one RDF stack (recommend oxrdf + rio) and feature-gate legacy query if needed.
  - Option A: Port query.rs to oxrdf, or
  - Option B: Put query.rs behind a cargo feature (e.g., legacy-query) to avoid pulling in two stacks.

## Error handling

Observations:
- There are two error types: ReasonableError (lib/src/error.rs) and ReasoningError (lib/src/reasoner.rs). The latter is local and not integrated with the former.
- Many `unwrap()` calls (e.g., index, disjoint_sets) will panic on malformed data instead of returning an error.
- Reasoner::load_file returns `std::io::Error` but may propagate parser errors that don’t belong to IO, conflating domains.

Recommendations:
- Use a single crate-visible error type with `thiserror` for ergonomic, idiomatic error modeling.
- Implement `From` conversions for rio_turtle::TurtleError, oxrdf::IriParseError, etc., into the library error.
- Replace panicking `unwrap()`s with error returns or explicit debug assertions where invariants are truly guaranteed.
- Return `Result<_, Error>` consistently from fallible functions in the library, and let the CLI convert to `anyhow::Result` at the boundary.

Example sketch:

```rust
// in lib/src/error.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ReasonableError {
    #[error("{0}")]
    ManagerError(String),
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    IriParse(#[from] oxrdf::IriParseError),
    #[error(transparent)]
    BNodeParse(#[from] oxrdf::BlankNodeIdParseError),
    #[error(transparent)]
    Turtle(#[from] rio_turtle::TurtleError),
    #[error("channel receive error: {0}")]
    ChannelRecv(#[from] std::sync::mpsc::RecvError),
}

pub type Result<T> = std::result::Result<T, ReasonableError>;
```

Then make library functions return `crate::error::Result<T>` instead of `std::io::Result<T>`.

## Logging and observability

Observations:
- The code uses the `log` facade; the CLI sets up `env_logger`. That’s fine, but spans/structured context would improve insight in complex rule iterations.
- Some debug prints exist in tests and code paths.

Recommendations:
- Consider moving to `tracing` with `tracing-subscriber` (still compatible with the log facade via `tracing-log`), adding spans around reasoning passes and rules to attribute costs.
- Keep `log` as a minimal baseline if you prefer not to introduce `tracing`, but remove ad-hoc prints and use proper log levels consistently (trace/debug/info/warn/error).
- For performance investigations, add optional timing metrics per rule group (feature-flagged), e.g., using `tracing` spans or `instant` for minimal overhead.

Sketch:

```rust
// engine boundaries
let _span = tracing::info_span!("reason", triples = self.input.len()).entered();
// inside datafrog iteration
tracing::trace!("starting fixed-point iteration");
```

## Indexing and ID stability

Observations:
- `URIIndex` maps `Term` to a `u32` computed via farmhash of `Term::to_string()`. Risks:
  - Collisions (hash map key is the ID; collisions overwrite).
  - `to_string()` allocations and reliance on formatting stability.
  - Reserve 0 as sentinel by inserting “urn:_”; later code filters entries with > 0. This is brittle.

Recommendations:
- Intern `Term` (or the underlying IRI for named nodes) to stable, collision-free IDs by maintaining a bijection map:
  - `HashMap<Term, NonZeroU32>` and `Vec<Term>` (or `IndexVec`) so you can go both ways.
  - Avoid `to_string()`; hash and compare on the `Term` variant data directly (derive Hash/Eq on Term if possible or wrap IRIs).
- Introduce a `newtype` for URI for type safety:
  - `#[repr(transparent)] struct UriId(NonZeroU32);` or `pub struct URI(NonZeroU32);`
- Replace sentinel `0` logic with `Option<URI>` or use `NonZeroU32` to encode “absence” as `None`. Remove the (0,0,0) triple pattern used as “no-op”.

Benefits:
- No accidental overwrites, no string allocations on the hot path, clearer invariants.

## Data structures and mutability

Observations:
- Widespread use of `Rc<RefCell<...>>` inside `Reasoner` for sets and maps of intermediate facts, making the type `!Send` and harder to reason about.
- `DisjointSets::new` uses iterators for side effects and multiple `unwrap()`s; it clones, collects, and then unions in a somewhat convoluted layout.
- `get_list_values` clones into a `Vec<URI>` instead of returning a view/iterator.

Recommendations:
- Encapsulate mutable internal state behind private methods; pass immutable borrows to datafrog closures where possible. Replace `Rc<RefCell<_>>` with owned fields and local mutable variables (or dedicated small structs) unless shared aliasing is truly necessary.
- In `DisjointSets`:
  - Accept `&[KeyedTriple]` instead of `&Vec<_>`.
  - Use straightforward `for` loops instead of `.map(...).count()` side-effect patterns.
  - Avoid `unwrap()` on lookups; return `Result` and allow caller to decide, or assert invariants once and comment them.
  - Store list buckets sorted and return `Option<&[URI]>` from `get_list_values` (slice view avoids clones).
- Where sentinel triples `(0, (0, 0))` are currently emitted and later filtered, prefer emitting nothing (e.g., use guards or dedicated filtered Variables). This both reduces memory traffic and avoids mixing semantics.

## Reasoner runtime and lifecycle

Observations:
- `Reasoner::rebuild` partially re-initializes variables, but not all; this risks “dangling” datafrog variables or stale state after multiple runs.
- `reason()` has an outer loop for complements and an inner datafrog fixed-point loop; both include a lot of state mutation across multiple fields.

Recommendations:
- Extract a dedicated “runtime” struct that owns the datafrog `Iteration` and all `Variable`s needed for a run. Build it from `self.base` + pending triples and drop it when done. Keep long-lived index and config in `Reasoner`.
- Alternatively, re-initialize all `Variable`s cleanly between runs and document the lifecycle; avoid reusing some but not others.
- Introduce an internal builder for rule groups so they are easier to test in isolation and reason about.

## Performance hotspots and allocation hygiene

Observations:
- Frequent `to_string()` conversions on `Term` for hashing or logging.
- Multiple `sort()` + `contains()` checks; `get_unique()` is O(n^2) in the worst case even with sorting.
- Copies of large vectors of triples across phases.

Recommendations:
- Remove `to_string()` from hot path; hash on raw IRI bytes or reference slices.
- Replace `get_unique()` with a linear-time two-pointer merge since both inputs are sorted:
  - Walk both vectors once, pushing only non-overlapping items.
- Prefer iterators and small, well-scoped allocations; use `Vec::with_capacity` with known bounds.
- Consider arenas or interning for recurring nodes to reduce term cloning.

## API ergonomics

Observations:
- `load_file` returns `std::io::Error` instead of crate error.
- Methods clone and return `Vec<Triple>`; there is also `view_output(&[Triple])` so duplication exists.
- Mix of `get_*` that clone vs. `view_*` that borrow, not consistently named.

Recommendations:
- Consistently return `crate::error::Result<_>`.
- Provide read-only views and iterators where possible; clone only at explicit user request.
- Introduce a builder for `Reasoner` configuration (e.g., enabling/disabling rule groups, transitivity, complement passes).

## Rule implementation style

Observations:
- Rule implementations sometimes emit sentinel triples `(0, (0, 0))` to be filtered later.
- Many repetitive `from_join` calls that could be grouped or abstracted slightly without hiding Datalog semantics.

Recommendations:
- Prefer guarded joins or post-join filtering to avoid generating invalid tuples.
- Consider factoring out helpers/macros for common idioms (e.g., emit_same_as_when_eq, emit_type_inference) to reduce boilerplate while keeping the structure explicit.

## Consistency and dependencies

Observations:
- query.rs uses a different RDF stack (`rdf` crate) and pulls in a custom S-expression parser; it’s conceptually orthogonal to this library’s core purpose.

Recommendations:
- Either:
  - Port query.rs to oxrdf/rio and integrate properly, or
  - Feature-gate it off by default and document it as “experimental/legacy”.
- Add `#![deny(missing_docs)]` at the crate root for public items (gradually).
- Run `clippy` in CI with a baseline allowed list; fix or allow lints explicitly.

## Testing and CI

Observations:
- Unit tests cover many rules. Good!
- Tests rely on stringly-equal output (`to_string()`), which can be brittle across upstream crate changes.
- No property-based tests around equivalences and symmetry/transitivity.

Recommendations:
- Prefer structural comparisons: compare `NamedNode` IRIs or `Term` variants directly.
- Add property-based tests (e.g., proptest) for eq-sym, eq-trans, and prp rules.
- Add CI steps:
  - cargo fmt --check
  - cargo clippy --all-features -- -D warnings

## Documentation

Recommendations:
- Add module-level docs to engine, index, and io modules explaining invariants (e.g., no zero/sentinel IDs).
- Document complexity characteristics of major operations (e.g., rule groups).
- Provide end-to-end examples in lib docs using oxrdf/rio only.

## Prioritized action plan

1) Replace sentinel IDs and hashing in `URIIndex` with a true interner (collision-free, stable).
2) Unify error handling using `thiserror`; propagate Results; remove `unwrap()`s in library code.
3) Separate runtime: re-initialize datafrog variables per run (or encapsulate in a runtime struct).
4) Remove sentinel triples; use filtered joins or guards instead.
5) Consolidate RDF stack; feature-gate legacy query module or port it to oxrdf.
6) Adopt `tracing` (optional) with spans for reasoning passes; keep `log` facade compatibility.
7) Improve `DisjointSets` API and safety; return slices and avoid clones.
8) Performance cleanup: `get_unique` linear merge, reduce clones/to_string, capacity hints.
9) Add CI linting (fmt, clippy) and property-based tests for rule properties.
10) Expand public API docs and examples; introduce a Reasoner builder.

This plan is incremental: items (1)-(4) yield the most correctness/robustness benefits with limited surface-area changes; (5)-(10) improve maintainability and developer experience.
