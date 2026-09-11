//! The uncertainty head must not move a single game.
//!
//! The directive that asked for it is explicit: "No Elo expected from this alone; it must not lose
//! any." A second output over the shared trunk can violate that in three ways that a compile check
//! will not catch, and each has a test here:
//!
//!   1. it changes what `eval` returns -- then every game changes;
//!   2. it breaks the on-disk format -- then the CHAMPION stops loading, which is worse than a
//!      regression because nothing downstream can run at all;
//!   3. it survives a widen incorrectly -- the ARCH arm widens nets, and `w2` needs a Net2WiderNet
//!      correction that the new head needs too, or the spread changes for free.
//!
//! The parity claim is not "we were careful". It is these assertions.

use board::chess::Position;
use nnue::Net;

fn champion_path() -> String {
    format!("{}/../../p1_champion.net", env!("CARGO_MANIFEST_DIR"))
}

fn positions() -> Vec<Position> {
    vec![
        Position::startpos(),
        Position::from_fen("rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1").unwrap(),
        Position::from_fen("r1bqkbnr/pppp1ppp/2n5/4p3/2B1P3/5N2/PPPP1PPP/RNBQK2R b KQkq - 3 3").unwrap(),
        Position::from_fen("8/8/8/4k3/8/4K3/4P3/8 w - - 0 1").unwrap(),
    ]
}

#[test]
fn the_champion_still_loads_and_reports_zero_spread() {
    // THE CHECK THAT MATTERS. Every net on this box is schema v1 -- the champion, the live
    // production net, and the whole banked set. If v1 stopped loading, nothing would run.
    let p = champion_path();
    let net = Net::load(&p).unwrap_or_else(|e| panic!(
        "the champion at {p} no longer loads after adding the uncertainty head: {e}. \
         Every banked net is v1; reading v1 is not a shim, it is the format they are in."));

    assert_eq!(net.bu, 0.0, "a v1 net must load with a ZERO uncertainty head");
    assert!(net.wu.iter().all(|&x| x == 0.0), "a v1 net must load with a ZERO uncertainty head");
    assert_eq!(net.wu.len(), net.n_hidden, "the head must be sized to the trunk even when unused");

    let mut scratch = Vec::new();
    for (i, pos) in positions().iter().enumerate() {
        assert_eq!(net.spread(pos, &mut scratch), 0,
                   "position {i}: a net with no trained head must report spread 0, not a guess");
    }
}

#[test]
fn the_head_cannot_change_what_eval_returns() {
    // The head shares the trunk, so the only thing standing between it and the eval path is that
    // `eval` never reads wu/bu. Asserted rather than reviewed: give the head large weights and
    // check every eval is bit-identical.
    let base = Net::random(16, 20260911);
    let mut with_head = base.clone();
    with_head.bu = 37.5;
    for (i, w) in with_head.wu.iter_mut().enumerate() {
        *w = 0.25 * (i as f32 + 1.0);       // deliberately large and non-uniform
    }

    let (mut s1, mut s2) = (Vec::new(), Vec::new());
    for (i, pos) in positions().iter().enumerate() {
        assert_eq!(base.eval(pos, &mut s1), with_head.eval(pos, &mut s2),
                   "position {i}: eval changed when the uncertainty head was populated");
    }
    // ... and the head must actually be doing something, or the test above is vacuous.
    let mut sc = Vec::new();
    let any_nonzero = positions().iter().any(|p| with_head.spread(p, &mut sc) != 0);
    assert!(any_nonzero, "positive control: a populated head must produce a non-zero spread somewhere");
}

#[test]
fn a_zero_head_still_writes_schema_v1_byte_for_byte() {
    // A v2 file is unreadable to every binary built before this change, and several long-lived
    // processes here run from snapshot binaries. So a net with nothing to say must still write v1.
    let net = Net::load(&champion_path()).expect("champion loads");
    let tmp = std::env::temp_dir().join("unc_parity_v1.net");
    net.save(tmp.to_str().unwrap()).expect("save");

    let orig = std::fs::read(champion_path()).expect("read champion");
    let back = std::fs::read(&tmp).expect("read round-trip");
    assert_eq!(u16::from_le_bytes([back[4], back[5]]), 1,
               "a zero-head net must serialise as schema v1");
    assert_eq!(orig.len(), back.len(), "v1 round-trip changed the file length");
    assert_eq!(orig, back, "v1 round-trip is not byte-identical");
    let _ = std::fs::remove_file(&tmp);
}

#[test]
fn a_trained_head_round_trips_through_schema_v2() {
    let mut net = Net::random(16, 7);
    net.bu = -1.25;
    for (i, w) in net.wu.iter_mut().enumerate() { *w = (i as f32) * 0.5 - 2.0; }

    let tmp = std::env::temp_dir().join("unc_parity_v2.net");
    net.save(tmp.to_str().unwrap()).expect("save");
    let bytes = std::fs::read(&tmp).expect("read");
    assert_eq!(u16::from_le_bytes([bytes[4], bytes[5]]), 2,
               "a populated head must serialise as schema v2");

    let back = Net::load(tmp.to_str().unwrap()).expect("v2 must load");
    assert_eq!(back.bu, net.bu);
    assert_eq!(back.wu, net.wu);
    assert_eq!(back.w2, net.w2, "the value head must survive a v2 round-trip unchanged");
    assert_eq!(back.b2, net.b2);

    // A v2 file must be a strict PREFIX-EXTENSION of the v1 layout: same header and body, head
    // appended. That is what lets the reader need one branch instead of a second format.
    let mut zeroed = net.clone();
    zeroed.bu = 0.0;
    zeroed.wu = vec![0.0; zeroed.n_hidden];
    let tmp1 = std::env::temp_dir().join("unc_parity_v2_prefix.net");
    zeroed.save(tmp1.to_str().unwrap()).expect("save v1");
    let v1 = std::fs::read(&tmp1).expect("read v1");
    assert_eq!(bytes[6..v1.len()], v1[6..],
               "v2 must repeat the v1 body verbatim after the version field");
    assert_eq!(bytes.len(), v1.len() + 4 + 4 * net.n_hidden,
               "v2 must add exactly bu plus n_hidden weights");
    let _ = std::fs::remove_file(&tmp);
    let _ = std::fs::remove_file(&tmp1);
}

#[test]
fn widening_to_the_same_width_preserves_the_spread() {
    // The ARCH arm widens nets. `w2` carries a Net2WiderNet correction (divide by the copy count)
    // and the head reads the SAME hidden layer, so it needs the same one. Without it the spread
    // would change for free every time a net was widened -- silently, since nothing reads it yet.
    let mut net = Net::random(16, 99);
    net.bu = 3.0;
    for (i, w) in net.wu.iter_mut().enumerate() { *w = 0.1 * (i as f32) - 0.7; }

    let same = net.widen(net.n_hidden, 12345);
    let (mut a, mut b) = (Vec::new(), Vec::new());
    for (i, pos) in positions().iter().enumerate() {
        assert_eq!(net.spread(pos, &mut a), same.spread(pos, &mut b),
                   "position {i}: widening to its own width changed the spread");
        assert_eq!(net.eval(pos, &mut a), same.eval(pos, &mut b),
                   "position {i}: widening to its own width changed the eval");
    }
}
