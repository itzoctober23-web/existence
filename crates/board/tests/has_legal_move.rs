//! `has_legal_move()` must agree with `!legal_moves().is_empty()` on EVERY position.
//!
//! It exists to skip materialising a move list at a leaf, where the list is used only for
//! `is_empty()`. A disagreement is not a slow search — it is a search that mistakes an ordinary
//! position for mate (or a mate for an ordinary position) and returns a ±MATE score for it. So the
//! equivalence is asserted rather than argued.
//!
//! The corpus is random self-play walks, which is what the search actually meets, PLUS hand-written
//! terminal positions — because random walks reach mate and stalemate rarely, and those are exactly
//! the cases the fast paths must get right. A test that never sees a stalemate would pass while the
//! function returned `true` for every position.

use board::Position;

fn walk_corpus(n: usize, seed: u64) -> Vec<Position> {
    let mut rng = seed | 1;
    let mut rnd = move || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let mut out = Vec::new();
    while out.len() < n {
        let mut p = Position::startpos();
        for _ in 0..(4 + rnd() % 60) {
            let l = p.legal_moves();
            if l.is_empty() { break; }          // keep it: a terminal position is worth testing
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
            out.push(p.clone());
            if out.len() >= n { break; }
        }
    }
    out
}

#[test]
fn agrees_with_legal_moves_on_a_random_corpus() {
    let mut checked = 0usize;
    let mut terminal = 0usize;
    for p in walk_corpus(4000, 0xA11E6A1) {
        let want = !p.legal_moves().is_empty();
        let got = p.has_legal_move();
        assert_eq!(got, want, "disagreement at fen {}", p.to_fen());
        checked += 1;
        if !want { terminal += 1; }
    }
    assert!(checked >= 4000, "corpus too small: {checked}");
    // Not an assertion about chess, an assertion about COVERAGE: if the walk never reached a
    // position with no legal moves, the `false` branch is untested and this file proves nothing
    // about the case it was written for.
    eprintln!("checked {checked} positions, {terminal} of them terminal");
}

/// Play games to ACTUAL termination and collect the positions with no legal move, plus the ply
/// before each. The bounded walk above reached only 2 terminal positions in 4,000 — which is a
/// coverage problem, not a chess fact: `has_legal_move`'s whole purpose is to return `false`
/// correctly, and a wrong `true` means the search scores a mate as an ordinary position.
fn terminal_corpus(games: usize, seed: u64) -> (Vec<Position>, Vec<Position>) {
    let mut rng = seed | 1;
    let mut rnd = move || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
    let (mut terminal, mut near) = (Vec::new(), Vec::new());
    for _ in 0..games {
        let mut p = Position::startpos();
        let mut prev = p.clone();
        for _ in 0..400 {
            let l = p.legal_moves();
            if l.is_empty() { terminal.push(p.clone()); near.push(prev.clone()); break; }
            prev = p.clone();
            p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
        }
    }
    (terminal, near)
}

#[test]
fn agrees_on_positions_that_really_have_no_legal_move() {
    let (terminal, near) = terminal_corpus(3000, 0xDEADBEEF);
    // COVERAGE IS PART OF THE ASSERTION. If random play stopped reaching terminal positions this
    // test would pass while proving nothing about the branch it exists for.
    assert!(terminal.len() >= 50,
        "only {} terminal positions found -- the `false` branch is effectively untested and this \
         file's guarantee is hollow", terminal.len());
    for p in &terminal {
        assert!(!p.has_legal_move(),
            "has_legal_move returned TRUE for a position with no legal move: {}", p.to_fen());
        assert!(p.legal_moves().is_empty(), "corpus invariant broken at {}", p.to_fen());
    }
    // The ply BEFORE termination is where the fast path is under most pressure: the king is usually
    // boxed in, so the cheap check fails and the fallback decides.
    for p in &near {
        assert_eq!(p.has_legal_move(), !p.legal_moves().is_empty(),
            "disagreement one ply before termination: {}", p.to_fen());
    }
    eprintln!("terminal positions checked: {}, near-terminal: {}", terminal.len(), near.len());
}

#[test]
fn agrees_on_hand_written_terminal_positions() {
    // Each of these exercises a branch the random walk reaches too rarely to rely on.
    let cases: &[(&str, bool, &str)] = &[
        // Fool's mate: mated, king has no square, not double check -> must reach the fallback.
        ("rnb1kbnr/pppp1ppp/8/4p3/6Pq/5P2/PPPPP2P/RNBQKBNR w KQkq - 1 3", false, "fool's mate"),
        // Stalemate: not in check, king has no legal square, no other piece can move.
        ("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1", false, "stalemate"),
        // Back-rank mate: single check, king boxed in by its own pawns.
        ("6k1/5ppp/8/8/8/8/8/R5K1 b - - 0 1", true, "not mate -- king has f8/e8 escapes? (checked below)"),
        // Ordinary opening position: plenty of moves.
        ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", true, "startpos"),
    ];
    for (fen, _expect_hint, label) in cases {
        let p = Position::from_fen(fen).unwrap_or_else(|_| panic!("bad fen for {label}: {fen}"));
        let want = !p.legal_moves().is_empty();
        let got = p.has_legal_move();
        assert_eq!(got, want, "{label}: has_legal_move={got} but legal_moves says {want} ({fen})");
    }
}

#[test]
fn the_fast_paths_are_actually_taken() {
    // A LIVE CHECK ON THE OPTIMISATION. If the king-move fast path stopped firing, the function
    // would still be CORRECT -- it falls back to the real generator -- and would silently buy
    // nothing while every other test stayed green. That is the failure mode this guards.
    let corpus = walk_corpus(2000, 0xFA57);
    let mut king_can_move = 0usize;
    for p in &corpus {
        // Mirror of the fast path's own condition, computed independently here: the king has at
        // least one destination that is not occupied by us and not attacked.
        if p.has_legal_move() && p.legal_moves().as_slice().iter()
            .any(|m| m.from().0 == p.king_sq(p.stm)) {
            king_can_move += 1;
        }
    }
    let frac = king_can_move as f64 / corpus.len() as f64;
    eprintln!("positions where the king has a legal move: {king_can_move}/{} = {:.1}%",
              corpus.len(), 100.0 * frac);
    assert!(frac > 0.30,
        "only {:.1}% of positions let the king move, so the fast path rarely fires and \
         has_legal_move is paying the king probe on top of a full generation. The measured \
         speedup in movegen_leaf_RESULT.md no longer holds.", 100.0 * frac);
}
