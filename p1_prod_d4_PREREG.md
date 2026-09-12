# PREREG — P1 production at fixed-depth-4 datagen (the one permitted Existence run)

**Scope sentence (a):** it can move the primary metric — pooled Elo on the SF ruler — because datagen depth 1→3 was the largest measured lever (+128 ± 72 on the ruler) and depth 4 from the current champion has never been run to a planned N with rungs on the ruler.

Registered 2026-09-12 before launch. Nothing else in this repo runs.

| item | value |
|---|---|
| start net | `p1_champion.net` md5 `bc0b165b28a79c9d6ed80b8dfd67c19f` (copied to `prod_d4_start.net` at launch) |
| binary | `bin/learn_prod` md5 bb7ad37d93a9379a0469af0d7c08d39f (the production build every prodk reading used) |
| datagen | `--depth 4`, `--games 8` per generation, `--threads 1` (one pinned core, cpu 11), `--epochs 3`, `--lr 0.0002 --lr-decay 1.0 --blend 0.85`, gate/arch/control off, `--seed 20260910` |
| planned N | 300 generations; rungs every 50 (`--rung-every 50`), i.e. gens 50,100,150,200,250,300 |
| ruler per rung | `sf_ruler.py --depth 4 --sf-elo 1320 --sf-nodes 10000 --games 120`, one 120-game sample per rung appended to `live_ruler.out` (pooled by `ruler_pool.py`) |
| control | the start net itself on the same ruler (gen 0 rung), same settings |
| result | `p1_prod_d4_RESULT.md`, written only when gen 300 and all 7 ruler readings exist; states every rung ± CI, the across-rung pool, and the slope with its error |
| reading rule | fixed in advance: a rung differing from the control by less than the combined SE is not movement; the run is called FLAT unless the slope over rungs is significant at z ≥ 2 |
| relaunch rule | a watchdog relaunch restarts from the champion under a new tag; only a launch that completes all 300 generations counts |
