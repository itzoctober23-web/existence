//! UCI front end. Iteration zero: random-init net, bare alpha-beta.
//!
//! Plays to a NODE BUDGET when `go` carries one (`movetime`, `wtime`/`btime`, or `nodes`), and to
//! the fixed `Depth` option otherwise -- so every existing fixed-depth measurement is reproduced
//! byte-for-byte by a `go` with no parameters.
//!
//! It plays legal chess and finds mates inside its horizon because terminal conditions are
//! rules; everything else it does is noise, and it is supposed to be. MASTER_PLAN says so
//! plainly, and the explanation layer will say so too when it exists.

mod search;

use board::types::MOVE_NONE;
use board::Position;
use nnue::Net;
use search::Search;
use std::io::{BufRead, Write};

const NAME: &str = "Existence";
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Nodes per millisecond, for converting a clock into a node budget.
///
/// MEASURED, not guessed: `search_bench` reports ~1.14-1.25M nps at width 16 across depths 3-6 on
/// this machine (1,143,167 nps at depth 6). 1000 nodes/ms is that figure rounded DOWN, so a
/// movetime is never overspent.
///
/// It is a calibration constant and it is machine-specific. It affects only how much searching a
/// given millisecond buys; it does not affect determinism, because the search stops on NODES.
/// **RE-MEASURED 2026-09-10: 1000 -> 2000.** The figure above was right when written and the engine
/// has since got faster (the incremental accumulator, the shuffle-division fix), so the constant
/// was silently costing time. Measured on the engine's OWN `go` at fixed depth 6 -- node counts are
/// identical run to run, so only the time varies -- over 5 positions x 5 repeats, with two trainers,
/// a 240-game 4PC anchor and a 224-pair netmatch all live on the box:
///
/// ```text
///   min nps        3.34M - 4.38M
///   median         2.37M - 4.07M
///   WORST observed 2,239,873      <- under full contention
/// ```
///
/// 2000 is below the WORST contended observation, i.e. rounded DOWN in the same spirit as the
/// original. It is not set to 3000: that exceeds the worst case, and overspending is unsafe here
/// because without iterative deepening an aborted search has no completed root move and returns
/// `score cp -32000` with a random move.
///
/// Measured consequence of the stale value: `go movetime 8000` used **1.2% of its clock** (depth 5,
/// 93 ms of 8 s). The remaining waste is the d5->d6 threshold being a 10x step, which only
/// ITERATIVE DEEPENING would fix -- and MASTER_PLAN line 38 forbids seeding that, so it must be
/// discovered by the search track rather than written in here.
const NPS_PER_MS: u64 = 2000;

/// How many nodes a full-width search costs at each depth, MEASURED BY THIS ENGINE on a single
/// position, which is what a `go` actually has to pay for.
///
/// The first version of this table used `search_bench` figures (d4 121,562) and was ~10x too large,
/// because that harness sums over a POSITION SET while a `go` searches ONE position (d4 = 12,469
/// from startpos). A table that overstates cost makes the engine pick a shallower depth than the
/// clock affords, i.e. silently throw time away -- the opposite of the defect this whole change
/// exists to fix.
///
/// Calibrated on the MIDGAME position, not the start position, because they differ by ~31x at
/// depth 6 (12,696,968 against 410,798) and a table calibrated on the cheap case would overspend
/// on every real position:
///
/// ```text
///   depth        2      3       4         5          6
///   startpos    176   2,352   12,469     81,421     410,798
///   midgame     352  10,309   72,977  1,234,802  12,696,968   <- used
/// ```
///
/// Depths 7 and 8 extrapolate at the midgame geometric branching factor of 13.8.
///
/// This is a CALIBRATION, and it is what converts speed into depth: a faster engine buys more nodes
/// per millisecond, crosses these thresholds sooner, and therefore searches deeper for the same
/// clock. That conversion is the entire point -- before it existed, a 2x faster engine searched the
/// identical tree and scored identically.
///
/// It is a start-position estimate and a midgame node is not a startpos node, so it is deliberately
/// conservative: `depth_for_budget` takes the largest depth whose estimate FITS, and `SAFETY` gives
/// the real search room to exceed the estimate before the cap bites.
const DEPTH_NODES: [(u32, u64); 6] = [
    (3, 10_309), (4, 72_977), (5, 1_234_802), (6, 12_696_968),
    (7, 174_964_226), (8, 2_410_907_034),
];
/// The node cap is set this many times the budget, so it catches a pathological position rather
/// than firing on an ordinary one. A cap that fires routinely reintroduces the -INF bug above.
const SAFETY: u64 = 4;

fn depth_for_budget(budget: u64) -> u32 {
    let mut best = 2;
    for (d, n) in DEPTH_NODES {
        if n <= budget { best = d; }
    }
    best
}

fn main() {
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    let mut pos = Position::startpos();
    // Iteration zero: the net has no knowledge in it. The seed only makes runs reproducible.
    // Load a learned net if one is present, else fall back to iteration zero. The engine used
    // to ALWAYS call Net::random(), so every training run was discarded and the shipped binary
    // stayed at iteration-zero strength no matter what the loop had learned.
    let net_path = std::env::var("EXISTENCE_NET").unwrap_or_else(|_| "champion.net".to_string());
    let (net, loaded) = match Net::load(&net_path) {
        Ok(n) => (n, true),
        Err(_) => (Net::random(256, 0xE1_57_E0_1C), false),
    };
    if loaded {
        eprintln!("info string loaded net {net_path} ({} hidden)", net.n_hidden);
    } else {
        eprintln!("info string no net at {net_path}; using random init (iteration zero)");
    }
    let mut search = Search::new(net, 0xC0FFEE);
    let mut depth: u32 = 4;

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        let mut it = line.split_whitespace();
        match it.next() {
            Some("uci") => {
                println!("id name {NAME} {VERSION}");
                println!("id author ExistenceIsPain");
                println!("option name Depth type spin default 4 min 1 max 8");
                println!("option name Nodes type spin default 0 min 0 max 1000000000");
                println!("uciok");
            }
            Some("isready") => println!("readyok"),
            Some("ucinewgame") => pos = Position::startpos(),
            Some("setoption") => {
                let toks: Vec<&str> = line.split_whitespace().collect();
                if let Some(i) = toks.iter().position(|t| *t == "value") {
                    if toks.get(i.wrapping_sub(1)) == Some(&"Depth")
                        || toks.contains(&"Depth")
                    {
                        if let Some(v) = toks.get(i + 1).and_then(|s| s.parse().ok()) {
                            depth = v;
                        }
                    }
                }
            }
            Some("position") => {
                let rest: Vec<&str> = it.collect();
                pos = parse_position(&rest).unwrap_or_else(Position::startpos);
            }
            Some("go") => {
                // SPEND THE CLOCK, not a fixed number of plies.
                //
                // Until 2026-09-10 this line read `search.best_move(&mut pos, depth)` and the `go`
                // parameters were parsed nowhere: the engine played `depth` plies whatever the
                // clock said. That made every speed measurement in this project worth zero Elo by
                // construction -- a 2x faster engine searched THE IDENTICAL TREE in half the time
                // and scored identically. `simd_refuted_RESULT.md` (0.3%), `width_clock_RESULT.md`
                // (11 attempts, sign never turned) and the +6.3% shuffle-division fix were all
                // measured against an engine that could not spend the speed.
                //
                // MASTER_PLAN line 43 specifies the gate at fixed TIME. This is what makes that
                // possible, and it is the Given column's `iterate to budget`, not a new technique:
                // no ordering, no hash reuse, no deepening, no pruning. Only when to stop.
                //
                // TIME IS CONVERTED TO A NODE BUDGET rather than checked against a clock inside the
                // search, so a game stays DETERMINISTIC and reproducible from its seed (FITNESS 10).
                // A millisecond check would make the same seed play differently under load, which on
                // this box -- eight datagen lanes and three training arms -- is not hypothetical.
                let toks: Vec<&str> = line.split_whitespace().collect();
                let num = |key: &str| -> Option<u64> {
                    toks.iter().position(|t| *t == key)
                        .and_then(|i| toks.get(i + 1))
                        .and_then(|v| v.parse::<u64>().ok())
                };
                let explicit_nodes = num("nodes");
                let ms: Option<u64> = num("movetime").or_else(|| {
                    // Classical clock: spend a modest constant fraction. No time-management
                    // cleverness -- that is a strength technique and belongs on the discovery list.
                    let (t, inc) = if pos.stm == board::Color::White {
                        (num("wtime"), num("winc").unwrap_or(0))
                    } else {
                        (num("btime"), num("binc").unwrap_or(0))
                    };
                    t.map(|t| t / 30 + inc / 2)
                });
                let cap = explicit_nodes.or_else(|| ms.map(|ms| ms.saturating_mul(NPS_PER_MS).max(1)));

                let (m, score) = match cap {
                    Some(c) => {
                        // PICK THE DEPTH THE BUDGET AFFORDS, then let the cap act only as a safety
                        // net. Simply raising depth and relying on the cap DOES NOT WORK and the
                        // first version of this did exactly that: without iterative deepening, an
                        // abort inside the FIRST root child means no root move ever completed, so
                        // `best_score` stays -INF and the move returned is whatever the shuffle put
                        // first. Measured: `go movetime 1000` reported `score cp -32000` and a
                        // random move. Depth must be chosen so the search FINISHES.
                        let d = depth_for_budget(c);
                        search.node_cap = c.saturating_mul(SAFETY);
                        let mut r = search.best_move(&mut pos, d);
                        // Belt and braces: if even that did not complete a single root move, fall
                        // back to a depth that always does. This is a robustness floor, not a
                        // deepening schedule -- it runs only when the budget was too small for the
                        // cheapest estimate, and it never runs DEEPER than the first attempt.
                        if r.1 <= -search::INF {
                            search.node_cap = u64::MAX;
                            r = search.best_move(&mut pos, 1);
                        }
                        search.node_cap = u64::MAX;
                        r
                    }
                    None => search.best_move(&mut pos, depth),
                };
                // Report the depth ACTUALLY searched. This printed the `Depth` OPTION even when a
                // budget had chosen a different depth, which would have put a wrong number in front
                // of every future time-control measurement.
                let reported = match cap { Some(c) => depth_for_budget(c), None => depth };
                println!(
                    "info depth {reported} score cp {score} nodes {} pv {}",
                    search.nodes,
                    if m == MOVE_NONE { "0000".into() } else { m.to_string() }
                );
                println!(
                    "bestmove {}",
                    if m == MOVE_NONE { "0000".into() } else { m.to_string() }
                );
            }
            Some("d") => println!("{}", pos.to_fen()),
            Some("quit") => break,
            _ => {}
        }
        let _ = out.flush();
    }
}

fn parse_position(toks: &[&str]) -> Option<Position> {
    let (mut pos, i) = match toks.first()? {
        &"startpos" => (Position::startpos(), 1usize),
        &"fen" => {
            let end = toks.iter().position(|t| *t == "moves").unwrap_or(toks.len());
            (Position::from_fen(&toks[1..end].join(" ")).ok()?, end)
        }
        _ => return None,
    };
    if toks.get(i) == Some(&"moves") {
        for ms in &toks[i + 1..] {
            let legal = pos.legal_moves();
            let found = legal.as_slice().iter().find(|m| m.to_string() == *ms).copied();
            match found {
                Some(m) => {
                    pos.make_move(m);
                }
                None => break,
            }
        }
    }
    Some(pos)
}
