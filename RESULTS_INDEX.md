# Results index — every `*_RESULT.md` headline, newest first

Regenerate with `./index_results.sh`. **Read this before designing an experiment.**
Grep it for the lever you are about to test; the headlines are written to be read alone.

> Two experiments on 2026-09-10 were launched against questions already answered here —
> one burned 95% of its wall clock re-deriving `width_clock_RESULT.md` at the exact
> setting that file abandoned. The cost of checking is one grep.

| result | headline |
|---|---|
| [diversity_reserve_SIZING.md](diversity_reserve_SIZING.md) | the diversity reserve HAS run (19 gens, dsl engaged) and found nothing — expected 88% of the time; a real test needs ~107 generations for 50% |
| [epochs_2v3_RESULT.md](epochs_2v3_RESULT.md) | epochs 2 vs shipped 3 is FLAT at full length (0.513 +/- 0.026, matched 2000/2000) — degrades above 3, flat below, and no cost advantage; epochs CLOSED |
| [hardn_inert_RESULT.md](hardn_inert_RESULT.md) | EXISTENCE_HARD_N is INERT on the code path that runs (hardcoded 8 at evolve.rs:2627) — the n=40 probe measured the default set, and its verdict came from 4 generations where P(zero)=0.74 |
| [gate_candidates_are_game_neutral_RESULT.md](gate_candidates_are_game_neutral_RESULT.md) | the gate's own candidates are 14.7% decisive against 31-42% for two reference programs (z=2.39, z=3.67) — the registered "mate guard selects for game-neutrality" mechanism survived its falsifier |
| [identity_does_not_predict_games_RESULT.md](identity_does_not_predict_games_RESULT.md) | PRE-REGISTERED CONTROL FAILED: hash reuse agrees with the seed on 40/40 positions and still went 8W-33D-7L — position-set identity does NOT imply identical games, and PATH 1 accepts with no game on exactly that inference |
| [gate_arithmetic_RESULT.md](gate_arithmetic_RESULT.md) | the 6-pair game gate CANNOT accept a candidate that draws >=4 of 6 pairs (ceiling 0.48) — and 85.7% of its games are draws; the constraint is PAIRS, not the rule |
| [proposals_choice_RESULT.md](proposals_choice_RESULT.md) | "94% of generations offer no choice" is BINOMIAL ARITHMETIC, not a pathology — and 32 proposals fixes it |
| [horizon_RESULT.md](horizon_RESULT.md) | The horizon cap is OBSOLETE past bootstrap: uncapped beats capped-at-10 by +0.064 ± 0.034 |
| [blend_sweep_RESULT.md](blend_sweep_RESULT.md) | blend 0.85 clears the promotion rule at FULL LENGTH — on one seed, by less than the between-seed spread |
| [ruler_trend_RESULT.md](ruler_trend_RESULT.md) | Every production run is FLAT on the absolute ruler — all 166 Elo came from BETWEEN runs, not within them |
| [low_sweep2_RESULT.md](low_sweep2_RESULT.md) | lr 0.0002 SHIPPED — the shipped rate lost to its own start, on exactly matched arms |
| [low_sweep_RESULT.md](low_sweep_RESULT.md) | Below the shipped rate: 0.0002 wins, 0.0005 LOSES to its own start — on a run that was cut short |
| [lr_decay_RESULT.md](lr_decay_RESULT.md) | The schedule is not the lever — ending low is. lr 0.0005 shipped |
| [static_deep_residual_RESULT.md](static_deep_residual_RESULT.md) | Half of the P1 kill criterion has never been measurable — the static-vs-deep residual, built and refuted |
| [arch_surrogate_filter_RESULT.md](arch_surrogate_filter_RESULT.md) | A third of all ARCH proposals were killed on held-out loss, without ever playing a game |
| [confident_when_wrong_RESULT.md](confident_when_wrong_RESULT.md) | The engine is MORE confident where its cheap search is wrong — FITNESS §8, implemented and failing |
| [learning_rate_is_the_plateau_RESULT.md](learning_rate_is_the_plateau_RESULT.md) | The plateau was the learning rate — 0.692 against 0.499, on a knob never once varied |
| [lr_sweep_RESULT.md](lr_sweep_RESULT.md) | The lr result replicates on a fresh seed and a different start — and the optimum is lower still |
| [promotion_was_sound_RESULT.md](promotion_was_sound_RESULT.md) | The champion's promotion was sound — my suspicion that it was a lucky seed is refuted |
| [nontransitive_walk_RESULT.md](nontransitive_walk_RESULT.md) | The production run IS improving — 0.541 over 2,650 generations, while its 5-generation steps are not |
| [games_per_gen_RESULT.md](games_per_gen_RESULT.md) | Quadrupling the data per generation changes nothing — 0.4642 against 0.4684 |
| [generator_is_net_negative_RESULT.md](generator_is_net_negative_RESULT.md) | Five generations of training make the net WORSE — the gate was never the problem |
| [movegen_leaf_RESULT.md](movegen_leaf_RESULT.md) | A leaf never reads the move list — not building one is a 1.72× speedup |
| [batch_gate_saturated_RESULT.md](batch_gate_saturated_RESULT.md) | The fix for the plateau decides on the instrument that was condemned the same day |
| [resume_dip_RESULT.md](resume_dip_RESULT.md) | A resume costs ~95 Elo before it pays back — and it confounds every short experiment |
| [gated_resume_RESULT.md](gated_resume_RESULT.md) | Unguarded acceptance causes the resume dip — and the gate prevents it by preventing progress |
| [speed_cannot_pay_RESULT.md](speed_cannot_pay_RESULT.md) | The engine cannot spend a speedup at all — and quantization is worth ~5 Elo, not a ply |
| [depth5_vs_depth3_RESULT.md](depth5_vs_depth3_RESULT.md) | Depth 5 datagen LOSES to its own start; depth 3 WINS from the same champion |
| [width_clock_RESULT.md](width_clock_RESULT.md) | Widening never pays on the CLOCK at this engine's speed — 11 attempts, 0 accepted, sign never flips |
| [w64_misspecified_RESULT.md](w64_misspecified_RESULT.md) | The staged capacity experiment would have run at width 16 and reported width 64 |
| [exploit_guard_scale_RESULT.md](exploit_guard_scale_RESULT.md) | The exploit guard switched itself off when the seed was weak — and a captured specimen proves it |
| [watch_the_moves_RESULT.md](watch_the_moves_RESULT.md) | RETRACTED — "the champion plays h7h5" was a harness bug. It plays e2e4. |
| [champion_deep_RESULT.md](champion_deep_RESULT.md) | The deep-datagen net PASSED THE GATE against the champion — 0.622 ± 0.022, and it is now champion |
| [proxies_RESULT.md](proxies_RESULT.md) | Every cheap proxy for strength has failed. Only games measure strength here. |
| [datagen_depth_RESULT.md](datagen_depth_RESULT.md) | Six generations of depth-3 datagen match a champion built from 2,200 — datagen depth IS the lever |
| [label_source_RESULT.md](label_source_RESULT.md) | Better labels ARE learned and do NOT become strength — and 4× the width changes nothing either |
| [gate_power_RESULT.md](gate_power_RESULT.md) | The P2 game gate: why 0 accepts in 203 decisions, and what replaced it |
| [untrained_baseline_RESULT.md](untrained_baseline_RESULT.md) | Training DID work — ~235 Elo, all of it before generation 200, then flat for 1200 generations |
| [elo_per_ply_RESULT.md](elo_per_ply_RESULT.md) | One ply of search is worth ~93 Elo. Twelve hundred generations of training bought ~0. |
| [guard_tolerance_worst_of_both_RESULT.md](guard_tolerance_worst_of_both_RESULT.md) | The shipped guard tolerance admits an exploit AND excludes the target — worst of both |
| [surrogate_inverts_RESULT.md](surrogate_inverts_RESULT.md) | The grammar fitness ranks the STRONGEST reference program LAST — a direct inversion |
| [absolute_ruler_RESULT.md](absolute_ruler_RESULT.md) | The champion is ~1182 Elo and FLAT across 1200 generations — the plateau is real and it is low |
| [simd_refuted_RESULT.md](simd_refuted_RESULT.md) | `target-cpu=native` buys NOTHING — the width-vs-clock result is not a SIMD artifact |
| [depth_transfer_r9_RESULT.md](depth_transfer_r9_RESULT.md) | The depth-1 gain is resolved; at depth 4 nothing is, and two instruments disagree in sign |
| [accept_audit_RESULT.md](accept_audit_RESULT.md) | Accepts sit just above 0.5 at depth 4 — and the call is 0.0006 from resolving. Not a null. |
| [ancestor_first_readings_RESULT.md](ancestor_first_readings_RESULT.md) | First readings from the ancestor control: no measurable gain over 400-generation windows |
| [accept_rate_vs_noise_RESULT.md](accept_rate_vs_noise_RESULT.md) | Accepts are NOT noise: 8.87% observed against a 2.50% false-positive floor (z = +29.1) |
| [pooled_runs_RESULT.md](pooled_runs_RESULT.md) | There was no decline. The origin control's "collapse" was two runs read as one curve |
| [reject_holdout_RESULT.md](reject_holdout_RESULT.md) | REFUTED: the depth-1 gate is NOT discarding depth-4 improvements. The deeper-gate lead is closed. |
| [surrogate_rewards_giving_up_RESULT.md](surrogate_rewards_giving_up_RESULT.md) | The grammar search's fitness surrogate can be improved by SOLVING FEWER POSITIONS |
| [control_caps_RESULT.md](control_caps_RESULT.md) | The origin control's node budget is STABLE to 1.9% — my "moving instrument" explanation is REFUTED |
| [p1_deceleration_RESULT.md](p1_deceleration_RESULT.md) | P1 is still gaining, at ~+24 Elo per 100 generations — and BOTH instruments are now at their limit |
| [decisiveness_RESULT.md](decisiveness_RESULT.md) | Self-play decisiveness FALLS as the champion improves — so it cannot be the horizon's strength signal |
| [depth2x2_RESULT.md](depth2x2_RESULT.md) | Depth is real; parity does not reach trained strength — 2026-09-09 |
| [ladder_valley_RESULT.md](ladder_valley_RESULT.md) | The hash-reuse valley — CURRENT STATE (2026-09-10 05:1x) |
| [bytecode_headroom_RESULT.md](bytecode_headroom_RESULT.md) | CRATE 4's register bytecode cannot be a throughput win: the headroom is ~4% |
| [interp_gate_precision_RESULT.md](interp_gate_precision_RESULT.md) | The GRAMMAR 8 gate PASSES, and "0.98x" was never a three-digit number |
| [incremental_delta_RESULT.md](incremental_delta_RESULT.md) | The accumulator's crossover was a property of its DIFF, not of the width |
| [specfilter_admits_noops_RESULT.md](specfilter_admits_noops_RESULT.md) | SPEC_FILTER admits NO-OPS to the game gate, and spends the whole budget on them |
| [editcount_RESULT.md](editcount_RESULT.md) | The loop DOES install both TT halves, at the pure-draw rate. The barrier is semantic placement. |
| [fitness_set_composition_RESULT.md](fitness_set_composition_RESULT.md) | The fitness set's discriminating power lives ENTIRELY in its non-mate-in-1 positions |
| [path1_is_empty_RESULT.md](path1_is_empty_RESULT.md) | PATH 1 (behaviour-preserving speedup) is CORRECT and EMPTY |
| [search_has_no_choice_RESULT.md](search_has_no_choice_RESULT.md) | The search almost never has a CHOICE: 94% of generations offer ≤1 distinct fitness |
| [eps_is_inert_RESULT.md](eps_is_inert_RESULT.md) | EPS 0.10 is INERT BY CONSTRUCTION: the band it buys is empty |
| [fitness_saturation_RESULT.md](fitness_saturation_RESULT.md) | ⚠⚠ TWO CORRECTIONS, 20:50 and 21:02. The tolerance is a DILEMMA, not a dial. |
| [gate_bounds_RESULT.md](gate_bounds_RESULT.md) | Gate bounds: the WIDTH was the cost driver, not the stopping rule |
| [eps_widen_RESULT.md](eps_widen_RESULT.md) | EPS 0.020 → 0.100: the knob is LIVE, and it retains TT-carrying members |
| [blend_RESULT.md](blend_RESULT.md) | The training target's blend is the highest-leverage knob measured, and the blind metric compressed it 3× |
| [throughput_RESULT.md](throughput_RESULT.md) | Eval is NOT the throughput bottleneck at the champion's width — measured 2026-09-09 |
| [instrument_saturation_RESULT.md](instrument_saturation_RESULT.md) | The frozen-origin metric saturates, and it has now reversed two signs |
| [acceptance_floor_RESULT.md](acceptance_floor_RESULT.md) | The gate demands an edge 2.7× larger than a generation produces — and the experiment that cleared it used a broken ruler |
| [tolerance_RESULT.md](tolerance_RESULT.md) | A guard tolerance of 4 separates genuine changes from known exploits — on the TARGETED guard only |
| [depth_RESULT.md](depth_RESULT.md) | Deeper labels beat more labels at equal compute — PROMISING, needs replication |
| [rung6_RESULT.md](rung6_RESULT.md) | Rung 6 was dead code. Now it plays differently — whether it plays better is UNRESOLVED. |
| [draws_RESULT.md](draws_RESULT.md) | Excluding draws is BETTER — REFUTED, resolved |
| [width_RESULT.md](width_RESULT.md) | Width is NOT the ceiling — REFUTED, resolved |
| [epochs_ab_RESULT.md](epochs_ab_RESULT.md) | epochs A/B — RESULT (2026-09-08): under-fitting REFUTED, the labels are the problem |
| [ladder_RESULT.md](ladder_RESULT.md) | GRAMMAR 9 ladder, re-run with the repaired reference programs (2026-09-08) |
| [gate_ab_RESULT.md](gate_ab_RESULT.md) | gate resolution A/B — RESULT (2026-09-08) |

## Findings that are not `*_RESULT.md`

Same standing as the table above. `search_track_WHY_NOTHING.md` is the file
`surrogate_inverts_RESULT.md` records re-deriving from scratch at a cost of hours --
it was never indexed because of its name.

| file | headline |
|---|---|
| [NET_TRACK_STATE.md](NET_TRACK_STATE.md) | The NET track, summarised across every configuration tried (2026-09-08) |
| [ceiling_ANALYSIS.md](ceiling_ANALYSIS.md) | The plateau is the TRAINING PROCEDURE'S CEILING, not a gating failure |
| [search_track_WHY_NOTHING.md](search_track_WHY_NOTHING.md) | Why the MAIN lineage has never produced an improvement, and structurally cannot |
| [EXPERIMENTS.md](EXPERIMENTS.md) | EXPERIMENTS — what was tried, and why it failed |
| [fitness_spec_gap_FINDING.md](fitness_spec_gap_FINDING.md) | The surrogate is not the one FITNESS §3 specifies — and that explains the saturation |
| [search_track_FINDING.md](search_track_FINDING.md) | The search track's surrogate rewarded searching LESS (2026-09-08) |
| [surrogate_validation.md](surrogate_validation.md) | The accept/reject surrogate does not predict strength — measured (2026-09-08) |
| [replay_ab_CAVEAT.md](replay_ab_CAVEAT.md) | replay_ab.sh — read the arms correctly (noted 2026-09-08, DURING the run) |

## Topic index — which lever has already been studied

A headline can only carry so much. On 2026-09-11 I re-derived `proxies_RESULT.md` (held-out
surrogate r=-0.095 over 239 gate results) because I grepped this index for "blend" when
designing that experiment, and never for "loss". The headline index answers *is there a file
about X*; this one answers *has anyone measured X*, which is the question that was actually
being asked.

| topic | files that measure it |
|---|---|
| learning rate | lr_decay,static_deep_residual lr_sweep,learning_rate_is_the_plateau NET_TRACK_STATE |
| training loss | proxies,EXPERIMENTS arch_surrogate_filter,replay_ab_CAVEAT epochs_ab |
| surrogate/proxy | EXPERIMENTS,fitness_saturation surrogate_inverts,surrogate_validation specfilter_admits_noops |
| net width | EXPERIMENTS,ladder_valley throughput,width_clock search_track_WHY_NOTHING |
| search depth | EXPERIMENTS,depth2x2 blend,speed_cannot_pay depth5_vs_depth3 |
| blend/target | EXPERIMENTS,blend blend_sweep,NET_TRACK_STATE depth2x2 |
| epochs | EXPERIMENTS,epochs_ab NET_TRACK_STATE,learning_rate_is_the_plateau label_source |
| seeds & noise | depth2x2,blend p1_deceleration,blend_sweep low_sweep2 |
| gate & thresholds | gate_power,EXPERIMENTS replay_ab_CAVEAT,ladder_valley search_track_WHY_NOTHING |
| calibration | static_deep_residual,confident_when_wrong search_track_FINDING,movegen_leaf lr_sweep |
| speed/nps | speed_cannot_pay,throughput movegen_leaf,depth2x2 path1_is_empty |
| plateau | EXPERIMENTS,ceiling_ANALYSIS ruler_trend,learning_rate_is_the_plateau fitness_saturation |

## Non-`_RESULT` files worth knowing

| file | what it is |
|---|---|
| [docs/MASTER_PLAN.md](docs/MASTER_PLAN.md) | Existence — Master Plan |
| [refmatch_discrimination_PREREG.md](refmatch_discrimination_PREREG.md) | PRE-REGISTRATION: can the gate's games discriminate at all — predictions, falsifier and a registered mechanism, all written before the numbers existed. Its 14.3% baseline is deliberately NOT updated |
| [low_sweep2_PREREG.md](low_sweep2_PREREG.md) | PRE-REGISTRATION for the full-length low-lr sweep, committed before any verdict existed |
| [STATE.md](STATE.md) | Existence — current state, 2026-09-11: measured champion strength, the search track's blocker, and the position-set/game decoupling |
