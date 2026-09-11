//! Inject fitted uncertainty-head weights into a net, then CHECK THE INJECTION DID NOT LIE.
//!
//! `fit_unc_head.py` fits 17 parameters offline and prints the AUC they achieve on a holdout. That
//! number is a claim about a Python dot product. The thing that will actually run inside search is
//! `Net::spread_from`, which differs from the Python in three ways that are easy to wave away and
//! each of which can destroy a ranking:
//!
//!   1. it multiplies by the net's `scale` and CLAMPS to [0, 30000];
//!   2. it CASTS to i32, so near-equal predictions collapse into ties;
//!   3. it floors at zero -- and the allocator consumes this signal NEGATED, so the positions it
//!      cares about most are the LOWEST predictions, which are exactly the ones a floor flattens
//!      into one tied value.
//!
//! So this recomputes the AUC through the REAL `spread_from`, on the SAME holdout rows, and prints
//! both numbers side by side. If they disagree, the offline result does not transfer and the reason
//! is in the clamp/cast, not in the fit. That is the whole point of the example existing: a head
//! whose measured behaviour is only ever the Python one is an unvalidated instrument.
//!
//! ROUND-TRIP IS CHECKED BY PARAMETER EQUALITY, not by sampling evals. Comparing `eval` on a handful
//! of positions could pass while a weight was corrupted in a region those positions do not reach;
//! comparing every element of w1/b1/w2/b2/scale cannot.
//!
//! WRITES ONLY WHERE IT IS TOLD. A net carrying a head serialises as schema v2, and the long-lived
//! snapshot binaries on this box (trainer, ruler, P2 arms) predate v2 and cannot load it. Point
//! `out` at a scratch path, never at a net any running job reads.
//!
//! USAGE: inject_unc_head <base.net> <weights.txt> <holdout.tsv> <out.net>

use nnue::Net;

const MATERIAL_CP: i64 = 10; // same "a real error" threshold the labeller and the probe use

fn auc(scored: &[(f64, bool)]) -> Option<f64> {
    let pos: Vec<f64> = scored.iter().filter(|&&(_, c)| c).map(|&(s, _)| s).collect();
    let neg: Vec<f64> = scored.iter().filter(|&&(_, c)| !c).map(|&(s, _)| s).collect();
    if pos.is_empty() || neg.is_empty() {
        return None;
    }
    let mut wins = 0.0f64;
    for &a in &pos {
        for &b in &neg {
            wins += if a > b {
                1.0
            } else if a == b {
                0.5
            } else {
                0.0
            };
        }
    }
    Some(wins / (pos.len() as f64 * neg.len() as f64))
}

fn main() {
    let a: Vec<String> = std::env::args().collect();
    if a.len() < 5 {
        eprintln!("usage: inject_unc_head <base.net> <weights.txt> <holdout.tsv> <out.net>");
        std::process::exit(2);
    }
    let (base_p, w_p, h_p, out_p) = (&a[1], &a[2], &a[3], &a[4]);

    let base = Net::load(base_p).unwrap_or_else(|e| panic!("load {base_p}: {e}"));
    println!("base net {base_p}: n_hidden={} scale={}", base.n_hidden, base.scale);

    // The base must have a ZERO head, or "before" is not a clean baseline.
    let zero_head = base.bu == 0.0 && base.wu.iter().all(|&x| x == 0.0);
    println!("base head is all-zero: {zero_head}");

    // ---- read the fitted weights (centipawn units) ------------------------------------------
    let txt = std::fs::read_to_string(w_p).unwrap_or_else(|e| panic!("read {w_p}: {e}"));
    let mut bu_cp = f32::NAN;
    let mut wu_cp = vec![f32::NAN; base.n_hidden];
    for ln in txt.lines() {
        if ln.starts_with('#') {
            continue;
        }
        let f: Vec<&str> = ln.split('\t').collect();
        match f.as_slice() {
            ["bu", v] => bu_cp = v.parse().expect("bu"),
            ["wu", i, v] => {
                let i: usize = i.parse().expect("wu index");
                assert!(i < base.n_hidden, "wu index {i} >= n_hidden");
                wu_cp[i] = v.parse().expect("wu value");
            }
            _ => {}
        }
    }
    assert!(bu_cp.is_finite(), "weights file has no bu");
    assert!(wu_cp.iter().all(|x| x.is_finite()), "weights file is missing a wu entry");

    // UNITS. spread_from computes (acc + bu) * scale, and the fit is in centipawns, so both
    // coefficients divide by the NET'S OWN scale. Reading it from the net rather than assuming
    // 600.0 means a net with a different scale cannot be silently mis-scaled.
    let mut net = base.clone();
    net.bu = bu_cp / base.scale;
    net.wu = wu_cp.iter().map(|&x| x / base.scale).collect();

    net.save(out_p).unwrap_or_else(|e| panic!("save {out_p}: {e}"));
    let back = Net::load(out_p).unwrap_or_else(|e| panic!("reload {out_p}: {e}"));

    // ---- round-trip: every eval parameter must be untouched ---------------------------------
    let mut drift = 0usize;
    if back.scale != base.scale || back.b2 != base.b2 || back.n_hidden != base.n_hidden {
        drift += 1;
    }
    for (x, y) in back.w1.iter().zip(base.w1.iter()) {
        if x != y {
            drift += 1;
        }
    }
    for (x, y) in back.b1.iter().zip(base.b1.iter()) {
        if x != y {
            drift += 1;
        }
    }
    for (x, y) in back.w2.iter().zip(base.w2.iter()) {
        if x != y {
            drift += 1;
        }
    }
    let head_ok = back.bu == net.bu && back.wu.iter().zip(net.wu.iter()).all(|(x, y)| x == y);
    println!("round-trip: eval params differing = {drift} (must be 0); head preserved = {head_ok}");
    assert_eq!(drift, 0, "the round-trip changed an EVAL parameter -- injection is not inert");
    assert!(head_ok, "the round-trip did not preserve the head");

    // ---- the real question: does spread_from reproduce the offline ranking? ------------------
    let hold = std::fs::read_to_string(h_p).unwrap_or_else(|e| panic!("read {h_p}: {e}"));
    let mut in_net: Vec<(f64, bool)> = Vec::new();
    let mut offline: Vec<(f64, bool)> = Vec::new();
    let mut zero_clamped = 0usize;
    let mut n = 0usize;
    for ln in hold.lines() {
        if ln.starts_with('#') {
            continue;
        }
        // cost, flip, resid, then n_hidden activations -- so 3 + n_hidden, NOT 4 + n_hidden. The
        // first version of this guard demanded one column too many, matched nothing, and reported
        // "n/a" instead of failing. An empty parse is a broken probe, so it now asserts below.
        let f: Vec<&str> = ln.split('\t').collect();
        if f.len() < 3 + base.n_hidden {
            continue;
        }
        let cost: i64 = f[0].parse().expect("cost");
        let flip: i64 = f[1].parse().expect("flip");
        let acts: Vec<f32> = f[3..3 + base.n_hidden]
            .iter()
            .map(|s| s.parse().expect("act"))
            .collect();
        let costly = flip == 1 && cost >= MATERIAL_CP;

        // Through the REAL head, exactly as search would call it.
        let s = back.spread_from(&acts) as f64;
        if s <= 0.0 {
            zero_clamped += 1;
        }
        in_net.push((s, costly));

        // The same dot product WITHOUT clamp or cast, to isolate what the clamp costs.
        let mut acc = 0.0f64;
        for h in 0..base.n_hidden {
            if acts[h] > 0.0 {
                acc += acts[h] as f64 * wu_cp[h] as f64;
            }
        }
        offline.push((acc + bu_cp as f64, costly));
        n += 1;
    }

    // An empty parse must be LOUD. The first run of this example silently reported "n/a" for every
    // AUC because a column guard was off by one, and an n/a reads like "no signal" when it actually
    // means "no data". A probe that returns nothing is broken until proven otherwise.
    assert!(
        n > 0,
        "parsed 0 holdout rows from {h_p} -- expected {} tab-separated columns per line. \
         This is a broken parse, not an absent signal.",
        3 + base.n_hidden
    );
    let npos = in_net.iter().filter(|&&(_, c)| c).count();
    assert!(npos > 0, "holdout has no costly flips -- AUC would be undefined");
    println!("\nholdout {n} rows, {npos} costly");
    println!(
        "  clamped to 0 by spread_from : {zero_clamped}/{n} ({:.0}%) -- all tied",
        100.0 * zero_clamped as f64 / n as f64
    );

    let f = |x: Option<f64>| x.map(|v| format!("{v:.3}")).unwrap_or("n/a".into());
    let neg = |v: &[(f64, bool)]| -> Vec<(f64, bool)> {
        v.iter().map(|&(s, c)| (-s, c)).collect()
    };
    println!("\n                       AUC      NEGATED");
    println!(
        "  offline (float)     {}      {}",
        f(auc(&offline)),
        f(auc(&neg(&offline)))
    );
    println!(
        "  in-net spread_from  {}      {}",
        f(auc(&in_net)),
        f(auc(&neg(&in_net)))
    );
    println!(
        "\n  A gap between these two rows is the CLAMP AND CAST, not the fit -- both rows are the\n  \
         same 17 numbers on the same positions. The negated column is the one an allocator would\n  \
         consume, and it is the column the floor-at-zero damages most."
    );
    println!("\nwrote {out_p} (schema v2 -- snapshot binaries CANNOT load this; scratch paths only)");
}
