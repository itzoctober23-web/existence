//! Evaluation network skeleton.
//!
//! The ARCHITECTURE IS DATA (CRATE.md 5): layer sizes and the input spec come from the net
//! file header, and this crate builds itself from them. Nothing here encodes a chess opinion —
//! there is no piece value, no mobility term, no king-safety term. At iteration zero the
//! weights are random, so the eval is noise, exactly as MASTER_PLAN "Iteration zero" states.
//!
//! Effective piece values are a THING TO READ OUT LATER by ablation, never a thing to write in.

use board::types::{Color, N_PIECE_KINDS, PieceKind};
use board::{Position, bitboard as bb};

/// Input layout, derived from the rules and nothing else:
///   12 x 64  one binary plane per (piece kind, side) per square
///        1   side to move
///        4   castling rights
///        8   en-passant file (one-hot, all zero when there is none)
pub const N_PIECE_PLANES: usize = 2 * N_PIECE_KINDS * 64;
pub const IDX_STM: usize = N_PIECE_PLANES;
pub const IDX_CASTLE: usize = IDX_STM + 1;
pub const IDX_EP: usize = IDX_CASTLE + 4;
pub const N_INPUTS: usize = IDX_EP + 8;
/// Most features that can be active at once: at most 32 men on the board, plus side-to-move,
/// four castling rights and one en-passant file -- 38. Sized to 64 so the hot-path stack buffer
/// has headroom, and so an illegal position with more men than chess allows hits a bounds panic
/// rather than silently evaluating a truncated feature set.
pub const MAX_ACTIVE: usize = 64;

/// A score in centipawn-like units. The UNIT is arbitrary and learned; nothing here fixes a
/// scale, and mapping a terminal Outcome to a score is `score_of`'s job, not this crate's.
pub type Score = i32;

#[derive(Clone)]
pub struct Net {
    pub n_hidden: usize,
    /// [N_INPUTS][n_hidden], INPUT-major. Stored this way on purpose: the feature vector is
    /// one-hot-ish (about 38 of 782 inputs are ever non-zero -- 32 pieces at most, plus side to
    /// move, castling and ep), so evaluation is a gather-add over the rows of the ACTIVE inputs
    /// rather than a dense 256x782 sweep. Same arithmetic, ~20x fewer operations, and each row
    /// is contiguous so it vectorises.
    pub w1: Vec<f32>,
    pub b1: Vec<f32>,
    pub w2: Vec<f32>,
    pub b2: f32,
    /// UNCERTAINTY HEAD — a SECOND linear output over the SAME hidden layer, predicting how far
    /// this net's static opinion sits from what its own deeper search returns.
    ///
    /// Self-referential by construction: the referee is the engine's own search, so no external
    /// judgment enters. It shares the trunk, so it costs one extra dot product over `n_hidden` and
    /// nothing in the eval path -- `eval()` does not touch it.
    ///
    /// ZERO-INITIALISED EVERYWHERE, and that is the parity guarantee. `spread()` returns exactly
    /// `bu` for a zero head, so a net that has never trained one is numerically identical to a net
    /// that has no head at all. Nothing in search reads it yet; adding it cannot move a single game.
    ///
    /// The TARGET this is trained against is NOT settled -- `uncertainty_target_PREREG.md` records
    /// that the obvious choice, |static - deep|, is measured ANTI-correlated with costly move flips
    /// (17.1% against a spec threshold of 80%). The head is the mechanism; the label is a separate
    /// question with a registered decision rule.
    pub wu: Vec<f32>,
    pub bu: f32,
    /// Multiplier from the network's natural units into `Score`. Declared, not learned-yet.
    pub scale: f32,
}

impl Net {
    /// FUNCTION-PRESERVING WIDENING (Net2WiderNet, Chen et al. 2015).
    ///
    /// WHY THIS EXISTS. The ARCH arm proposed width 16 -> 32 three times in one run and the
    /// surrogate filter killed it every time, before any game was played:
    ///     ARCH w 16 -> w 32: held-out 0.0744 vs champ 0.0687 -- surrogate filter, no gate
    /// The candidate was built by `train_fresh` -- RANDOM init at the new width, 30 epochs --
    /// and judged on held-out loss against a champion carrying 75 generations of training. A
    /// randomly-initialised wider net cannot win that comparison AT BIRTH, so capacity could
    /// never increase and the champion was pinned at width 16 permanently. The origin control
    /// sat flat across exactly that span: 0.831 -> 0.808 -> 0.825 at generations 25/50/75.
    ///
    /// This makes the wider net compute EXACTLY the same function as the narrow one, so it
    /// starts at the champion's loss instead of at random and is then judged on what training
    /// ADDS rather than on the handicap it was born with.
    ///
    /// Each new unit k >= h copies a source unit m[k] < h: same incoming weights, same bias.
    /// Copying alone would multiply that unit's contribution, so each source unit's OUTGOING
    /// weight is divided by its replica count. Then for any input:
    ///     sum_k relu(a[m[k]]) * w2[m[k]]/count[m[k]] == sum_j relu(a[j]) * w2[j]
    /// which is the original output, exactly.
    ///
    /// The copies get a tiny asymmetry in w1. Exact duplicates receive identical gradients
    /// forever and stay tied, giving the wider net more parameters and no more capacity -- the
    /// failure mode the paper explicitly warns about. The noise is 1e-4 of the init scale:
    /// far too small to move the eval, large enough to break the tie.
    pub fn widen(&self, new_hidden: usize, seed: u64) -> Self {
        assert!(new_hidden >= self.n_hidden, "widen() cannot narrow a net");
        let h = self.n_hidden;
        let mut r = Rng(seed | 1);
        let map: Vec<usize> = (0..new_hidden)
            .map(|k| if k < h { k } else { (r.next() % h as u64) as usize })
            .collect();
        let mut count = vec![0usize; h];
        for &j in &map { count[j] += 1; }

        let s1 = (2.0f32 / N_INPUTS as f32).sqrt() * 1e-4;
        let mut w1 = vec![0.0f32; N_INPUTS * new_hidden];
        for f in 0..N_INPUTS {
            for k in 0..new_hidden {
                let v = self.w1[f * h + map[k]];
                // Only the COPIES are perturbed. Units k < h stay bit-exact, so widening a net
                // to its own width reproduces it byte-for-byte.
                w1[f * new_hidden + k] = if k < h { v } else { v + r.normal() * s1 };
            }
        }
        Net {
            n_hidden: new_hidden,
            w1,
            b1: (0..new_hidden).map(|k| self.b1[map[k]]).collect(),
            w2: (0..new_hidden).map(|k| self.w2[map[k]] / count[map[k]] as f32).collect(),
            b2: self.b2,
            // The uncertainty head reads the SAME hidden layer, so it needs the SAME Net2WiderNet
            // correction as w2: a unit split into `count` copies must have its outgoing weight
            // divided by `count` or the output changes. Widening a net to its own width therefore
            // reproduces its SPREAD byte-for-byte, exactly as it already does its eval.
            wu: (0..new_hidden).map(|k| self.wu[map[k]] / count[map[k]] as f32).collect(),
            bu: self.bu,
            scale: self.scale,
        }
    }

    /// Deterministic pseudo-random init. Iteration zero has no knowledge in it; the seed only
    /// makes runs reproducible (MASTER_PLAN Safeguards: everything reproducible from a seed).
    pub fn random(n_hidden: usize, seed: u64) -> Self {
        let mut r = Rng(seed | 1);
        // He-style scaling keeps the pre-activations in a sane range at init; this is generic
        // initialisation practice, not a chess choice.
        let s1 = (2.0f32 / N_INPUTS as f32).sqrt();
        let s2 = (2.0f32 / n_hidden as f32).sqrt();
        Net {
            n_hidden,
            w1: (0..N_INPUTS * n_hidden).map(|_| r.normal() * s1).collect(),
            b1: vec![0.0; n_hidden],
            w2: (0..n_hidden).map(|_| r.normal() * s2).collect(),
            b2: 0.0,
            // ZERO, not He-scaled. A random net has no opinion about its own error, and a random
            // one would be a confident lie. It also keeps `Net::random` byte-identical in behaviour
            // to every measurement taken before this head existed.
            wu: vec![0.0; n_hidden],
            bu: 0.0,
            scale: 600.0,
        }
    }

    /// Indices of the ACTIVE inputs. Absolute frame: no flipping, no mirroring, no
    /// king-relativity (MASTER_PLAN Given). Whether a mover-relative view helps is something
    /// architecture search may DISCOVER; it is not given.
    pub fn active(pos: &Position, out: &mut Vec<u16>) {
        out.clear();
        Self::active_with(pos, |i| out.push(i));
    }

    /// Shared body for `active` and the hot-path stack variant below. It exists so the two
    /// cannot drift: a second hand-copied feature enumeration would be a silent eval divergence,
    /// and the dense reference in `features_dense` only checks the Vec path.
    #[inline]
    fn active_with(pos: &Position, mut push: impl FnMut(u16)) {
        for c in [Color::White, Color::Black] {
            for k in PieceKind::ALL {
                let base = ((c.idx() * N_PIECE_KINDS + k.idx()) * 64) as u16;
                for sq in bb::squares(pos.pieces[c.idx()][k.idx()]) {
                    push(base + sq as u16);
                }
            }
        }
        if pos.stm == Color::White {
            push(IDX_STM as u16);
        }
        for i in 0..4 {
            if pos.castling & (1 << i) != 0 {
                push((IDX_CASTLE + i) as u16);
            }
        }
        if let Some(ep) = pos.ep {
            push(IDX_EP as u16 + ep.file() as u16);
        }
    }

    /// Dense reference version, kept ONLY so tests can prove the sparse path agrees with it.
    pub fn features_dense(pos: &Position, out: &mut [f32]) {
        out.fill(0.0);
        let mut idx = Vec::new();
        Self::active(pos, &mut idx);
        for i in idx {
            out[i as usize] = 1.0;
        }
    }

    /// Evaluate from WHITE's point of view, then flip for the mover. The flip is a convention
    /// of the search interface (negamax wants mover-relative), not a claim about the position.
    pub fn eval(&self, pos: &Position, scratch: &mut Vec<f32>) -> Score {
        // STACK BUFFER, NOT A Vec. This function is the hottest in the program and the previous
        // `Vec::with_capacity(40)` was a malloc and a free on EVERY eval. That cost is fixed
        // while the arithmetic is not: at the shipped width of 16 an eval is only ~38x16 row-adds
        // plus a 16-wide head, so the allocation is a large fraction of it. At width 512 it would
        // be noise -- which is why this was easy to leave in and easy to miss.
        //
        // Sized to MAX_ACTIVE (64) against a true maximum of 38: at most 32 men, plus side to
        // move, four castling rights and one ep file. Writing through `buf[n]` means an
        // impossible position panics on the bounds check instead of silently truncating its
        // feature set, which would be a wrong eval rather than a crash.
        let mut buf = [0u16; MAX_ACTIVE];
        let mut n = 0usize;
        Self::active_with(pos, |i| { buf[n] = i; n += 1; });
        scratch.clear();
        scratch.extend_from_slice(&self.b1);
        for &i in &buf[..n] {
            let row = &self.w1[i as usize * self.n_hidden..(i as usize + 1) * self.n_hidden];
            for (a, w) in scratch.iter_mut().zip(row.iter()) {
                *a += *w;
            }
        }
        let mut acc = 0.0f32;
        for h in 0..self.n_hidden {
            let s = scratch[h];
            if s > 0.0 {
                acc += s * self.w2[h]; // ReLU
            }
        }
        let white_pov = (acc + self.b2) * self.scale;
        let v = if pos.stm == Color::White { white_pov } else { -white_pov };
        v.clamp(-30_000.0, 30_000.0) as Score
    }

    /// The uncertainty head: how far this net expects its own static opinion to sit from what its
    /// own deeper search would return, in `Score` units.
    ///
    /// THE TRUNK IS DUPLICATED FROM `eval` ON PURPOSE. Factoring the shared hidden-layer loop out
    /// would be tidier and would touch the hottest function in the program -- whose own comments
    /// record a malloc in this exact loop having been a large fraction of an eval at the shipped
    /// width. The requirement on this change is that it cannot move a single game, and the way to
    /// guarantee that is not to edit the path that plays them.
    ///
    /// NOT NEGATED BY SIDE TO MOVE, unlike `eval`. A spread is a MAGNITUDE -- "how wrong might I be
    /// here" is the same quantity whoever is to move -- so applying eval's stm flip would make it
    /// negative for one side and meaningless.
    ///
    /// Floored at zero because it predicts an ABSOLUTE residual. A zero head therefore returns
    /// exactly 0, which is what every net written before this head existed reports.
    pub fn spread(&self, pos: &Position, scratch: &mut Vec<f32>) -> Score {
        let mut buf = [0u16; MAX_ACTIVE];
        let mut n = 0usize;
        Self::active_with(pos, |i| { buf[n] = i; n += 1; });
        scratch.clear();
        scratch.extend_from_slice(&self.b1);
        for &i in &buf[..n] {
            let row = &self.w1[i as usize * self.n_hidden..(i as usize + 1) * self.n_hidden];
            for (a, w) in scratch.iter_mut().zip(row.iter()) {
                *a += *w;
            }
        }
        self.spread_from(scratch)
    }

    /// The head applied to an ALREADY-COMPUTED hidden layer, for callers that have just run the
    /// trunk and should not pay for it twice.
    pub fn spread_from(&self, scratch: &[f32]) -> Score {
        let mut acc = 0.0f32;
        for h in 0..self.n_hidden {
            let s = scratch[h];
            if s > 0.0 {
                acc += s * self.wu[h]; // ReLU, same activation as the value head
            }
        }
        let v = (acc + self.bu) * self.scale;
        v.clamp(0.0, 30_000.0) as Score
    }
}

/// Incremental accumulator. `eval` rebuilds the hidden sums from ~38 active feature rows at
/// every node; a move changes at most 4 features (from, to, a capture, a castled rook), so the
/// same value is reachable with 2-4 row updates instead of ~38. That is the difference between
/// search depth and datagen volume being a trade-off and not being one.
///
/// Correctness is not assumed: `tests/incremental.rs` asserts refresh() == update() over every
/// legal move of thousands of positions from random games.
#[derive(Clone)]
pub struct Acc {
    pub vals: Vec<f32>,
}

impl Acc {
    pub fn new(net: &Net) -> Self {
        Acc { vals: net.b1.clone() }
    }
    /// Full rebuild from a position.
    pub fn refresh(&mut self, net: &Net, pos: &Position) {
        self.vals.clear();
        self.vals.extend_from_slice(&net.b1);
        // Same per-call allocation the eval path had; same stack fix. refresh() runs at every
        // root and on every accumulator reset, so it is hot too.
        let mut buf = [0u16; MAX_ACTIVE];
        let mut n = 0usize;
        Net::active_with(pos, |i| { buf[n] = i; n += 1; });
        for &i in &buf[..n] {
            self.add(net, i, 1.0);
        }
    }
    #[inline]
    fn add(&mut self, net: &Net, feature: u16, sign: f32) {
        let h = net.n_hidden;
        let row = &net.w1[feature as usize * h..(feature as usize + 1) * h];
        if sign > 0.0 {
            for (a, w) in self.vals.iter_mut().zip(row) { *a += *w; }
        } else {
            for (a, w) in self.vals.iter_mut().zip(row) { *a -= *w; }
        }
    }
    /// Apply a feature-set delta: features that turned on and off.
    pub fn update(&mut self, net: &Net, on: &[u16], off: &[u16]) {
        for &f in off { self.add(net, f, -1.0); }
        for &f in on { self.add(net, f, 1.0); }
    }
    /// Output head, given the accumulated hidden sums. White-POV, like Net::eval's internals.
    pub fn output(&self, net: &Net) -> f32 {
        let mut acc = 0.0f32;
        for h in 0..net.n_hidden {
            let s = self.vals[h];
            if s > 0.0 { acc += s * net.w2[h]; }
        }
        acc + net.b2
    }
    /// Mover-relative Score, matching Net::eval exactly.
    pub fn score(&self, net: &Net, pos: &Position) -> Score {
        let white = self.output(net) * net.scale;
        let v = if pos.stm == Color::White { white } else { -white };
        v.clamp(-30_000.0, 30_000.0) as Score
    }
}

/// A snapshot of EXACTLY the position fields `Net::active_with` reads, and nothing else.
///
/// WHY THIS EXISTS. `Acc::update` needs the on/off feature delta of a move, and there was no way to
/// produce one cheaply. `tests/incremental.rs` derives it by enumerating BOTH positions' active sets
/// and diffing them with `contains()` -- correct, and O(38^2) on top of two full enumerations, i.e.
/// strictly slower than the `refresh()` it was meant to replace. That test proves the accumulator is
/// right; it is not a usable delta, which is why nothing in `engine` ever called it.
///
/// WHY IT IS A SNAPSHOT AND NOT A `Move`. Deriving the delta from a move means re-deriving, by hand,
/// every rule that moves a piece: promotions change the KIND, en passant removes a pawn that is not
/// on the destination square, castling moves a rook too, and both castling rights and the ep square
/// change independently of any of that. Each is a separate chance to disagree with `active_with` in a
/// way that only shows up as a wrong eval. Diffing the board state instead is mechanically derived
/// from the same fields the feature encoder reads, so the two cannot drift apart.
///
/// It is 12 bitboards plus three scalars -- ~100 bytes, no allocation.
#[derive(Clone)]
pub struct FeatSnap {
    pieces: [[bb::Bb; N_PIECE_KINDS]; 2],
    stm: Color,
    castling: u8,
    ep: Option<board::types::Square>,
}

impl FeatSnap {
    #[inline]
    pub fn of(pos: &Position) -> Self {
        FeatSnap { pieces: pos.pieces, stm: pos.stm, castling: pos.castling, ep: pos.ep }
    }

    /// Features that turned ON and OFF going from `self` to `after`.
    ///
    /// The piece half is a per-plane XOR: a move touches at most a couple of planes, and an
    /// unchanged plane costs one compare and is skipped. Only the bits that actually differ are
    /// walked, so this is O(changed features), not O(active features).
    pub fn delta(&self, after: &FeatSnap, on: &mut Vec<u16>, off: &mut Vec<u16>) {
        on.clear();
        off.clear();
        for c in 0..2 {
            for k in 0..N_PIECE_KINDS {
                let b = self.pieces[c][k];
                let a = after.pieces[c][k];
                let diff = b ^ a;
                if diff == 0 {
                    continue;
                }
                let base = ((c * N_PIECE_KINDS + k) * 64) as u16;
                for sq in bb::squares(diff) {
                    if a & (1u64 << sq) != 0 {
                        on.push(base + sq as u16);
                    } else {
                        off.push(base + sq as u16);
                    }
                }
            }
        }
        // `active_with` pushes IDX_STM only for White, so the feature toggles on every move.
        if self.stm != after.stm {
            if after.stm == Color::White {
                on.push(IDX_STM as u16);
            } else {
                off.push(IDX_STM as u16);
            }
        }
        for i in 0..4 {
            let b = self.castling & (1 << i) != 0;
            let a = after.castling & (1 << i) != 0;
            if a != b {
                if a {
                    on.push((IDX_CASTLE + i) as u16);
                } else {
                    off.push((IDX_CASTLE + i) as u16);
                }
            }
        }
        if self.ep != after.ep {
            if let Some(e) = self.ep {
                off.push(IDX_EP as u16 + e.file() as u16);
            }
            if let Some(e) = after.ep {
                on.push(IDX_EP as u16 + e.file() as u16);
            }
        }
    }
}

impl Net {
    /// Serialise. SCHEMAS.md 4: the HEADER IS THE ARCHITECTURE -- n_hidden and the input count
    /// come from the file, and `nnue` builds itself from them, so an architecture change is a
    /// data change and not a code change.
    ///
    /// Without this the learning loop was a no-op in practice: it trained a champion, printed a
    /// score, and exited, discarding the weights. The engine called Net::random() on every
    /// start, so the shipped binary stayed at iteration zero no matter how much it learned.
    pub fn save(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let mut f = std::io::BufWriter::new(std::fs::File::create(path)?);
        // VERSION 2 IS WRITTEN ONLY WHEN THERE IS A HEAD TO WRITE, and that is deliberate rather
        // than tidy. A v2 file is unreadable to any binary built before this change, and several
        // long-lived processes on this box run from SNAPSHOT binaries (the trainer, the P2 arms,
        // the ruler) that will not be rebuilt for hours. Emitting v2 unconditionally would make
        // every net this build writes unloadable by all of them at once.
        //
        // So a net whose uncertainty head is all zeros -- which is every net until one is trained
        // -- still serialises as v1, byte-for-byte what it always did. The head's bytes are
        // APPENDED, so a v2 file is a strict prefix-extension of the v1 layout and the reader below
        // needs one extra branch rather than a second format.
        let has_unc = self.bu != 0.0 || self.wu.iter().any(|&x| x != 0.0);
        f.write_all(b"EXNT")?;                                  // magic
        f.write_all(&(if has_unc { 2u16 } else { 1u16 }).to_le_bytes())?;   // schema version
        f.write_all(&(self.n_hidden as u32).to_le_bytes())?;
        f.write_all(&(N_INPUTS as u32).to_le_bytes())?;
        f.write_all(&self.scale.to_le_bytes())?;
        f.write_all(&self.b2.to_le_bytes())?;
        for v in self.b1.iter().chain(self.w2.iter()).chain(self.w1.iter()) {
            f.write_all(&v.to_le_bytes())?;
        }
        if has_unc {
            f.write_all(&self.bu.to_le_bytes())?;
            for v in self.wu.iter() { f.write_all(&v.to_le_bytes())?; }
        }
        f.flush()
    }

    pub fn load(path: &str) -> std::io::Result<Self> {
        use std::io::Read;
        let mut f = std::io::BufReader::new(std::fs::File::open(path)?);
        let mut m = [0u8; 4];
        f.read_exact(&mut m)?;
        if &m != b"EXNT" {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad magic"));
        }
        let mut u16b = [0u8; 2];
        f.read_exact(&mut u16b)?;
        let ver = u16::from_le_bytes(u16b);
        // 1 = no uncertainty head (every net written before 2026-09-11, including the live
        // champion and the whole banked set). 2 = the head's bytes are appended after w1.
        // Reading v1 is not a compatibility shim to be removed later -- it is how the champion
        // loads, and it must keep working.
        if ver != 1 && ver != 2 {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("unknown schema version {ver}")));
        }
        let mut u32b = [0u8; 4];
        f.read_exact(&mut u32b)?;
        let n_hidden = u32::from_le_bytes(u32b) as usize;
        f.read_exact(&mut u32b)?;
        let n_in = u32::from_le_bytes(u32b) as usize;
        if n_in != N_INPUTS {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("net has {n_in} inputs, this build has {N_INPUTS}")));
        }
        let rf = |f: &mut std::io::BufReader<std::fs::File>| -> std::io::Result<f32> {
            let mut b = [0u8; 4];
            f.read_exact(&mut b)?;
            Ok(f32::from_le_bytes(b))
        };
        let scale = rf(&mut f)?;
        let b2 = rf(&mut f)?;
        let mut b1 = Vec::with_capacity(n_hidden);
        for _ in 0..n_hidden { b1.push(rf(&mut f)?); }
        let mut w2 = Vec::with_capacity(n_hidden);
        for _ in 0..n_hidden { w2.push(rf(&mut f)?); }
        let mut w1 = Vec::with_capacity(N_INPUTS * n_hidden);
        for _ in 0..N_INPUTS * n_hidden { w1.push(rf(&mut f)?); }
            // A v1 net gets a ZERO head, which `spread()` reports as exactly 0. That is the honest
            // value: a net trained before the head existed has no opinion about its own error.
            let (bu, wu) = if ver >= 2 {
                let bu = rf(&mut f)?;
                let mut wu = Vec::with_capacity(n_hidden);
                for _ in 0..n_hidden { wu.push(rf(&mut f)?); }
                (bu, wu)
            } else {
                (0.0, vec![0.0; n_hidden])
            };
            Ok(Net { n_hidden, w1, b1, w2, b2, wu, bu, scale })
    }
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    /// Irwin–Hall approximation of a standard normal; adequate for initialisation.
    fn normal(&mut self) -> f32 {
        let mut s = 0.0f32;
        for _ in 0..6 {
            s += (self.next() >> 11) as f32 / (1u64 << 53) as f32;
        }
        (s - 3.0) * 0.7071
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use board::Position;

    /// Positions from short random walks, not just startpos: the accumulator's behaviour
    /// depends on WHICH features are active, and startpos exercises one arrangement.
    fn walk(n: usize) -> Vec<Position> {
        let mut rng: u64 = 0xC0FFEE;
        let mut rnd = || { rng ^= rng << 13; rng ^= rng >> 7; rng ^= rng << 17; rng };
        let mut out = Vec::new();
        while out.len() < n {
            let mut p = Position::startpos();
            for _ in 0..(4 + rnd() % 20) {
                let l = p.legal_moves();
                if l.is_empty() { break; }
                p.make_move(l.as_slice()[(rnd() % l.len() as u64) as usize]);
            }
            out.push(p);
        }
        out
    }

    /// Widening to the SAME width must reproduce the net exactly. Units k < h are copied
    /// bit-for-bit and every replica count is 1, so this is the degenerate case that catches a
    /// wrong index or a stray division without any tolerance to hide behind.
    #[test]
    fn widen_to_same_width_is_identical() {
        let net = Net::random(32, 7);
        let same = net.widen(32, 12345);
        assert_eq!(same.n_hidden, net.n_hidden);
        assert_eq!(same.w1, net.w1, "w1 changed when widening to the same width");
        assert_eq!(same.b1, net.b1);
        assert_eq!(same.w2, net.w2);
        assert_eq!(same.b2, net.b2);
    }

    /// THE POINT OF THE WHOLE CONSTRUCTION: a widened net computes the SAME function.
    ///
    /// This is what makes the ARCH arm fair. The candidate used to be random-initialised at the
    /// new width, so its held-out loss at birth was far worse than a champion with dozens of
    /// generations behind it, and the surrogate filter rejected every widening before a single
    /// game was played -- capacity could never increase. Starting from an identical function
    /// means the wider net is judged on what training ADDS, not on a handicap it was born with.
    ///
    /// Tolerance exists only because the copies carry a deliberate 1e-4 asymmetry to break the
    /// gradient tie; without it this would be exact. 2cp on a scale where evals run to hundreds.
    #[test]
    fn widen_preserves_the_function() {
        let net = Net::random(16, 7);
        let wide = net.widen(64, 999);
        assert_eq!(wide.n_hidden, 64);
        let (mut sa, mut sb) = (Vec::new(), Vec::new());
        let mut worst = 0i32;
        for p in walk(40) {
            let a = net.eval(&p, &mut sa);
            let b = wide.eval(&p, &mut sb);
            worst = worst.max((a - b).abs());
        }
        assert!(worst <= 2, "widening changed the eval by {worst}cp; it must preserve the function");
    }

    /// The copies must NOT be exact duplicates. Identical units receive identical gradients
    /// forever and stay tied, which is the failure mode Net2Net warns about: more parameters,
    /// no more capacity. So the wider net would be judged fairly and then still learn nothing.
    #[test]
    fn widen_breaks_the_symmetry() {
        let net = Net::random(8, 7);
        let wide = net.widen(16, 4242);
        let h = net.n_hidden;
        let mut identical_rows = 0;
        for k in h..wide.n_hidden {
            let src: Vec<f32> = (0..N_INPUTS).map(|f| wide.w1[f * wide.n_hidden + k]).collect();
            for j in 0..h {
                let orig: Vec<f32> = (0..N_INPUTS).map(|f| wide.w1[f * wide.n_hidden + j]).collect();
                if src == orig { identical_rows += 1; }
            }
        }
        assert_eq!(identical_rows, 0, "copied units are bit-identical to their source: they will never diverge");
    }
}
