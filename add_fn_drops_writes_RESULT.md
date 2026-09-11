# `Op::AddFn` was NOT behaviour-preserving — it lifted alpha-beta's score update and dropped the write, playing a different move 91x cheaper. Found, root-caused and fixed 2026-09-11

`mutate::PARKED_OPS` keeps `Op::AddFn` out of the drawn set on the strength of two claims, both
stated as self-evident and neither tested:

1. *"AddFn is behaviour-PRESERVING by construction — it is a refactor"*;
2. *"every candidate it produces is its parent plus a `Call` node, which `interp::cost_of` charges 2
   … under FITNESS 3 (mates per COST) that is strictly worse"*, therefore it cannot be accepted and
   is safe to leave implemented.

`crates/grammar/tests/add_fn.rs` checks that the operator adds a function, keeps scope, stays
well-typed, and refuses to lift a `ret`. **Nothing ran a lifted program and compared what it DID to
its parent.** Claim 1 is false, and claim 2 is false in the dangerous direction.

## The measurement

`crates/interp/tests/add_fn_diagnose.rs`, bare alpha-beta (main seed), `fi=1` (the `ab` function),
`k=40`, startpos, budget 4:

```
DETERMINISM CONTROL: parent twice -> Move(6030) / Move(6030)   cost 133906979 / 133906979
lifted fn: lifted2 params=[("vv", Score)] ret=Unit body_size=4
lifted body = Set("best", Max(Var("best"), Var("vv")))

parent    move=Move(6030)  cost=133906979  forfeit=false
candidate move=Move(1025)  cost=1463398    forfeit=false
cost cap  = 2000000000
```

**The lifted subtree is alpha-beta's score update, `best = max(best, vv)`.** The candidate plays a
different move and costs **91.5x less**.

## The controls, run before the claim

* **Determinism.** The same program run twice returns the same move and the same cost, to the unit.
  Had it not, the comparison would have been meaningless and the failure mine.
* **Cost cap.** `cost_cap` is 2e9; the larger run reaches 1.3e8 and neither side forfeits, so
  truncation is excluded rather than assumed.
* **The candidate is genuinely different.** `funcs.len()` rose on every candidate, so "identical
  behaviour" could not have been vacuously true.
* **Not everything forfeits.** After the fix, 180 of 180 comparisons returned a real move. A test
  where both sides return `MOVE_NONE` proves nothing, and that is asserted, not hoped.

## Root cause — three correct pieces composing into a wrong whole

1. `Node::Call` builds a **fresh env** from the parameters, executes the body against it, and drops
   it (`interp/src/lib.rs`, `Node::Call` arm). Only the return value escapes. A write inside a
   callee cannot reach its caller.
2. `typecheck::free_vars` does not report an assignment target as free. Its own comment says why:
   *"A `Set` inside the subtree creates its own name, so it is not a requirement on the site."* That
   is **right for deciding what a site REQUIRES and wrong for deciding what a lift must THREAD
   OUT.** So `best` never became a parameter — only `vv` did.
3. `typecheck::scope_check` sees the `Set` as a binder and reports no unbound read.

The result is a well-typed, well-scoped program whose meaning has been destroyed. **No type checker
can catch this**, which is exactly why the untested premise survived.

## Claim 2 is not merely wrong, it is backwards — and that is the part that mattered

Parking was justified by cost *rising*: a candidate is "strictly worse … and cannot be accepted".
A lift that silently drops a write is not costlier. It was **91.5x cheaper**, because the search it
gutted no longer runs. FITNESS 3 is mates per COST, so the operator was not harmlessly-worse; it
could be spuriously-**better**.

**NOT established, and not claimed:** that such a candidate would actually have been selected. That
needs its mate count, which was not measured. What is established is that the stated reason for
believing the operator safe does not hold, and it fails toward acceptance rather than away from it.

## The fix

`mutate::contains_set`, mirroring the existing `contains_ret` refusal: `try_add_fn` declines any
subtree containing a `Set`.

Deliberately conservative — it also refuses a `Set` whose target the subtree itself binds, which
would lift correctly. Separating those needs precisely the binder analysis shown above to be wrong
for this question, and the operator is parked, so a lost valid lift costs nothing while a lost write
corrupts a candidate invisibly.

### Verified after the fix

```
behaviour:  60 candidates, 180 program/position pairs, 180 identical moves
            180 of 180 returned a REAL move (nothing forfeited)
cost:       rose 172   unchanged 8   FELL 0      worst ratio 1.004x the parent
coverage:   crates/grammar/tests/add_fn.rs still applies AddFn 539 times
```

Claim 2 is now true **and quantified**: a lift costs at most 1.004x its parent here, where the park
comment asserted a bare "+2" without a denominator. Claim 1 is now tested rather than asserted, by
`crates/interp/tests/add_fn_is_behaviour_preserving.rs`, which pins it.

The guard did not over-refuse: 539 applications remain, so the operator still closes the function-
count gap (`0 of 858` before it existed) that is its entire purpose.

## What this does and does not change today

**Nothing in any current run.** `Op::AddFn` is in `PARKED_OPS`, never drawn, so no live search has
ever produced one of these candidates. This is a latent defect fixed before its trigger, not a
regression recovered.

**What it changes is the unpark decision.** `mutate.rs:148` records the unpark condition as "once
something can diverge the lifted body from its origin, or the cost model stops charging a bare
call". Had AddFn been unparked before today it would have injected silent semantic corruption
labelled "refactor", with a cost profile that made the corruption look like an improvement.

## A methodological note worth keeping

The bug was reachable only by running the program. Four separate checks — the type checker, the
scope checker, and two existing tests — all passed on a program whose central update had been
deleted. The premise "behaviour-preserving by construction" had been carried in a doc comment, a
test-file header, and GRAMMAR 4, and repetition across three places is not evidence.

Cost was also the more sensitive detector than behaviour: a 91x drop is unmistakable, whereas a
changed move could plausibly be dismissed as noise by anyone who had not run the determinism
control first.
