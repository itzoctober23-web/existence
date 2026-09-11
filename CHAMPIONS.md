# Champion lineage

Every champion this project has promoted, with the measurement that promoted it. One row per
promotion, newest last. **A row is only added when the candidate passed the project's acceptance
rule** — `rate − ci95 ≥ 0.5` on `netmatch`, 448 pairs at depth 4, which is the strength standard
`gate_depth_cap` declares.

| # | file | vs | score | interval | passed |
|---|---|---|---|---|---|
| 0 | `p1_champion_pre_deep.net` | — | — | — | the ~2,200-generation champion trained on **depth-1** labels; absolute ~1216 on the SF ruler |
| 1 | `p1_champion_gen1045.net` | #0 | **0.622 ± 0.022** | [0.599, 0.644] | yes — cleared the bar by 0.099 |
| 2 | `p1_champion_prev_pre_lr002.net` (was "current") | #1 | **0.586 ± 0.020** | [0.566, 0.606] | yes |
| 3 | `p1_champion_prev_pre_lr0005.net` — md5 `dfd258b07026` | #2 | **0.555 ± 0.028** | [0.527, 0.583] | yes — first lr 0.002 net |
| 4 | `p1_champion_prev_g2052.net` — md5 `91c6eb472d04` | #3 | **0.628 ± 0.027** | [0.601, 0.656] | yes — first lr 0.0005 net |
| 5 | `p1_champion.net` (current) — md5 `34a5ace752d8` | #4 | **0.531 ± 0.027** | [0.504, 0.558] | yes — banked by `auto_promote` |

> **Rows 3-5 all landed on 2026-09-11**, and they are the learning-rate result arriving in the
> lineage. #3 is the first net trained at `lr 0.002` and #4 the first at `lr 0.0005`; #5 is 2,052
> generations of production at 0.0005 on top of #4, promoted automatically at 03:48 on the same rule
> everything else here uses (0.531 − 0.027 = 0.504 ≥ 0.5). See `lr_sweep_RESULT.md` and
> `lr_decay_RESULT.md`.
>
> **Rows are keyed by md5 from #3 onward, and that is a fix, not decoration.** Row 2 read
> "`p1_champion.net` (current)" and was silently wrong within hours: `p1_champion.net` is a MUTABLE
> path that three different nets have occupied today. A lineage table whose primary key is a filename
> that gets overwritten records nothing durable. The previous occupant is preserved on each promotion
> — by hand as `p1_champion_prev_pre_*` and by `auto_promote.sh` as `p1_champion_prev_g<gen>.net` —
> so every row above is still on disk and checkable by hash.

## What changed between them

**Only the datagen search depth.** Same width 16, same trainer, same blend, same seed family.
`main.rs:152` defaulted `--depth` to **1**, so champion #0 — and every P1 measurement that preceded
it, including the ~1216 plateau, the deceleration curve and the width/blend/horizon sweeps — was
trained on labels produced by a ONE-PLY search. Raising it to 3 is the whole difference.
`datagen_depth_RESULT.md` has the equal-clock comparison that found it.

## Reading the margins

0.622 then 0.586. Both passed, but the second is smaller against a stronger opponent, which is what
deceleration looks like rather than a problem. `depth5_from_champion.sh` is staged to ask whether the
same lever has more left in it, and carries a pre-registered prediction that it does **not** —
depth 3 → 5 costs 35× more per label, and depth-3 labels are already informative, so it must beat 35×
fewer training steps.

## Two things this table deliberately does not contain

* **No ruler numbers as evidence of promotion.** The absolute ruler carries ±50 Elo at 120 games and
  produced a four-reading "decline" while the net was genuinely stronger. It answers *roughly where
  is this net*; it cannot answer *is it moving*. Promotion is decided by the paired instrument only.
* **No "+N Elo" for anything unshipped.** A row here means a gate was passed, which is a different
  and smaller claim than an Elo attribution.

Promotions are now banked automatically by `auto_promote.sh` on the same rule, so a gain no longer
waits for someone to run the match by hand.
