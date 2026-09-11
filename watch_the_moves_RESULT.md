# RETRACTED — "the champion plays h7h5" was a harness bug. It plays e2e4.

**2026-09-10, retracted the same evening it was written.** The original version of this file claimed
the champion passed two gates while playing `c2c3` from the opening and `h7h5` in reply to
1.e4 e5 2.Nf3, and concluded the absolute level was low in a way the instruments could not show.

**The engine was never loading the net I thought.** The probe was written as:

```bash
    EXISTENCE_NET=p1_champion.net printf 'uci\nposition ...\ngo\nquit\n' | engine
```

`VAR=x cmd1 | cmd2` sets the variable for **cmd1**. It applied to `printf`, not to the engine, so
every arm silently ran the engine's default `champion.net`. The tell was there and I walked past it:
the engine prints `info string loaded net <path>` on stderr, and it said `champion.net` even when the
path given was `/nonexistent.net`.

## What the correct measurement says

`printf ... | EXISTENCE_NET=<net> engine` — env on the ENGINE:

| position | champion | untrained |
|---|---|---|
| start | **e2e4** | e2e4 |
| 1.e4 e5 2.Nf3 | **d7d6** | h7h6 |
| 1.d4 d5 | e2e4 | e2e4 |
| 1.e4 c5 | **f1b5** | d1f3 |
| 1.Nf3 Nf6 | **e2e4** | a2a3 |
| 1.e4 e6 2.d4 d5 | **b1c3** | d1g4 |

The champion opens **1.e4**, answers 2.Nf3 with **d7d6**, meets the Sicilian with **Bb5**, and
develops with **Nc3**. That is ordinary, reasonable chess. The untrained net plays `h7h6`, `Qf3`,
`a2a3`, `Qg4` — visibly worse. They differ on **4 of 6** positions, so training changes move choice,
which is exactly what the 235 Elo gap between them predicts.

## How it was caught, which is the part worth keeping

The retracted version produced "champion and untrained agree on 10 of 10 positions". That
**contradicted a measured 235 Elo gap** — two nets that play identically cannot differ by 235 Elo.
The contradiction with a known measurement is what exposed the harness, not any re-reading of the
code.

That is the standing rule this repo already carries: *two wrong hypotheses in a row means the harness
is wrong, not the subject.* Here it was one impossible result, and it was enough.

## What survives

* **Watching actual play is still the standard**, and `status.sh` still prints the engine's move
  every check — with the env placement fixed.
* The 4PC half of the same exercise stands and was never affected: a real 462-ply gate game whose
  last sixteen plies are four kings shuffling, sized at 3% of games consuming 13% of plies.
* Nothing about the promotions, the datagen-depth result, or the regression catch depended on this
  file.
