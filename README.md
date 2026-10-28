# egyptian-fraction-search

A small Rust library and CLI for exact Egyptian fractions: sums of distinct unit
fractions. Compare the greedy algorithm with bounded exhaustive search, choosing
between fewer terms and a smaller largest denominator.

For example, both of these represent **2/7**:

- Greedy: `1/4 + 1/28` — 2 terms, largest denominator 28.
- Alternative: `1/6 + 1/14 + 1/21` — 3 terms, largest denominator 21.

## Run

Use a Rust toolchain supporting edition 2024. All commands run locally.

```sh
cargo run --release -- compare 2/7 --max-denominator 35 --max-terms 4
cargo run --release -- greedy 4/17
cargo run --release -- search 4/5 --max-denominator 10 --max-terms 3 --objective terms
cargo run --release -- search 2/7 --max-denominator 35 --max-terms 4 --objective denominator
cargo run --release -- verify 2/7 6 14 21
cargo run --release -- --help
```

`verify` also accepts comma-separated denominators, such as `6,14,21`. Every input
must satisfy `0 < numerator < denominator`; GCD reduction happens before search
limits are checked. Signs, zero denominators, and improper fractions are rejected.

## Search bounds and objectives

Search deliberately stays small:

| Setting | Default | Allowed |
| --- | ---: | --- |
| Reduced input numerator | — | 1–32 |
| Reduced input denominator | — | 2–200 |
| `--max-denominator` | 200 | 2–10,000, inclusive |
| `--max-terms` | 5 | 1–8, inclusive |
| `--max-nodes` | 1,000,000 | Any `u64`; zero performs no search |
| `--objective` | `pareto` | `terms`, `denominator`, `pareto` |

- `terms` minimizes term count, then largest denominator.
- `denominator` minimizes largest denominator, then term count.
- `pareto` keeps the nondominated trade-offs between those two metrics.

Each metric pair has one representative: the lexicographically smallest
denominator list. Search returns representatives, rather than every equivalent
decomposition. All denominators are strictly increasing.

A completed result is **optimal within the configured denominator and term
bounds**. An exhausted node budget returns **best found; search incomplete**,
possibly with no solution yet. A completed empty search reports **no solution
within bounds**. Neither an empty bounded search nor a stopped search establishes
that the fraction has no Egyptian representation.

Greedy is compared separately and may exceed the search bounds. Its independent
limits are `--max-steps` (default 128) and `--max-bits` (default 16,384). The bit
limit applies to input components, selected denominators, and reduced remainders.
If a limit is reached, the CLI reports the partial prefix and exact remaining
fraction. Greedy accepts larger inputs than bounded search.

Exit codes: `0` for a completed run (including no solution within bounds), `2` for
invalid input, failed verification, or an I/O error, and `3` for an incomplete
greedy or search result. `compare` returns `3` if either algorithm is incomplete.

## Algorithms and exact verification

Rationals use `num-bigint::BigUint` and Euclid's GCD. No floating point arithmetic
is used to construct or verify decompositions.

Greedy chooses `d = ceil(b/a)` from the remainder `a/b`, then reduces
`(a*d - b)/(b*d)` and repeats until zero.

Search enumerates exact lengths from one through the term ceiling. With `k`
terms left, previous denominator `p`, and ceiling `D`, the next denominator lies
between `max(p+1, ceil(b/a))` and `min(D-k+1, floor(k*b/a))`. These bounds avoid
overshooting the remainder and leave room for distinct later denominators. A
single remaining term is checked directly. For the `terms` objective, the search
can stop after fully exploring the first successful length; other objectives
explore every allowed length unless the node budget is exhausted.

The CLI checks every complete result by independent exact fraction addition and
cross multiplication. Search results are also checked against their configured
bounds. Partial greedy prefixes are verified together with their exact remainder;
they are never displayed as complete representations.

## Metrics

Greedy and search have separate work reports. Parsing and independent verification
are excluded from algorithm counters and runtime.

| Metric | Definition |
| --- | --- |
| Terms | Number of unit fractions in the reported decomposition or partial prefix |
| Largest denominator | Maximum reported denominator; absent for an empty prefix |
| Denominators examined | Greedy selections; for search, candidates whose remainders are computed plus feasible final unit denominators accepted within bounds |
| Search nodes | Entered DFS states, including exact-length roots and immediately pruned states |
| Search branches | Recursive edges into entered DFS states; roots are excluded |
| Arithmetic operations | Executed big-integer additions, subtractions, multiplications, divisions, and remainders |
| GCD steps | Euclidean remainder operations, already included in arithmetic operations |
| Runtime | Measured algorithm elapsed time, including result selection |

Arithmetic counts exclude comparisons, allocations, and primitive loop counters.
They count software operations, not CPU instructions or equal-cost work. Candidate
arithmetic may occur before a child node is denied by the node budget. Greedy has
zero search nodes and branches.

## Library and validation

`src/lib.rs` exports `Rational`, `greedy`, `bounded_search`, their configuration
and report types, and independent verification functions. The binary adds argument
parsing and human-readable output.

```sh
cargo test
cargo test --release
cargo fmt --check
cargo clippy --all-targets -- -D warnings
```

Tests cover large-integer reduction, invalid inputs, greedy exactness over a grid,
all search objectives against independent tiny subset enumeration, deterministic
ties, inclusive bounds, budget boundaries, partial results, and CLI exit codes.

Generated outputs belong in ignored `artifacts/`; Cargo output is ignored under
`target/`.
