# replay_ab.sh — read the arms correctly (noted 2026-09-08, DURING the run)

At 22s/generation, a 420s arm reaches ~19 generations. So the three windows bind very
differently, and the labels are misleading if taken at face value:

| arm | binds from | effective meaning at this budget |
|---|---|---|
| `--replay-gens 2` | generation 3 | a real, tight window (~17 gens of binding) |
| `--replay-gens 8` | generation 9 | a real window (~11 gens of binding) |
| `--replay-gens 32` | generation 33 | **NEVER BINDS — this is the "keep everything" arm** |

That is not a wasted arm. "Keep everything" is exactly what his argument predicts should win:
if the champion is not improving (measured flat at 0.838 / 0.853 / 0.831 across generations
30/60/90), then old positions came from an equally-strong player, the staleness premise fails,
and discarding is pure loss. The unlimited arm tests that directly.

But it must be REPORTED as "unlimited", not as "a 32-generation window". Testing 8 vs 32 as
windows would need arms of ~1500s+ so that 32 actually binds for long enough to matter, which
is a different and more expensive experiment.

WHAT THIS RUN CAN AND CANNOT ANSWER
  CAN: does discarding hurt at all? (2 and 8 vs unlimited)
  CANNOT: what the best finite window is. 8 vs 32 is not tested here, because 32 never engages.

Also unchanged: the loop's 0.151 run-to-run variance means a small difference will not resolve.
A null is NOT RESOLVED at this budget, never no-effect.

## WHICH REGIME THIS TESTS (noted during the run, 2026-09-08)

The arms start from SCRATCH (no --init), so they cover generations 1-19, where the champion is
improving rapidly. His argument is about the PLATEAU regime -- gens 30-90 measured flat at
0.838 / 0.853 / 0.831 -- where old positions came from an equally-strong player and the
staleness premise fails.

These are different regimes, and the distinction cuts in a useful direction:

  EARLY (what this run tests): the champion IS improving fast, so old positions really ARE from
  a weaker player. The conventional staleness argument is at its STRONGEST here. This is the
  HARDEST case for "keep everything".

  PLATEAU (what his argument is about): the champion is not improving, so the staleness premise
  does not apply and discarding should be pure loss.

So a win for the unlimited arm HERE would be strong evidence -- it would mean keeping old data
helps even in the regime where discarding has its best case. A win for the tight window here
proves much less about the plateau, because the two regimes genuinely differ.

FOLLOW-UP OWED either way: rerun with `--init champion_long.net` so every arm starts from the
plateaued champion. That is the regime the argument is actually about. Not done in this run
because the script was already executing and editing a running bash script is how 2811 gate
pairs were destroyed -- bash reads it by byte offset.

## CONFOUND found mid-run (2026-09-08): the surrogate systematically penalises the larger windows

Observed at generation 12:

    arm 1 (window 1)  train 9747-49151   loss ~0.04
    arm 8 (window 8)  train 44697        loss 0.0718   pool 296393

The larger-window arm trains on 6.6x more data and reaches HIGHER held-out loss. That is not a
failure -- a net fitted to eight generations of varied positions will fit THIS generation's
held-out slice worse than one fitted to that slice alone. It is the ordinary bias/variance
trade, showing up exactly where you would expect.

The problem is what judges it. The sweep runs `--gate-pairs 32`, so ci95 ~0.079 and the
`resolves` test (ci95 < 0.05) is almost never true -- which means the FITNESS 5 surrogate, whose
statistic is held-out loss on this generation's slice, decides most accepts. So the larger-window
arms are penalised by the very quantity that a larger window is expected to raise.

MEASURED FROM THE LEDGER, not inferred from the interval width -- the gate resolved in **0 of 57
generations**:

    rp_1 (window 1)  0/42 could resolve   accepted 6, regression 14, no_evidence 22
    rp_8 (window 8)  0/15 could resolve   accepted 2, regression  4, no_evidence  9

Not "rarely". Never. Every acceptance decision in this sweep came from the surrogate or the
non-regression guard, and `no_evidence` dominates (31 of 57) -- the loop honestly reporting that
its gate saw nothing either way. The strict branch of the acceptance rule did not execute once.

WHAT THIS DOES AND DOES NOT INVALIDATE
  - The FINAL verdict is sound. Each arm's champion is scored against the same frozen origin by
    GAMES (examples/control.rs, uncapped depth 2), which the surrogate cannot touch.
  - The PATH to that champion is biased. If arm 8's genuinely stronger candidates were rejected
    on loss, arm 8 ends with a weaker champion, and the sweep would attribute that to the WINDOW
    when the cause is the surrogate.

So a win for the small window must NOT be read as "history hurts". It could equally be "history
raises single-generation held-out loss, and the current acceptance rule punishes that". A win for
the LARGE window is the cleaner result: it would have happened despite this bias, not because of
it.

THE FIX is already identified and committed for other reasons: gate-pairs 40 -> 224 puts the
expected interval at 0.030, under the 0.05 `resolves` threshold, so the GAMES decide and the
surrogate returns to proposing. Re-running this sweep at 224 pairs would remove the confound
entirely. Not done here because changing the binary mid-sweep gives the arms different code,
which is the confound that already invalidated one experiment today.
