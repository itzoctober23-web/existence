# Results index — every `*_RESULT.md` headline, newest first

Regenerate with `./index_results.sh`. **Read this before designing an experiment.**
Grep it for the lever you are about to test; the headlines are written to be read alone.

> Two experiments on 2026-09-10 were launched against questions already answered here —
> one burned 95% of its wall clock re-deriving `width_clock_RESULT.md` at the exact
> setting that file abandoned. The cost of checking is one grep.

> **P2 / SEARCH TRACK — the binding constraint is the GENERATOR, not the gate.** Several of
> the newest entries below are gate measurements, and read top-down they invite more gate
> work. They are downstream. `gate_power_RESULT.md` settles the gate: the SPRT gate is built,
> wired, and resolves in 37 games; more pairs buy more draws; a trained net would defeat the
> track's purpose. That file records TWO occasions of cores spent re-testing SELECTION after
> the measurement had moved the problem to GENERATION. On 2026-09-11 a third was begun and
> stopped at the audit. Next P2 work is GRAMMAR 4 — the type checker and mutation operators.

| result | headline |
|---|---|
| [budget_label_channel_RESULT.md](budget_label_channel_RESULT.md) | The node budget's LABEL channel is null — 0.4960 with 47.8% of labels changed, against a −43 Elo full effect |
| [champion_absolute_RESULT.md](champion_absolute_RESULT.md) | UNRESOLVED at 600 games a side — and the within-run OLS trend I cited as evidence is REFUTED |
| [cell_c_label_move_mismatch_RESULT.md](cell_c_label_move_mismatch_RESULT.md) | Cell C is the weakest of the three arms — and the permitted reading is the one fixed in advance: decoupling the label from the played move is harmful |
| [candidate_a_budget_loses_RESULT.md](candidate_a_budget_loses_RESULT.md) | Candidate A: the node-budget arm LOSES to fixed depth — 0.4383 over 672 pairs, and the registered mechanism was backwards |
| [budget_makes_games_decisive_RESULT.md](budget_makes_games_decisive_RESULT.md) | The node budget makes self-play 14.5 points more DECISIVE and yields 27% more usable rows — a third channel the pre-registration never named |
| [budget_realised_depth_RESULT.md](budget_realised_depth_RESULT.md) | The node-budget gate PASSES — but the budget reallocates effort in the OPPOSITE direction to the one pre-registered |
| [datagen_node_census_RESULT.md](datagen_node_census_RESULT.md) | Depth-3 datagen costs 5,269 nodes/move, not 10,309 — and effort per position varies 18-25x |
| [eval_agreement_tracks_games_RESULT.md](eval_agreement_tracks_games_RESULT.md) | The first proxy in this project to reproduce a game-measured ranking — eval-agreement, ρ = +1.000 on n=4 |
| [between_vs_within_RESULT.md](between_vs_within_RESULT.md) | The week's central claim, reproduced on a THIRD instrument: agreement rises BETWEEN runs (t = +8.3) and is FLAT WITHIN one (t = +0.46) |
| [p1_kill_conjunct2_RESULT.md](p1_kill_conjunct2_RESULT.md) | P1 kill, conjunct 2 MEASURED for the first time: the static-vs-deep agreement is FLAT. Both conjuncts now hold — and the kill should NOT be fired. |
| [promo_g39836_RESULT.md](promo_g39836_RESULT.md) | The gen-39836 promotion does NOT replicate — the effect falls from +0.039 to +0.001 at 2x the pairs |
| [p2_fitness_RESULT.md](p2_fitness_RESULT.md) | Growing `disagreement_set` made candidates LESS decisive, not more — pre-registered FAIL |
| [prop_gens40_RESULT.md](prop_gens40_RESULT.md) | P2 `search_long_run` at its planned 40 generations: 0 accepts — and the MCTS gate could not have produced any other number |
| [p1_compounding_RESULT.md](p1_compounding_RESULT.md) | P1 compounding did NOT pass — and the result does NOT refute the compounding shape |
| [unc_head_fitted_RESULT.md](unc_head_fitted_RESULT.md) | The uncertainty head is FITTED, and the in-engine head reproduces the offline ranking EXACTLY |
| [gate_candidates_are_game_neutral_RESULT.md](gate_candidates_are_game_neutral_RESULT.md) | The gate's own candidates are anomalously game-neutral — 14.7% decisive against 31-42% for reference programs |
| [unc_signal_is_inverted_RESULT.md](unc_signal_is_inverted_RESULT.md) | The uncertainty signal is INVERTED where it exists at all — and it does NOT exist on every net |
| [flip_cost_concentration_RESULT.md](flip_cost_concentration_RESULT.md) | The prize is concentrated — 38% of all flip cost sits in the top decile — and the proposed key does not fit it |
| [yardstick_reachability_RESULT.md](yardstick_reachability_RESULT.md) | The nearest decision-theoretic shape is the one nothing can build, and the farthest is reachable |
| [add_fn_drops_writes_RESULT.md](add_fn_drops_writes_RESULT.md) | `Op::AddFn` was NOT behaviour-preserving — it lifted alpha-beta's score update and dropped the write, playing a different move 91x cheaper. Found, root-caused and fixed 2026-09-11 |
| [gate_arithmetic_RESULT.md](gate_arithmetic_RESULT.md) | The 6-pair game gate cannot accept a candidate that draws, and 85.3% of its games are draws — this is arithmetic about the 6-pair RULE and is NOT an argument for raising `gate_pairs`, which `gate_power_RESULT.md` measured as buying more draws |
| [gate_power_RESULT.md](gate_power_RESULT.md) | The P2 game gate: 0 accepts in 203 decisions — the GATE is settled, and the binding constraint is UPSTREAM: the GENERATOR has no gradient. Raising `gate_pairs`, swapping in a trained net, and any further acceptance rule are all MEASURED WRONG. Next P2 work is GRAMMAR 4 (type checker + mutation operators) |
| [identity_does_not_predict_games_RESULT.md](identity_does_not_predict_games_RESULT.md) | Position-set identity does NOT imply identical games — measured, and PATH 1 rests on the inference |
| [blend_sweep_RESULT.md](blend_sweep_RESULT.md) | blend 0.85 clears the promotion rule at FULL LENGTH — on one seed, by less than the between-seed spread |
| [hardn_inert_RESULT.md](hardn_inert_RESULT.md) | EXISTENCE_HARD_N was inert on the code path that runs, and an experiment concluded from it |
| [epochs_2v3_RESULT.md](epochs_2v3_RESULT.md) | epochs 2 vs the shipped 3 — FLAT at full length, in both directions and on cost |
| [proposals_choice_RESULT.md](proposals_choice_RESULT.md) | "94% of generations offer no choice" is BINOMIAL ARITHMETIC, not a pathology — and 32 proposals fixes it |
| [horizon_RESULT.md](horizon_RESULT.md) | The horizon cap is OBSOLETE past bootstrap: uncapped beats capped-at-10 by +0.064 ± 0.034 |
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
| [promotion_ladder_decay_FINDING.md](promotion_ladder_decay_FINDING.md) | The promotion margin is constant at ~0.535 while the absolute step has decayed to zero |
| [champion_absolute_PREREG.md](champion_absolute_PREREG.md) | Two sound instruments disagree about the same net: does the promotion ladder track ABSOLUTE strength? |
| [prodk0127_plateau_STATUS.md](prodk0127_plateau_STATUS.md) | The fresh lineage reproduces the plateau: 1561 pooled, FLAT, 39 short of the day-7 line |
| [auto_promote_confirm_untested_STATUS.md](auto_promote_confirm_untested_STATUS.md) | The promotion confirmation stage is ARMED but has never fired — status, not a success claim |
| [cell_c_transitivity_FINDING.md](cell_c_transitivity_FINDING.md) | The three arms form a consistent ranking A > B > C — a transitivity check the instrument could have failed |
| [cell_c_interpretation_PREREG.md](cell_c_interpretation_PREREG.md) | How to read cell C — written with ONE of two seeds in, before the second lands |
| [candidate_a_replication_PREREG.md](candidate_a_replication_PREREG.md) | PRE-REGISTRATION — a SECOND training seed for Candidate A, written before the arms run |
| [structural_track_exhausted_STATUS.md](structural_track_exhausted_STATUS.md) | All three registered structural candidates are now answered — and they point the same way |
| [structural_next_PREREG.md](structural_next_PREREG.md) | PRE-REGISTRATION — what the loop does after the configuration wins run out |
| [replay_window_plateau_FINDING.md](replay_window_plateau_FINDING.md) | Candidate B is NOT "genuinely unmeasured" — a plateau-regime replay sweep ran on 2026-09-08 and returned a null |
| [candidate_a_channel_FINDING.md](candidate_a_channel_FINDING.md) | Candidate A's label channel is LIVE, but it is the channel `label_source` measured as null — the arm needs a third cell |
| [cand_arm_harness_NOTES.md](cand_arm_harness_NOTES.md) | Harness facts for the Candidate A arms, checked rather than assumed |
| [generation_is_not_a_unit_FINDING.md](generation_is_not_a_unit_FINDING.md) | "Generation" means 8 games in one result and 2,400 in another — a 300x unit gap that made Candidate A look infeasible |
| [p1_kill_criterion_STATUS.md](p1_kill_criterion_STATUS.md) | The P1 kill criterion: conjunct 1 is now SATISFIED. Conjunct 2 is measurable but not yet measured. |
| [WEEK1_RETRO.md](WEEK1_RETRO.md) | WEEK 1 RETRO — which lever failed, and why |
| [grammar4_addfn_unpark_blocker.md](grammar4_addfn_unpark_blocker.md) | The AddFn unpark condition is blocked by a SECOND ordering bias — the function sweep short-circuits |
| [STATE.md](STATE.md) | Existence — current state, 2026-09-11 |
| [p2_fitness_PREREG.md](p2_fitness_PREREG.md) | PRE-REGISTRATION — the P2 fitness change: games PRIMARY, mates FILTER |
| [p1_compounding_PREREG.md](p1_compounding_PREREG.md) | PRE-REGISTRATION — P1 compounding: champion-following datagen at a node budget, into a declared |
| [uncertainty_target_PREREG.md](uncertainty_target_PREREG.md) | PRE-REGISTRATION — the uncertainty head's TARGET, before either track is built on it |
| [diversity_reserve_SIZING.md](diversity_reserve_SIZING.md) | The diversity reserve has been run, and the run was far too short to mean anything |
| [refmatch_discrimination_PREREG.md](refmatch_discrimination_PREREG.md) | PRE-REGISTRATION — can the gate's games discriminate at all? (written before the results exist) |
| [low_sweep2_PREREG.md](low_sweep2_PREREG.md) | Pre-registration: the full-length low-lr sweep tests a claim the truncated run made |
| [CHAMPIONS.md](CHAMPIONS.md) | Champion lineage |
| [NET_TRACK_STATE.md](NET_TRACK_STATE.md) | The NET track, summarised across every configuration tried (2026-09-08) |
| [ceiling_ANALYSIS.md](ceiling_ANALYSIS.md) | The plateau is the TRAINING PROCEDURE'S CEILING, not a gating failure |
| [lr_sweep_PREREGISTRATION.md](lr_sweep_PREREGISTRATION.md) | Pre-registration: what the game-free instrument predicts for the lr sweep |
| [gated_resume_PREREG.md](gated_resume_PREREG.md) | PRE-REGISTRATION: does the resume dip survive a live strength gate? |
| [resume_dip_PREREG.md](resume_dip_PREREG.md) | PRE-REGISTRATION: does a resume damage the champion, and if so WHEN? |
| [nps_calibration_PREREG.md](nps_calibration_PREREG.md) | PRE-REGISTRATION: the engine spends 1–11% of its clock, and `NPS_PER_MS` is stale by ~2× |
| [depth5_vs_depth3_PREREG.md](depth5_vs_depth3_PREREG.md) | PRE-REGISTRATION — depth 5 vs depth 3, matched start, equal wall clock |
| [depth_ruler_PREREG.md](depth_ruler_PREREG.md) | PRE-REGISTRATION — datagen depth on the ruler, written before the games were read |
| [scopefix_prereg.md](scopefix_prereg.md) | PRE-REGISTRATION — P2 restarted with the scope fix, written before the log was read |
| [w64_prereg.md](w64_prereg.md) | PRE-REGISTRATION — the w64 cells, written before the games were read |
| [search_track_WHY_NOTHING.md](search_track_WHY_NOTHING.md) | Why the MAIN lineage has never produced an improvement, and structurally cannot |
| [depth_transfer_window_note.md](depth_transfer_window_note.md) | Before reading the depth-4 arm: the two windows are NOT the same size |
| [reject_holdout_PREREG.md](reject_holdout_PREREG.md) | PRE-REGISTRATION — does the depth-1 gate reject real improvements? Holdout test |
| [editcount_power_PREREG.md](editcount_power_PREREG.md) | Pre-registration: what the edit-count sweep CAN and CANNOT resolve |
| [EXPERIMENTS.md](EXPERIMENTS.md) | EXPERIMENTS — what was tried, and why it failed |
| [fitness_spec_gap_FINDING.md](fitness_spec_gap_FINDING.md) | The surrogate is not the one FITNESS §3 specifies — and that explains the saturation |
| [pipeline_ALIVE.md](pipeline_ALIVE.md) | The search track's pipeline runs end to end for the first time |
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
| learning rate | lr_decay,STATE static_deep_residual,lr_sweep learning_rate_is_the_plateau |
| training loss | proxies,EXPERIMENTS arch_surrogate_filter,replay_ab_CAVEAT epochs_ab |
| surrogate/proxy | STATE,EXPERIMENTS fitness_saturation,surrogate_inverts surrogate_validation |
| net width | STATE,EXPERIMENTS ladder_valley,throughput width_clock |
| search depth | STATE,EXPERIMENTS depth2x2,blend speed_cannot_pay |
| blend/target | STATE,EXPERIMENTS blend,blend_sweep NET_TRACK_STATE |
| epochs | STATE,EXPERIMENTS epochs_ab,epochs_2v3 NET_TRACK_STATE |
| seeds & noise | STATE,promo_g39836 depth2x2,blend p1_deceleration |
| gate & thresholds | STATE,gate_power gate_arithmetic,EXPERIMENTS replay_ab_CAVEAT |
| calibration | static_deep_residual,unc_signal_is_inverted STATE,uncertainty_target_PREREG confident_when_wrong |
| speed/nps | STATE,speed_cannot_pay throughput,movegen_leaf depth2x2 |
| plateau | STATE,EXPERIMENTS WEEK1_RETRO,p1_kill_conjunct2 ceiling_ANALYSIS |

## Non-`_RESULT` files worth knowing

| file | what it is |
|---|---|
| [docs/MASTER_PLAN.md](docs/MASTER_PLAN.md) | Existence — Master Plan |
| [STATE.md](STATE.md) | Existence — current state, 2026-09-11 |
