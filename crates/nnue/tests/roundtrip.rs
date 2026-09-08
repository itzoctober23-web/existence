//! save() -> load() must reproduce the net EXACTLY, or a trained champion silently degrades
//! the moment it is written to disk and read back.
use board::Position;
use nnue::Net;

#[test]
fn save_load_roundtrip_is_exact() {
    let net = Net::random(48, 12345);
    let path = std::env::temp_dir().join("existence_roundtrip.net");
    let p = path.to_str().unwrap();
    net.save(p).expect("save");
    let back = Net::load(p).expect("load");

    assert_eq!(net.n_hidden, back.n_hidden);
    assert_eq!(net.scale, back.scale);
    assert_eq!(net.b2, back.b2);
    assert_eq!(net.b1, back.b1);
    assert_eq!(net.w2, back.w2);
    assert_eq!(net.w1, back.w1);

    // and it must EVALUATE identically on real positions
    let mut s1 = Vec::new();
    let mut s2 = Vec::new();
    let mut pos = Position::startpos();
    let mut rng: u64 = 99;
    for _ in 0..200 {
        assert_eq!(net.eval(&pos, &mut s1), back.eval(&pos, &mut s2), "eval differs after roundtrip");
        let l = pos.legal_moves();
        if l.is_empty() { break; }
        rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17;
        pos.make_move(l.as_slice()[(rng % l.len() as u64) as usize]);
    }
    let _ = std::fs::remove_file(p);
}
