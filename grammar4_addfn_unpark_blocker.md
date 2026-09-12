# The AddFn unpark condition is blocked by a SECOND ordering bias — the function sweep short-circuits

**2026-09-11 23:55.** Code-reading finding, not a runtime measurement. Recorded because it changes what
the next GRAMMAR 4 step is, and because the measurement that would confirm it is cheap and named below.

## Why this was looked at

`p2_fitness_RESULT.md` closed the fitness-composition route today, and both it and
`gate_power_RESULT.md` name the same successor: **GRAMMAR 4 — the type checker and mutation
operators**. The live gap `shape_reachability.rs` asserts is:

> *"0 of 858 applied mutations change `Program::funcs.len()`, so GRAMMAR 4's `add-fn` remains absent
> and every program the search can reach has exactly one function, against a type checker that admits
> 1..4."*

`Op::AddFn` is the only operator that can raise `funcs.len()`, and it is in `PARKED_OPS`. Its unpark
condition (`mutate.rs:148`) is recorded as:

> *"UNPARK WHEN there is a reason for a second function to EARN its call — i.e. once something can
> diverge the lifted body from its origin, or the cost model stops charging a bare call."*

`add_fn_drops_writes_RESULT.md` fixed AddFn TODAY (it had been silently dropping alpha-beta's score
update). So the first clause deserved re-checking against the code as it now stands.

## What the code says

Mutation sites CAN address any function — `mutate()` at `mutate.rs:450` picks
`fi = rng.below(p.funcs.len())`, uniformly. But the search does not call that. It calls
`mutate_program_n`, which sweeps functions **in order and breaks on the first successful placement**:

```rust
for fi in 0..cur.funcs.len() {
    let total = count_nodes(&cur.funcs[fi].body);
    ... shuffled positions ...
    for k in order { if let Some(next) = mutate_at(&cur, op, rng, fi, k) { ok = Some(next); break; } }
    if ok.is_some() { break; }          // <-- funcs[1..] reached ONLY if funcs[0] matched nowhere
}
```

**So a lifted body can diverge only when the drawn operator cannot be placed anywhere in `funcs[0]`.**
For broadly-applicable operators that is essentially never — the same file records that `InsertMax`
wraps *any* Score node and won 32 of 67 ledger placements, while `Tweak` matches ~6 of the seed's 71
nodes and won 0.

## The shape of this is a bias ALREADY FIXED one level up

The comment directly above that loop describes the identical problem for OPERATORS and its fix:

> *"The original drew a fresh random operator on every retry and kept whichever applied first, which is
> a race that broadly-applicable operators always win … PICK THE OPERATOR FIRST, then retry that SAME
> operator on different nodes."*

That fix made the drawn operator set the declared one. **The same race remains across FUNCTIONS**:
positions are shuffled *within* a function, but functions are tried in index order and the first hit
wins. Pick-first-then-place was applied to the operator dimension and not to the function dimension.

## What this means for the unpark decision

The unpark condition is **not** met, but not for the reason previously recorded. It is not that
divergence is impossible — the code supports it. It is that the search reaches `funcs[1..]` only as a
fallback, so a lifted function would sit inert, costing its `Call` (measured ≤1.004x the parent) and
earning nothing. Unparking AddFn alone would therefore still produce only strictly-worse candidates,
exactly as the park says — via a different mechanism than the one written down.

## The measurement that would settle it — cheap, and NOT run here

Construct a 2-function program (apply AddFn once, directly), then draw N mutations via
`mutate_program_n` and count how many land in `funcs[1]`. The prediction from the code is **≈0 for
broadly-applicable operators**. If confirmed, the GRAMMAR 4 step is to make function choice
independent of placement order — the same fix already applied to operator choice — and only then
re-examine unparking AddFn.

**Not done tonight:** that is a `cargo test -p grammar` build, and the Existence slot is running the
953-pair promotion resolve. Nothing here changes a live search: AddFn stays parked, and no operator
behaviour was modified.
