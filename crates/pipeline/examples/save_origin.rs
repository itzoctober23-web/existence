//! Persist the FROZEN ORIGIN as a net file.
//!
//! WHY THIS DID NOT EXIST AND SHOULD HAVE. The origin is the reference for every "progress from
//! nothing" number this project quotes, and it is built inline in two places --
//! `main.rs` (`Net::random(WIDTH_MENU[rung], seed)`) and `control.rs:77` -- but never written to
//! disk. So it can only be measured by the ONE instrument that constructs it, the in-loop control,
//! and can never be played head-to-head by `netmatch` or entered into a `pool_rating` field.
//!
//! That gap is exactly why a contradiction went unresolvable on 2026-09-10: the control said the
//! champion was RESOLVED WORSE against the origin over gens 100-400 (0.873 -> 0.819, -0.054 +/-
//! 0.033) while `netmatch` said gen400 BEATS gen200 head-to-head (0.539 +/- 0.027). Two resolved
//! results pointing opposite ways, and no way to cross-check the origin leg with a second
//! instrument because the opponent existed only inside the loop that measured it.
//!
//! Same construction as control.rs:77, so the file IS the origin rather than a lookalike.
//!
//! usage: save_origin <width> <seed> <out.net>
use nnue::Net;

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let width: usize = a.first().and_then(|s| s.parse().ok()).unwrap_or(16);
    let seed: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(20260907);
    let out = a.get(2).cloned().unwrap_or_else(|| "origin.net".to_string());
    let net = Net::random(width, seed);
    match net.save(&out) {
        Ok(()) => println!("wrote {out}: Net::random({width}, {seed}), n_hidden {}", net.n_hidden),
        Err(e) => { eprintln!("FAILED to write {out}: {e}"); std::process::exit(1); }
    }
}
