# Better labels ARE learned and do NOT become strength — and 4× the width changes nothing either

> **HEADLINE CORRECTED 2026-09-10, by the experiment in this same file.** This was first titled
> *"…the bottleneck is capacity, not the label"*, written after the w16 row and before the w64 row
> existed. The 2×2 then measured **zero** effect from quadrupling width, which refutes it. The
> original title is left visible here rather than quietly swapped, because a headline written one
> section ahead of its evidence is exactly the failure this repo keeps recording.

**2026-09-10.** The second of the two controls: same positions, same trainer, same architecture, same
epochs, same init seed — **only the label source differs.**

```text
  ARM self : target = tanh(loop_root_cp / 600)     the loop's own depth-1 root
  ARM sf   : target = tanh(sf_cp        / 600)     Stockfish @10k nodes on the SAME position
```

`blend = 1.0` in both arms, so the outcome term `z` is switched off and nothing but the label moves.
Both arms train on an identical row set: `dump_positions` recorded the loop's own root per position,
so the two arms are literally one column apart and no re-generation was involved.

## The pre-registered decision rule, and why the answer is neither branch

The rule was: *jumps to 1600+ → the pipeline works and the labels are the problem; sits at ~1182 →
the net/trainer itself is broken.*

### On the ruler — depth 4, SF-1320 @10k nodes, 120 games, the same instrument throughout

| net | W-D-L | score | Elo vs SF-1320 | absolute |
|---|---|---|---|---|
| untrained w16 | 0-26-94 | 0.1083 | −366 ± 67 | ~954 |
| **ARM self** | 8-22-90 | 0.1583 | **−290 ± 69** | ~1030 |
| **ARM sf** | 7-34-79 | 0.2000 | **−241 ± 58** | ~1079 |
| champion (gen ~2200) | 22-41-57 | 0.3542 | −104 ± 52 | ~1216 |

No jump. SF labels are worth **+49 Elo** over their matched control, against intervals of ±58 and
±69 — **unresolved**, and nowhere near 1600.

**But the games alone cannot close this**, and saying "labels don't help" on that evidence would be
the mistake this repo keeps recording. Both arms land *below* the champion, so they share a limit the
game comparison cannot see. A one-shot 12-epoch train that was simply too short would produce exactly
this picture while saying nothing whatsoever about labels.

### The measurement that separates them, and costs no games

Held out 10% of positions, split **by FEN hash rather than by file position** — consecutive rows are
plies of the same game, so a prefix/suffix split would leak the tail of every training game into the
holdout and report a flattering number.

| net | held-out MSE vs **SF** label | vs **self** label |
|---|---|---|
| ARM self | 0.09605 | **0.01935** |
| ARM sf | **0.04711** | 0.08376 |
| untrained | 0.52474 | 0.56622 |

Each arm fits its own label best, and both beat the untrained reference by **11× and 27×**.

**So the trainer is not broken and the arms are not undertrained.** The SF arm demonstrably *learned*
Stockfish's eval — an 11× improvement in held-out fit over random — and converted that into +49 Elo
that does not resolve. The learner works; better labels were absorbed; strength did not follow.

## The finding hiding in the same table: SF's eval is 2.4× HARDER for w16 to fit

The SF arm reaches 0.04711 on its own label. The self arm reaches 0.01935 on its own. Same net, same
epochs, same optimiser, same positions — **the only difference is which function is being
approximated**, and Stockfish's is 2.4× harder for a 16-wide net to represent.

> **⚠ THE INFERENCE BELOW WAS WRONG, and the 2×2 in the next section is what refuted it.** I read
> "SF's label is 2.4× harder to fit" as a CAPACITY signature and predicted more width would help.
> Measured: w64 improves held-out SF fit by only 12% (0.04711 → 0.04163), improves the self label not
> at all, and buys **exactly zero Elo**. The 2.4× is real; reading it as "too few parameters" was not.
> A label can be harder to fit because it depends on information the FEATURES do not carry, and no
> amount of width recovers information that was never in the input. The paragraph is kept because the
> reasoning is a fair trap and the correction is the useful part.

That looked like a capacity signature, and it puts a previously-settled result back in play. `width_RESULT.md`
records "Width is NOT the ceiling — REFUTED, resolved", and `width_clock_RESULT.md` records 11
widening attempts with the sign never turning. **Both were measured under the loop's own labels** —
and this table says those labels are the *easy* ones, already nearly saturated by w16 at 0.019. Under
a label a 16-wide net can already fit, widening cannot pay: there is no residual error for the extra
capacity to remove. The measurements are correct; the conclusion drawn from them ("width is not the
ceiling") was generalised past the regime it was taken in. That is the same shape as
`right-measurement-wrong-conclusion`, and it is why this file does not claim width IS the ceiling
either — only that the existing evidence cannot speak to the case where the label is hard.

## The 2×2 has now RUN, and the pre-registered prediction was right

`w64_prereg.md` was written while the games were in flight and predicted both w64 cells would land
within noise of their w16 counterparts, because held-out fit had already said the arms were
data-limited rather than capacity-limited. Measured:

| Elo vs SF-1320 (120 games each) | self label | SF label |
|---|---|---|
| **w16** | −290 ± 69 | −241 ± 58 |
| **w64** | −290 ± 66 | −241 ± 61 |

**Quadrupling width changed the score by exactly zero** — 0.1583 and 0.2000 in both rows, to four
decimal places.

That double coincidence is the kind of thing that is usually a broken harness, so it was checked
before being believed: the w64 files are 4× larger (200,470 vs 50,134 bytes), all four md5s differ,
the W-D-L compositions differ (8-22-90 vs 6-26-88; 7-34-79 vs 9-30-81, so different games really were
played), and w64 is measurably slower per match (181s/206s against 162s/179s), as a bigger net at
fixed depth must be. The engines were genuinely different. The aggregate scores coincided.

Fixed depth is the right instrument here and not a confound: our engine ignores go parameters and
searches to its `Depth` option, so every cell gets the **same search tree** and differs only in eval
quality. A wider net costs more wall-clock per node and buys no extra nodes.

### Pooling the two widths, because the label effect replicated

The same +49 appears at both widths, so the two are independent replications of one effect and can be
pooled:

| arm | W-D-L | n | score | Elo |
|---|---|---|---|---|
| self, pooled | 14-48-178 | 240 | 0.1583 | −290 ± 48 |
| sf, pooled | 16-64-160 | 240 | 0.2000 | −241 ± 42 |

**SF label advantage over 480 games: +49 ± 64. Still unresolved** — the true value lies somewhere in
[−15, +113] Elo. It is not zero-with-confidence and it is not the ~400 Elo that "jumps to 1600+"
would have required.

## Verdict

Against the pre-registered rule, the answer is **neither branch**:

* **The net/trainer is not broken.** Held-out fit improves 11–27× over untrained, and each arm fits
  its own label best. The learner absorbs whatever it is given.
* **The self-play labels are not the binding constraint either.** Substituting a genuinely better
  label — one the net demonstrably learned — buys at most ~113 Elo and probably far less, against a
  ~800 Elo gap to the P1 milestone.

What is left is the regime itself. Every arm here trains on **18,188 positions**, and the champion —
built from far more data over 2,200 generations — reaches 1216 while the best arm here reaches ~1079.
The whole 2×2 sits in a data-starved regime, and that is the honest limit of what it can conclude:
the *comparison between arms* is clean because they share the data, but "labels don't help" is
established **at this data volume**, not at the loop's.

## Standing constraint

Stockfish is METHODOLOGY here, the same standing as the absolute ruler and the perft oracle. Every net
in this file is a diagnostic artefact. None may be shipped, gated, or fed back into the loop, and no
Stockfish-derived label may become training input for the real engine.
