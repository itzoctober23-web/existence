//! Train the net on self-play outcomes. Hand-written backprop for the 782 -> H -> 1 net.
//!
//! Target = the game outcome from the MOVER's point of view, blended with the engine's own
//! root search score. Both are self-referential (MASTER_PLAN Given: "game outcome; agreement
//! with own deeper search"). Nothing human enters.

use board::{Color, Position};
use nnue::Net;

use crate::datagen::Sample;

pub struct Trainer {
    pub lr: f32,
    /// Weight on the engine's own search score vs the raw game outcome. The outcome is a very
    /// noisy label for an early position; the root score is lower-variance and self-derived.
    pub blend: f32,
}

impl Trainer {
    pub fn new(lr: f32, blend: f32) -> Self {
        Trainer { lr, blend }
    }

    /// White-POV forward pass and the blended target for one sample, shared by training and by
    /// the held-out surrogate so the two cannot drift apart. FITNESS 5 compares a candidate to
    /// the champion on "loss on the blended target"; if the surrogate computed a DIFFERENT loss
    /// from the one being optimised, it would be measuring a quantity nothing is minimising.
    fn forward_and_target(&self, net: &Net, s: &Sample, idx: &mut Vec<u16>, acc: &mut Vec<f32>)
        -> Option<(f32, f32)>
    {
        let pos = Position::from_fen(&s.fen).ok()?;
        let h = net.n_hidden;
        acc.resize(h, 0.0);
        Net::active(&pos, idx);
        acc.copy_from_slice(&net.b1);
        for &f in idx.iter() {
            let row = &net.w1[f as usize * h..(f as usize + 1) * h];
            for (a, w) in acc.iter_mut().zip(row) { *a += *w; }
        }
        let mut out = net.b2;
        for k in 0..h {
            if acc[k] > 0.0 { out += acc[k] * net.w2[k]; }
        }
        let pred = out.tanh();
        let root_white = if pos.stm == Color::White { s.root } else { -s.root };
        let root = (root_white as f32 / net.scale).tanh();
        let target = (1.0 - self.blend) * s.z + self.blend * root;
        Some((pred, target))
    }

    /// Mean squared error on a held-out set. No update. This is the FITNESS 5 surrogate.
    pub fn loss(&self, net: &Net, data: &[&Sample]) -> f64 {
        let (mut idx, mut acc) = (Vec::with_capacity(40), Vec::new());
        let (mut total, mut n) = (0.0f64, 0usize);
        for s in data {
            if let Some((pred, target)) = self.forward_and_target(net, s, &mut idx, &mut acc) {
                let e = (pred - target) as f64;
                total += e * e;
                n += 1;
            }
        }
        if n == 0 { f64::INFINITY } else { total / n as f64 }
    }

    /// PAIRED significance on held-out squared error: positive z means `cand` beats `champ` by
    /// more than the sample-to-sample noise, on the SAME positions.
    ///
    /// Comparing two mean losses by eye ("0.5579 vs 0.9376, clearly better") is the same mistake
    /// as reading a gate's point estimate without its interval. The two nets are scored on
    /// identical positions, so the per-position DIFFERENCE is the statistic with the variance
    /// that matters; most of the spread is the position, and it cancels.
    pub fn paired_loss_z(&self, champ: &Net, cand: &Net, data: &[&Sample]) -> f64 {
        let (mut idx, mut acc) = (Vec::with_capacity(40), Vec::new());
        let mut d: Vec<f64> = Vec::with_capacity(data.len());
        for s in data {
            let a = self.forward_and_target(champ, s, &mut idx, &mut acc);
            let b = self.forward_and_target(cand, s, &mut idx, &mut acc);
            if let (Some((pa, ta)), Some((pb, tb))) = (a, b) {
                // champ error minus cand error: > 0 when the candidate is closer.
                d.push(((pa - ta) as f64).powi(2) - ((pb - tb) as f64).powi(2));
            }
        }
        let n = d.len();
        if n < 30 { return 0.0; }
        let mean = d.iter().sum::<f64>() / n as f64;
        let var = d.iter().map(|x| (x - mean) * (x - mean)).sum::<f64>() / (n as f64 - 1.0);
        if var <= 0.0 { return if mean > 0.0 { f64::INFINITY } else { 0.0 }; }
        mean / (var / n as f64).sqrt()
    }

    /// SGD over a FIXED NUMBER OF SAMPLES, drawn from `data` regardless of how large `data` is.
    ///
    /// WHY THIS EXISTS. `epoch` walks the whole dataset, so the number of gradient steps per
    /// generation is whatever datagen happened to produce. That was harmless while a generation
    /// yielded ~200 training positions. Raising self-play volume 125x took it to ~110,000, so
    /// the step count per generation grew with it and the champion was being overwritten every
    /// cycle: measured gate rate 0.469 -> 0.398 and held-out McNemar z down to -13.31 across
    /// generations 13-18, i.e. the gate and the surrogate agreeing the candidate was worse,
    /// while training loss kept falling.
    ///
    /// The volume increase was right -- it is what made the gate resolve at all. What was wrong
    /// was leaving the trainer's step budget coupled to it. This decouples them: more data now
    /// means a more DIVERSE draw, not a longer one.
    ///
    /// Sampling is with replacement from a seeded stream, so a large replay buffer contributes
    /// broadly rather than the loop grinding the most recent generation.
    pub fn steps(&self, net: &mut Net, data: &[Sample], n_steps: usize, rng_seed: u64) -> f32 {
        if data.is_empty() || n_steps == 0 { return 0.0; }
        let mut r = rng_seed | 1;
        let mut idx: Vec<usize> = Vec::with_capacity(n_steps);
        for _ in 0..n_steps {
            r ^= r << 13; r ^= r >> 7; r ^= r << 17;
            idx.push((r % data.len() as u64) as usize);
        }
        let picked: Vec<Sample> = idx.into_iter().map(|i| data[i].clone()).collect();
        self.epoch(net, &picked, rng_seed ^ 0xA5A5)
    }

    /// One epoch of SGD. Returns mean squared error before the update.
    pub fn epoch(&self, net: &mut Net, data: &[Sample], rng_seed: u64) -> f32 {
        let mut order: Vec<usize> = (0..data.len()).collect();
        let mut r = rng_seed | 1;
        for i in (1..order.len()).rev() {
            r ^= r << 13; r ^= r >> 7; r ^= r << 17;
            order.swap(i, (r % (i as u64 + 1)) as usize);
        }

        let h = net.n_hidden;
        let mut idx: Vec<u16> = Vec::with_capacity(40);
        let mut acc = vec![0f32; h];
        let mut total = 0f32;

        for &i in &order {
            let s = &data[i];
            let pos = match Position::from_fen(&s.fen) {
                Ok(p) => p,
                Err(_) => continue,
            };
            Net::active(&pos, &mut idx);

            // forward
            acc.copy_from_slice(&net.b1);
            for &f in &idx {
                let row = &net.w1[f as usize * h..(f as usize + 1) * h];
                for (a, w) in acc.iter_mut().zip(row) { *a += *w; }
            }
            let mut out = net.b2;
            for k in 0..h {
                if acc[k] > 0.0 { out += acc[k] * net.w2[k]; }
            }
            let pred = out.tanh();

            // FRAME: the network's raw output is WHITE-POV -- Net::eval computes a white-POV
            // value and applies the mover flip only at the very end. So the target here must be
            // white-POV too. Training this output toward a MOVER-relative target asks the same
            // output to be +m and -m for the same material, which is contradictory, and the
            // cheapest solution is to predict the mean. That bug made the trainer unable to fit
            // even material -- a target linear in its own inputs (examples/trainer_control.rs).
            let z_white = s.z;
            // s.root is mover-relative (it comes from the search), so bring it to white-POV.
            let root_white = if pos.stm == Color::White { s.root } else { -s.root };
            let root = (root_white as f32 / net.scale).tanh();
            let target = (1.0 - self.blend) * z_white + self.blend * root;

            let err = pred - target;
            total += err * err;

            // backward
            let d_out = 2.0 * err * (1.0 - pred * pred);
            net.b2 -= self.lr * d_out;
            for k in 0..h {
                if acc[k] > 0.0 {
                    let d_w2 = d_out * acc[k];
                    let d_acc = d_out * net.w2[k];
                    net.w2[k] -= self.lr * d_w2;
                    net.b1[k] -= self.lr * d_acc;
                    for &f in &idx {
                        net.w1[f as usize * h + k] -= self.lr * d_acc;
                    }
                }
            }
        }
        total / data.len().max(1) as f32
    }
}
