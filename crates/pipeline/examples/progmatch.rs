//! Match two SAVED programs head to head. The test STATE.md calls decisive, made runnable.
//!
//! WHY THIS EXISTS. STATE.md:2655 names the decisive test for the acceptance-rule question:
//! *"whether the veto arm's final champion beats the control's, head to head, after both complete 25
//! generations -- and that is one match, on nets that will exist for free."* It was not runnable. The
//! nets persist, but the thing that DIFFERS between the two arms is the evolved program, and
//! `evolve.rs:927` records that saved `.prog` files are `{:#?}` Debug dumps, "readable but not
//! parseable back". `sexp.rs` fixed the persistence; this is the consumer that turns a pair of saved
//! champions into an answer.
//!
//! DEPTH 4 IS THE DEFAULT ON PURPOSE. `netmatch.rs:23-29` sets depth 4 as this project's strength
//! standard, and STATE.md:277 records the reason in the strongest terms available: blend 1.00 read
//! +0.050 at depth 2 and did NOT hold at depth 4, which is called "a real depth-dependent difference
//! in the nets, not an instrument failing". Depth 2 is the DATAGEN depth. A champion comparison taken
//! at the datagen depth would repeat exactly the mistake that cost this project a shipped default.
//!
//! ON PAIR COUNT. The default here is 96, not the gate's 6. `gate_power_RESULT.md` establishes that
//! the 6-pair gate has never once accepted a candidate because its acceptance bar has never been
//! inside the achievable range, and the A/A curve measures the near-parity half-width at 6 pairs as
//! +/-0.250 -- a bar of 0.750. Two champions from sibling arms are BY CONSTRUCTION near parity, which
//! is the regime where a small sample is least able to say anything.
//!
//! WHAT IT DOES NOT DO. It does not decide anything. It prints a rate and an interval; whether that
//! clears a bar is a separate, pre-registered judgement. A harness that also renders a verdict is how
//! an interval quietly becomes a claim.
use grammar::sexp;
use nnue::Net;
use pipeline::gate;

/// `ref:<substring>` loads a reference program by name; anything else is a path to a `.sexp`.
///
/// The `ref:` form exists because the most useful champion comparison is usually against the SEED it
/// descended from, and requiring that to be dumped to a file first is friction that invites skipping
/// the control.
fn load(spec: &str) -> grammar::Program {
    if let Some(want) = spec.strip_prefix("ref:") {
        let all = grammar::reference::all();
        let hit: Vec<_> = all.iter().filter(|(n, _)| n.contains(want)).collect();
        match hit.len() {
            1 => return hit[0].1.clone(),
            0 => panic!("no reference program matches {want:?}; have: {:?}",
                        all.iter().map(|(n, _)| *n).collect::<Vec<_>>()),
            _ => panic!("{want:?} is ambiguous: {:?}",
                        hit.iter().map(|(n, _)| *n).collect::<Vec<_>>()),
        }
    }
    let text = std::fs::read_to_string(spec)
        .unwrap_or_else(|e| panic!("cannot read {spec}: {e}"));
    sexp::from_str(&text).unwrap_or_else(|e| panic!("cannot parse {spec}: {e}"))
}

fn main() {
    let mut a = std::env::args().skip(1);
    let (pa, pb) = match (a.next(), a.next()) {
        (Some(x), Some(y)) => (x, y),
        _ => {
            eprintln!("usage: progmatch <a.sexp> <b.sexp> [pairs=96] [depth=4] [seed=777] [budget=16]");
            eprintln!("  prints a's score; below 0.5 means b is stronger");
            std::process::exit(2);
        }
    };
    let pairs: usize = a.next().and_then(|s| s.parse().ok()).unwrap_or(96);
    let depth: i64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    let seed: u64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(777);
    let budget: i64 = a.next().and_then(|s| s.parse().ok()).unwrap_or(16);

    let (pa_prog, pb_prog) = (load(&pa), load(&pb));
    let net = Net::random(32, 20260907);

    // Node counts are printed because two champions can differ in COST as well as in strength, and a
    // rate alone cannot tell those apart. GRAMMAR 6 counts nodes; this is that same measure.
    let sa: usize = pa_prog.funcs.iter().map(|f| f.body.size()).sum();
    let sb: usize = pb_prog.funcs.iter().map(|f| f.body.size()).sum();
    println!("progmatch — {pairs} pairs, depth {depth}, budget {budget}, seed {seed}");
    println!("  a: {pa}  ({sa} nodes, lineage {:?})", pa_prog.lineage);
    println!("  b: {pb}  ({sb} nodes, lineage {:?})", pb_prog.lineage);
    if pa_prog == pb_prog {
        println!("  NOTE: the two programs are IDENTICAL — this is an A/A test and must read ~0.500.");
    }

    let sc = gate::match_progs(&pa_prog, &pb_prog, &net,
                               vec![depth, 32_000, interp::uct_exploration()],
                               budget, pairs, seed, 4, u64::MAX);
    let (r, c) = (sc.pent_rate(), sc.ci95());
    println!("\n  a's score {r:.3} +/- {c:.3}   95% CI [{:.3}, {:.3}]   ({} games)",
             r - c, r + c, sc.games());
    println!("  {}", if r - c > 0.5 { "a is resolved STRONGER" }
                     else if r + c < 0.5 { "b is resolved STRONGER" }
                     else { "NOT RESOLVED at this pair count — the interval spans 0.500" });
    println!("\n  Below 0.5 means b is stronger. An interval spanning 0.500 is a non-answer, not a tie:");
    println!("  it means this many pairs cannot separate them, which is the expected outcome for two");
    println!("  champions from sibling arms unless one genuinely diverged.");
}
