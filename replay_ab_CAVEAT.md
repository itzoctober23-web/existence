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
