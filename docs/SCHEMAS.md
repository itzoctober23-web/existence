# SCHEMAS.md — Data Formats

Everything the engine learns is data; everything it does is recorded. These formats are
fixed before champion 1 exists so that the ledger is a single, comparable series from the
first entry. Every record is append-only and content-hashed (BLAKE3, 32 hex chars).

## 0. Conventions
- Encoding: binary records use a fixed little-endian layout with a 4-byte magic and a
  u16 schema version; text records are canonical JSON (sorted keys, no whitespace) so that
  hashes are stable.
- Every record carries `schema_version`. Readers reject unknown versions loudly.
- Content addressing: `data/objects/<hash>` holds nets, tables, programs, books, position
  sets, config snapshots. The ledger references objects by hash only.
- Nothing in `data/` is edited in place. Ever.
- Candidate identifiers: per-class sequential, assigned at proposal time by `ledger`:
  `P####` program, `T####` table set, `N####` net, `A####` architecture step, `F####`
  feature, `S####` statistic, `H####` hyperparameter, `Q####` purity-lineage program.
  Identifiers are for humans; hashes are the truth. Both appear on every record.

## 1. Position record (training data) — binary, fixed width
Written by `datagen`; read by `trainer`, `oracle`, `datagen` (set mining).
| Field | Type | Notes |
|---|---|---|
| planes | packed bits | `board.planes()`: one bit per (piece type, side, square) + rules-carried state (castling bits, ep square index, in-hand counts where the rules have them); width is a property of the board type, recorded in the file header |
| side_to_move | u8 | |
| outcome | i8 | +1 / 0 / -1 from the mover's perspective (rules-derived) |
| search_score | i16 | root score from the champion's search at the datagen budget |
| eval_static | i16 | the champion's static eval (for residual weighting, FITNESS 4) |
| ply | u16 | |
| cost_units | u32 | cost consumed by the search that produced search_score |
| flags | u8 | bit0 in_check, bit1 capture_pending, bit2 opening_random_ply, bit3 book_start, bit4 c960_start, bit5 held_out |
| game_id | u64 | for retrograde mining and dedup |
File header: magic, schema_version, board_id (chess8 / teams14), plane_width, champion_hash,
datagen_config_hash, count. Files are immutable once closed.

## 2. Program (search program) — canonical JSON
```
{ "schema_version": 1, "board_id": "chess8",
  "functions": [ { "name": "choose", "params": [["p","Pos"],["B","Int"]], "ret": "Move",
                   "body": <typed AST node> }, ... ],
  "tables": [ {"name": "D", "shape": [], "hash": "<hash>"}, ... ],
  "size_nodes": 29, "lineage": "MAIN" | "PURITY", "parent": "<program hash>",
  "mutations": [ {"op": "wrap-if", "site": [1,3,0]} ] }
```
AST node: `{"prim": "max", "type": "Int", "args": [...]}`; leaves carry `"const": n`,
`"var": "name"`, `"pred": "is_capture"`, `"op": "sub"`, `"rel": ">="`, `"field": "count"`.
`size_nodes` is computed by `grammar`, never written by hand (it is what retires the
hand estimates in GRAMMAR 6). Exactness taint is NOT stored — it is recomputed from the
AST by `grammar` on load so it cannot drift from the code that defines it.

## 3. Table — binary
Header: magic, schema_version, name, dtype (i16/i32), rank, dims, index_features
(list of feature ids, e.g. `["depth","move_index","improving"]`), producer (spsa /
bootstrap / score_of). Body: row-major values. `score_of` is a table with dims
[outcome=4, depth] and is stored like any other.

## 4. Network — binary
Header: magic, schema_version, board_id, input_feature_spec_hash (points to a Feature
Set object), layer sizes, activation ids, output buckets, quantization scales, trainer
config hash, training data hashes (list), parent net hash. Body: weights. The header IS
the architecture; `nnue` builds itself from it.

## 5. Feature set — canonical JSON
List of features, each a conjunction of `PredId`s over (piece, square, relation, other
piece), with `feature_id` and the champion at which it was accepted. `board.planes()` is
feature set 0 and is implicit. Incremental-update eligibility is a computed flag, not
stored.

## 6. Statistic (online counter) definition — canonical JSON
`{ "stat_id", "keys": ["from_sq","to_sq"], "update_on": ["cutoff"], "bonus_form": ..., 
"decay": ..., "read_weight_table": "<hash>" }`. Definitions only; runtime memory is not
persisted.

## 7. Champion bundle — canonical JSON, one per accepted champion
```
{ "schema_version": 1, "champion_index": 412, "ledger_hash": "<hash of all entries <= this>",
  "name": "kidop-sinub", "board_id": "chess8",
  "program": "<hash>", "tables": {"D": "<hash>", "reduce": "<hash>", ...},
  "net": "<hash>", "feature_set": "<hash>", "stats": ["<hash>", ...],
  "score_of": "<hash>", "hyper": "<hash>", "configs": {"gate": "<hash>", "cost": "<hash>",
  "hardware": "<hash>", "phases": "<hash>"},
  "identity_string": "Existence 412 kidop-sinub — program P0031 (cleared e1=2.0 at LTC)" }
```
`name` = proquint of the first 32 bits of `ledger_hash` (MASTER_PLAN "Versioning").
`identity_string` is rendered by `ledger`, never typed. The engine binary loads exactly
this and nothing else.
PURITY lineage bundles use the same format with `"lineage": "PURITY"`, their own
`champion_index` series, and the MAIN champion's data hashes for everything except
`program` (CRATE.md 7, lineage semantics). They are never shipped; they are the record.

## 8. Ledger entry — canonical JSON, append-only
One per gate decision (accept OR reject) and per system event. Every field below is one
the specification has already justified; nothing is stored that could not be defended.
```
{ "schema_version": 1, "entry_index": n, "prev_hash": "<hash>", "timestamp_utc": ...,
  "kind": "accept" | "reject" | "anchor" | "release" | "config_change" | "preflight_fail"
          | "allocation" | "phase_boundary",
  "arm": "PROGRAM"|"TABLE"|"NET"|"ARCH"|"FEATURE"|"STAT"|"HYPER"|"PURITY",
  "candidate": { "class": ..., "object_hash": "<hash>", "parent_object_hash": "<hash>",
                 "diff": <class-specific: program mutations list / table delta summary /
                         net header diff / feature or stat definition / hyper delta> },
  "lineage": "MAIN" | "PURITY",
  "champion_before": 411, "champion_after": 412 | null,   // null on reject
  "stages": {
    "typecheck_cost": {"pass": true},
    "oracle": {"ran": true, "exact_sites": 3, "exact_pass": true, "inexact_pass": true,
               "determinism_pass": true, "set_hash": "<hash>"},
    "surrogate": {"mates_per_cost": {"N1":..,"N2":..,"N3":..,"N4":..},
                  "agreement": .., "held_out_loss": ..},
    "fixed_cost": {"ran": true, "bounds": [e0,e1], "llr": .., "pairs": .., "pass": true},
    "stc": {"tc": "5+0.05", "bounds": [e0,e1], "llr": .., "pairs": .., "pentanomial": [..],
            "pass": true},
    "ltc": {"tc": "30+0.3", "bounds": [e0,e1], "llr": .., "pairs": .., "pentanomial": [..],
            "pass": true},
    "honesty": {"ran": false, "reason": "pre-P3"} | {"ran": true, "set_size": 300,
                "low_conf_when_wrong": 0.86, "pass": true} },
  "earned": { "e1_at_acceptance": 2.0, "note": "bar provably cleared; same quantity the
              bandit uses" },
  "where_it_mattered": { "position_set_hash": "<hash>", "top_disagreements": [
      {"fen_or_board_hash": "<hash>", "old_move": "..", "new_move": "..",
       "old_score": .., "new_score": .., "old_pv": [..], "new_pv": [..]} ] },
  "meaning": { <class-specific: predicates of a feature + exemplar hashes / mates now found /
               positions where move changed / cost and eval counts per move at LTC> },
  "confidence": { "pairs_total": .., "pv_stability_on_disagreements": .., 
                  "residual_class_on_disagreements": .. },
  "resources": { "gate_compute_hours": .., "cost_units_per_move_ltc": .., 
                 "evals_per_move_ltc": .., "wall_ms_per_move_ltc": .., 
                 "hardware_hash": "<hash>" },
  "configs": { "gate": "<hash>", "cost": "<hash>", "phases": "<hash>" },
  "hash": "<hash of this entry excluding this field>" }
```
Anchor entries: `{ "kind":"anchor", "champion_a": 392, "champion_b": 412, "games": 2000,
"delta_elo": 41.2, "ci95": [26.3, 56.1], "e1_derived": 2.06, "e1_applied": 2.06 (after
floor) }  // CI: 2,000 games at ~+/-15 Elo per 2,000 -> SE(total) ~8.0 -> +/-15.7 at 95%;
           // e1 SE ~0.4 (FITNESS 7.2)`. Release entries carry the 4,000-game match. Allocation entries carry the
bandit's per-arm reward inputs (acceptances, e1, compute-hours) and resulting shares.

## 9. Explanation record (P3) — canonical JSON, per move on request
`{ "champion": 412, "position": "<hash>", "move": "..", "plan": [trajectories],
"contrast": [ {"alt": "..", "refutation_pv": [..], "drop": ..} ],
"attribution": { "ablation": [ {"square": .., "delta": ..} ], "features": [ {"id": ..,
"contribution": ..} ] }, "mode": "depth" | "eval" | "mixed", "confidence": {"pv_stability":
.., "residual_class": ..}, "rendered": "<template text>" }`. Every rendered sentence must
map to a field in the same record (`explain` enforces this by construction: the template
has no free text).

## 10. Activation dump (concept layer) — binary
Per position: position hash + hidden-layer activations (i16, per accumulator half) +
output. Written on request by `engine` in a debug mode; read only by `analysis/`. Never on
the training or gating path.

## 11. Self-audit
- Is anything stored that a later document could not defend? Every ledger field maps to
  FITNESS.md stages or MASTER_PLAN Self-documentation items. `earned` is e1 only, and so
  is the number in `identity_string` (an earlier draft published a per-acceptance Elo
  there — the quantity FITNESS proves unmeasurable — in the most public field in the
  system; corrected). PASS.
- Can a stored field drift from the code that defines it? Exactness taint and
  `size_nodes` are recomputed on load, not trusted from disk. Identity strings are
  rendered, not stored by hand. PASS.
- Can two champions have incomparable numbers? `stages.*.tc`, `configs.*`, and
  `resources.hardware_hash` are on every entry; a reader can group by them. PASS.
- Is the 4PC board a retrofit here? `board_id` and header-carried plane width on every
  format; nothing assumes 64 squares. PASS.
- Known gaps: (a) the `diff` and `meaning` fields are class-specific and only loosely
  specified — tighten per class as each proposer is built; (b) no schema for the 4PC
  protocol messages yet (P6). (Pentanomial is unconditional per FITNESS 7.3, so the
  arrays are always meaningful.)
