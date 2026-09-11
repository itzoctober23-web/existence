# The uncertainty head is FITTED, and the in-engine head reproduces the offline ranking EXACTLY

2026-09-11. `fit_unc_head.py` + `crates/pipeline/examples/inject_unc_head.rs`, on the same 296-row
dump and the same 207/89 split as `unc_probe.py`.

## Why this had to happen before any grammar discovery run

`crates/interp/tests/unc_primitive.rs` asserts that an untrained head returns exactly 0. The champion
is untrained, so `unc(p)` was a **constant** — and a grammar row with zero variance cannot change any
program's behaviour. A mutation inserting `unc(p)` produces a program that plays *identically* to its
parent, so fitness cannot distinguish them and selection cannot retain it. `UNC_READ_IN_MAIN` would
have been measuring drift.

**Discovery on a constant row is guaranteed-null BY CONSTRUCTION, not by the paradigm failing.** That
is the trap this closes: a null result from an uncertainty-gated-extension search would have looked
like evidence against the idea while actually being evidence about a zeroed vector. Step 1 gates
step 2.

## This is a fit, not a training run — and that is not a shortcut

`Net::spread_from` IS a linear map over the ReLU'd hidden layer times a scale: 16 weights and a bias
at width 16. That is closed-form least squares. **No gradient loop, no GPU, no datagen.** The probe
that answered "does a signal exist" and the fit that produces deployable coefficients are the same
computation; only the disposition of the output differs.

## Control first: the fitter reproduces the published probe

Same seed (20260911), same shuffle, same 70/30 cut, same net filter, same ridge routine:

```
train 207   holdout 89 (38 costly)
offline head AUC          0.305        NEGATED  0.695
random band       [0.394, 0.600]
```

Identical to `unc_signal_is_inverted_RESULT.md` in every digit. A fitter that did not land on the
published split would be fitting something else.

## The measurement that mattered: does the REAL head agree?

The offline AUC is a claim about a Python dot product. What runs in search is `spread_from`, which
additionally multiplies by `scale`, **clamps to [0, 30000]**, and **casts to i32**. The clamp was the
specific worry: the allocator consumes this signal NEGATED, so the positions it cares about are the
LOWEST predictions — exactly the ones a floor-at-zero would flatten into one tied value.

```
                       AUC      NEGATED
  offline (float)     0.305      0.695
  in-net spread_from  0.305      0.695

  clamped to 0 by spread_from : 0/89 (0%)
  round-trip: eval params differing = 0; head preserved = true
```

**The clamp hypothesis is REFUTED on this holdout** — no prediction reaches the floor, so the cast
and clamp cost nothing here. The in-engine head reproduces the offline ranking to three decimals.

**Eval is provably untouched.** The round-trip is checked by comparing every element of
w1/b1/w2/b2/scale, not by sampling evals on a few positions — a sampled check could pass while a
weight was corrupted in a region those positions never reach.

## What is NOT established

**No Elo is claimed and none is implied.** This says the head now carries the ranking the probe
found. Whether allocating search by it wins games is Track B step 2, untouched.

**Calibrated on p1_champion.net, and the effect is net-dependent.**
`unc_signal_is_inverted_RESULT.md` measured this signal ABSENT on `epochs_03`. These coefficients
are this net's. **They must be refitted after any promotion**, and the champion does change.

**89 holdout points, 38 costly, one net, one seed.** AUC 0.695 is outside the random band but it is
not a large effect.

**The clamp is refuted for THIS holdout, not in general.** A different net or target could produce
negative predictions, and then the floor would bite exactly where it hurts most. `inject_unc_head`
prints the clamped count every run so this cannot pass unnoticed.

## The deployment gap this exposes

A net carrying a head serialises as **schema v2**, and `crates/nnue/src/lib.rs:436-443` records why
that matters: the long-lived snapshot binaries on this box — trainer, ruler, P2 arms — predate v2 and
**cannot load it**. So the fitted net was written to a scratch path, and nothing in production reads
it.

That means a discovery run which needs a varying `unc(p)` requires a freshly-built binary AND a v2
net. Running `evolve` against the current champion would still see a constant 0. **This is the next
concrete blocker for Track A**, and it is a packaging problem, not a research one.

## Reproduce

```
./fit_unc_head.py <dump.tsv> p1_champion.net w.txt hold.tsv resid
cargo build --release --example inject_unc_head -p pipeline
inject_unc_head p1_champion.net w.txt hold.tsv <scratch>/p1_champion_unc.net
```

## A note on the harness

The first run of `inject_unc_head` printed `n/a` for every AUC and `0/0 (NaN%)` clamped, because a
column guard demanded `4 + n_hidden` fields where the holdout has `3 + n_hidden`. It parsed zero rows
and said so in a way that reads like "no signal" rather than "no data". It now **asserts** on an empty
parse. An empty result is a broken probe until proven otherwise.
