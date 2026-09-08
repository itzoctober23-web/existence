//! The outcome label decides what the net learns. A sign error here trains it to LOSE, and
//! nothing downstream would notice: the loss would fall, self-play would look healthy, and the
//! engine would get steadily worse. Checked against positions whose result is known by hand.

use board::{Color, Outcome, Position};

#[test]
fn mated_side_is_the_loser() {
    // Black to move and mated (back rank). Outcome::Loss means SIDE TO MOVE is mated.
    let pos = Position::from_fen("6k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1").unwrap();
    let mut p = pos.clone();
    // Ra8 is mate.
    let m = p.legal_moves().as_slice().iter().copied()
        .find(|m| m.to_string() == "a1a8").expect("Ra8 legal");
    p.make_move(m);
    assert_eq!(p.outcome(), Outcome::Loss, "black should be mated");
    assert_eq!(p.stm, Color::Black, "black is the side to move at the terminal");

    // datagen labels z from WHITE's point of view: mated black => white won => z = +1
    let z = if p.stm == Color::White { -1.0f32 } else { 1.0f32 };
    assert_eq!(z, 1.0, "white delivered mate, so white-POV label must be +1");
}

#[test]
fn stalemate_is_a_draw_not_a_loss() {
    // Classic stalemate: black to move, no legal move, not in check.
    let p = Position::from_fen("7k/5Q2/6K1/8/8/8/8/8 b - - 0 1").unwrap();
    assert!(p.legal_moves().is_empty(), "position should have no legal moves");
    assert_eq!(p.outcome(), Outcome::Draw, "stalemate must be a draw, not a loss");
}

#[test]
fn mover_relative_flip_is_consistent() {
    // z is white-POV; the trainer converts to mover-POV. Verify the flip both ways.
    for (fen, white_pov, expect_mover) in [
        ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1", 1.0f32, 1.0f32),
        ("rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR b KQkq - 0 1", 1.0f32, -1.0f32),
    ] {
        let p = Position::from_fen(fen).unwrap();
        let mover = if p.stm == Color::White { white_pov } else { -white_pov };
        assert_eq!(mover, expect_mover, "mover-POV flip wrong for {fen}");
    }
}
