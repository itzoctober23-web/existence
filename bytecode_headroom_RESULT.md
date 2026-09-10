# CRATE 4's register bytecode cannot be a throughput win: the headroom is ~4%

The loop brief lists it as task 5, *"now a PERF task, not a survival one"*. As a perf task it is
already bounded, and the bound comes from a measurement the project has been taking all along.

## The measurement

`bench_interp` compares the INTERPRETED seed against a hand-written Rust bare alpha-beta on the same
net, and its equivalence check proves both arms walk the same tree:

    hand-written        46,312 nodes    0.2206s
    interpreted    141,705,122 cost     0.2274s
    EQUIVALENCE CHECK  evals: hand 42,820 vs interp 42,820  -> same tree
    BEST-OF-9  0.954x

Best-of-N across independent invocations: **0.954 - 0.970x**, stable to ~1% (see
`interp_gate_precision_RESULT.md` for why best-of and not median).

## The arithmetic

    interp/hand = 0.961  ->  interp takes 1.0406x the hand time  ->  overhead = 4.1% of interp time
    interp/hand = 0.970  ->  interp takes 1.0309x the hand time  ->  overhead = 3.1% of interp time

**A register bytecode's entire job is removing interpretation overhead. That overhead is 3-4%, so 3-4%
is its CEILING** -- achievable only if the compiled bytecode matches hand-written Rust exactly, which
nothing does.

`throughput_RESULT.md` states what MASTER_PLAN's "make deep search cheap enough" route actually needs:
**~10x**. A 4% ceiling is not within two orders of magnitude of that.

## What this does NOT say

It does not say the bytecode is worthless. CRATE 4 specifies it, and `interp/src/lib.rs:3` is explicit
that the tree-walker is stage 1 and its ratio is *"a LOWER BOUND on what the design can achieve"* --
that framing is correct and unaffected. If the bytecode is built for design reasons, this document is
not an argument against it. It says only that **it must not be justified, scheduled, or measured as a
throughput win**, because the throughput it can buy is bounded at ~4%.

The result is also a genuinely good one in the other direction: **a tree-walking interpreter running
within 4% of hand-written Rust means the interpretation layer is close to free.** That is the
favourable reading of the same number, and it is why the ratio has always cleared the 0.50 gate so
comfortably.

## Where the time actually is, and therefore what the real target is

`node_profile` (width 16, 200 positions, best of 9):

    legal_moves() TOTAL      395.5 ns     <- 59% of a 669 ns interior node
      residual (pins+emit)   351.6 ns     (89% of movegen)
    make + unmake             42.0 ns
    eval                     244.7 ns     (26.8% of a LEAF)
    shuffle + buffer copy    231.5 ns

**Movegen is the target, not dispatch and not eval.** `throughput_RESULT.md` reaches the same place
from the other direction -- after refuting the shuffle's division end-to-end it concludes *"there is no
cheap throughput win available in this engine ... needs a structural change (the bytecode, or a
fundamentally cheaper movegen)"*. Of those two named routes, this document rules the first one out on
headroom, which leaves movegen.

## Method note, because I nearly repeated a documented mistake

`node_profile` prints `division cost 139.9 ns/node available` for the shuffle's modulo, which reads as
a 15-21% win sitting on the table. It is not: the multiply-shift replacement was implemented at both
call sites, measured end-to-end, and **REVERTED** -- it is SLOWER in wall clock at depths 4 and 5
(0.141 vs 0.139s, 2.430 vs 2.324s), because the per-node saving is 2-4% rather than 12% AND the
different permutation grew the tree 6-7%.

That is recorded in `throughput_RESULT.md:205`, which also states the general rule: **`node_profile` is
useful for RANKING components and is not usable for predicting end-to-end gains.** I read the profiler
line first and the refutation second; the order should have been reversed, and the check cost seconds.
