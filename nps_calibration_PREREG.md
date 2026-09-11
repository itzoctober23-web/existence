# PRE-REGISTRATION: the engine spends 1–11% of its clock, and `NPS_PER_MS` is stale by ~2×

**2026-09-10, written before the change is made.**

## What was measured

`go movetime T` converts the clock to a node budget (`T * NPS_PER_MS`) and picks the deepest entry in
`DEPTH_NODES` that fits. Measured on the midgame position, startup overhead subtracted (2 ms):

| movetime | depth chosen | nodes | search time | **% of clock used** |
|---|---|---|---|---|
| 100 ms | 4 | 39,018 | 11 ms | **10.7%** |
| 500 ms | 4 | 39,018 | 11 ms | **2.2%** |
| 2,000 ms | 5 | 366,673 | 89 ms | **4.4%** |
| 8,000 ms | 5 | 366,673 | 93 ms | **1.2%** |
| 20,000 ms | 6 | 3,480,703 | 832 ms | **4.2%** |

The engine never uses more than ~11% of the time it is given.

## Two causes, separated

**1. `NPS_PER_MS = 1000` is stale.** Its own comment says *"MEASURED: `search_bench` reports
~1.14–1.25M nps ... 1000 nodes/ms is that figure rounded DOWN"*. Re-measured today on the engine's
own `go` at fixed depth 6 — node counts are identical run to run, so only the time varies — across 5
positions × 5 repeats:

| | min nps | median | worst observed |
|---|---|---|---|
| across all positions | 3.34M–4.38M | 2.37M–4.07M | **2,239,873** |

Even the **worst** observation, taken with two trainers, a 240-game 4PC anchor and a 224-pair
netmatch all live on the box, is **2.24× the assumed figure**. The engine got faster (the
accumulator, the shuffle-division fix) and the constant did not follow.

**2. `DEPTH_NODES` is conservative, and deliberately so.** It over-estimates against every one of 5
positions — d4 1.77×, d5 1.76×, d6 2.52× versus the *worst* measured. That conservatism is a
documented design choice for positional variance, and the calibration position really is more
expensive than any of my five, so **it is not being changed here.**

## The change

`NPS_PER_MS: 1000 -> 2000`. Nothing else.

2000 is below the **worst contended observation** (2,239,873), i.e. rounded DOWN in exactly the
spirit of the original comment. 3000 would exceed it and risk overspending under load, which on this
engine is unsafe: there is no iterative deepening, so an aborted search has no completed root move
and returns `score cp -32000` with a random move (`speed_cannot_pay_RESULT.md` §4, bug 1).

## Why not the obvious bigger fix

The deeper cause of the waste is that depth is a **step function** of the clock — the d5→d6
threshold is a 10× jump, so a budget just under it wastes ~90% by construction. The fix for that is
**iterative deepening**, and MASTER_PLAN line 38 forbids seeding it: *"no ordering, no hash reuse, no
iterative deepening, no quiescence"* — it must be DISCOVERED by the search track. So the discreteness
is a spec-imposed cost, not a defect, and this change only removes the part that is pure stale
calibration.

## Pre-registered predictions

1. **A bare `go` is byte-identical.** No movetime means no budget, so `depth_for_budget` is never
   consulted. If a bare `go` changes at all, the change touched more than the calibration and must be
   reverted. This is the control.
2. **The depth thresholds halve in time.** d5 should arrive at ~620 ms instead of ~1235 ms, and d6 at
   ~6.35 s instead of ~12.7 s.
3. **`go movetime 8000` moves depth 5 → 6**, taking clock use from 1.2% to roughly 25–30%.
4. **The `SAFETY` cap (4× budget) must not fire.** At movetime 8000 the budget becomes 16M nodes and
   the worst measured d6 cost is 5.03M, so the cap has ~3× headroom. If `score cp -32000` appears
   anywhere, the change is unsafe and reverts.
5. **Determinism is preserved.** The search still stops on NODES, never on a clock read, so the same
   seed plays the same game (FITNESS 10).

## What this does NOT claim

No Elo. Every gate, `netmatch` and the ruler run at **fixed depth**, so no measured result in this
repo moves. This changes movetime play only. The honest statement is "the engine now spends the
clock it is given"; whether that converts to strength is what `elo_vs_time.sh` measures, and it has
not been run against this change.

Deployment note: `sf_ruler.py` spawns the SCRATCHPAD engine copy and a ruler game is live, so the
rebuild goes to `target/release/engine` and the deployed copy is swapped only when no match is in
flight — overwriting a binary a match is spawning per game is the arm-swap that voided a 4PC gate.
