Scope rule. No experiment, script, or tool may start unless its pre-registration file states, in one sentence, either (a) how it could move this project's primary metric named below, or (b) which named ladder step it unblocks. If that sentence cannot be written, the idea goes in `PARKED.md` as one line and nothing runs. "Interesting", "follow-up", "instrument check", "calibration", and "quick" are not sentences of type (a) or (b). This rule binds every session that opens this repo.

# Existence — CLAUDE.md (2026-09-12)

**Primary metric:** pooled Elo on the SF ruler (`sf_ruler.py`, SF-1320 @10k nodes, depth 4, 120 games/rung, pooled ± CI; readings go to `live_ruler.out`, pooled by `ruler_pool.py`).

**Status: PARKED.** `WEEK1_RETRO.md` is final: ruler ~1550 and flat; all gains were configuration changes (datagen depth 1→3 +128; lr 0.01→0.0002; blend 0.75→0.85); running longer bought nothing; node-budget datagen was −43 because the search lacks iterative deepening — a search finding, not a datagen one.

**Exactly one run is permitted:** P1 production at fixed-depth-4 datagen from the current champion (`p1_champion.net`), one pinned core, 300 generations, ruler rungs every 50 (prereg `p1_prod_d4_PREREG.md`, launcher `p1_prod_d4.sh`, result at planned N in `p1_prod_d4_RESULT.md`). No P2, no arms, no tooling, no dashboards, no cost-model work. The auto-promote / keepalive / live-ruler services and every experiment timer are disabled; nothing in this repo restarts except through `~/ops/manifest.txt`.

## Rules

* Parity before tuning, every step. One change per gate. Fixed time for anything involving speed. Pentanomial, both colours.
* Every `_RESULT.md` names both engines by hash and includes a control. Results only at planned N; running work writes to `STATE.md` only.
* One `STATE.md` line per project per day: ladder step, engine hash, gate LLR/pairs or ruler Elo ± CI, pin map. Nothing else in STATE.
* No tooling, dashboards, loaders, arenas. The Existence watch page exists; do not extend it.
* Kill criteria: maswabe2 gate 3 not passed in 2 days → report what differs. 3.1 flat in both engines → direction dead; write it and stop.

## Machine (recorded 2026-09-12; see `~/ops/manifest.txt`, `~/ops/watchdog.sh`)

Pin map, disjoint: 4PC cores 0-5 · maswabe2 gate 6-9 · maswabe2 builds/parity/ruler core 10 · Existence core 11 · cores 12-15 are his desktop and are never taken. The Aporia960 Lichess bot (`aporia-bot.service`, rules in `~/aporia/STATE.md`) runs on core 11 now that the Existence run has reached its planned N; it is not an experiment, not in the manifest, and ignored by the watchdog. Runs are launched only through the manifest command (`systemd-run --user`, persistent binaries on btrfs, never tmpfs). The cron watchdog restarts a dead manifest run at most 3×/day; it never starts anything else. Week stop: day 7 = 2026-09-19 — if 4PC or maswabe2 has not passed 3.1 and Existence's pooled ruler is not above 1600 with a significant trend, stop every run and write `WEEK2_RETRO.md` before anything new starts.

**Human-gated steps.** When a step needs the human — credentials, tokens, account creation or changes, anything irreversible — stop, state exactly what is needed and where it should go, and END THE TURN. Do not poll, sleep-loop, or re-check on a timer; the human will say when it is done. Never work around a human gate (no asking for secrets in chat, no guessing, no touching the account).

## Version control (binding for every session)
**Branches.** `main` = the frozen champion; nothing lands on `main` except a gate-accepted promotion (merge
commit whose message is the `_RESULT.md` summary line). `step/<n>-<name>` per ladder step or experiment —
all tuning, preregs, and results for that step live there. `infra/<name>` for tooling/gates/docs — never
changes engine behaviour; may merge any time with the bench fingerprint proving node counts unchanged.
No force-push, no history rewriting on anything ever pushed.

**Tags.** `baseline-<eval>-<date>` on every frozen baseline; `champion-<n>` on every promotion; `result/<step>`
on the commit a `_RESULT.md` refers to. Annotated; the message carries binary md5, bench fingerprint,
rustc version, build flags, and the artifact hashes (net/tables/book) the engine used.

**Engine identity = git.** UCI `id name` is built at compile time from `git describe --tags --dirty`
(e.g. `Aporia champion-3-2-gab12cd`). A `-dirty` suffix means an unmeasured build; gate scripts REFUSE to
run a binary whose id ends in `-dirty`. No hand-typed version strings.

**Learned artifacts are content-addressed, not in git.** Nets, tables, books, position sets live in
`data/objects/<blake3>`; the repo tracks `artifacts.toml` (name -> hash). `.gitignore` must exclude
`*.nnue *.qnn *.bin data/`. A champion tag pins its artifact hashes.

**Commits.** One logical change; `<area>: <what>` on line one; body says why and, for engine changes,
bench fingerprint before/after. Result files, preregs, and STATE lines commit WITH the code they describe.
Every `_RESULT.md` names both engines by commit hash + tag, never by path.

**Remotes.** Private GitHub remote on every repo; `git push --tags` is part of writing a result or a
promotion. `data/objects/` is rsynced nightly to a second disk or cloud bucket (it is not in git and the
machine is a single point of failure).

**CI.** On every push: build, `cargo test`, perft fixtures, bench fingerprint check against `PARITY.md`
for `main`, `cargo fmt --check`, `cargo clippy -D warnings`. Red CI blocks merge to `main`. The gate
uses the CI-built artifact for `main` where possible, so the measured binary is reproducible.

**Gate provenance.** Every gate records: both engine tags + md5, options snapshot (hash size, threads,
book, adjudication rules), hardware pin, TC, seed. Games are archived as PGN under `data/games/<gate>/`
(content-addressed) for later analysis and adversary mining.
