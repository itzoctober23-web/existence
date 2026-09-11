# The champion passes gates and plays bad chess — watch the moves, not the Elo

**2026-09-10.** Two promotions today, both clean on the paired instrument (0.622 ± 0.022, then
0.586 ± 0.020, 448 pairs at depth 4). The absolute ruler reads ~1300. Then I actually looked at what
it plays:

| position | champion's move | verdict |
|---|---|---|
| start | **c2c3** | passive; not a developing move |
| 1.e4 e5 2.Nf3 | **h7h5** | a rook-pawn lunge no reasonable player makes |
| 1.d4 d5 2.c4 | **d5c4** | at least a real reply (accepts the gambit) |

**h7h5 is the finding.** It is not a subtle positional error; it is the kind of move that says the
net has no idea what the position wants. And it is played by a net that has beaten its predecessor
twice on 448-pair matches.

## Why the gates did not catch it

The gates are **relative**. They ask "is this net better than that net", and both nets can be bad.
The absolute ruler is anchored, but at ±50 Elo per 120-game sample it cannot see a single bad move —
it sees an aggregate over thousands of positions, most of which are not the ones where the engine
embarrasses itself.

Everything measured today is consistent with this and none of it revealed it:

* depth-1 → depth-3 labels: +128 Elo, real, and the resulting engine still plays h7h5.
* two passed gates: real, and both were "better than a net that also played badly".
* rising training loss: explained as a moving target, and it is — but a well-fit bad target is still
  a bad target.

## The standing change

**`status.sh` now prints what the engine plays on three fixed positions, every check.** Liveness and
CPU% say a process exists; they say nothing about whether it plays sensible chess. This is the same
lesson the Rocket League work records as *watch the games, not the stats* — LLR, income and entropy
all lie, and the only ground truth is what the thing actually does.

## What this does NOT mean

* It does not retract the promotions. Both are correctly measured relative improvements.
* It does not mean the datagen-depth result is wrong. +128 Elo is +128 Elo.
* It means **the absolute level is low and the instruments in use cannot show that** — which is
  exactly why MASTER_PLAN's P1 milestone is stated against an external opponent (~2000 vs SF-limited)
  and not against the project's own history.

The engine is ~1300 and plays like it. The measurements were all correct; the thing they were
measuring was further from good than any of them could say.
