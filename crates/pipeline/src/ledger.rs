//! The ledger: what was tried, what happened, and WHY IT FAILED.
//!
//! MASTER_PLAN "Self-documentation" specifies a ledger written by the system at acceptance
//! time, and it was not implemented — `grep -rn ledger crates/**/*.rs` returned nothing. The
//! only record of a run was stdout, which the next run overwrote. Today's ARCH run made 10
//! architecture proposals and 6 of them were rejected; all 6 are gone.
//!
//! TWO DELIBERATE DEPARTURES FROM THE SPEC, both toward recording more:
//!
//! 1. **Rejections are recorded, not just acceptances.** The spec says "Every ACCEPTED change
//!    is entered in the ledger". But a rejection is the more reusable fact: it says do not
//!    spend the compute again. A ledger of successes only is a list of what worked with no
//!    memory of what was already ruled out, which is how the same dead end gets retried.
//!
//! 2. **Every rejection carries a NAMED REASON, not a boolean.** "reject" is not a finding.
//!    "the surrogate filter refused it before any games were played" and "it won on equal
//!    nodes and lost on the clock" are different facts with different follow-ups — the second
//!    is the degenerate case FITNESS 10 names ("bigger net that wins fixed-cost-budget, loses
//!    on clock"), the first means the candidate never reached a gate at all.
//!
//! Format is JSONL, appended, one record per DECISION. Append-only because a ledger that can be
//! rewritten is not evidence; JSONL because it survives a killed process mid-write with at most
//! the last line lost.
//!
//! Hand-rolled serialisation on purpose: this crate has no serde dependency, and a ledger is
//! not worth adding one for. The escaper below handles the only cases these records produce.

use std::io::Write;

/// Why a candidate did not become the champion. The point of the enum is that adding a new
/// rejection path forces a new variant — a catch-all string would let a future reason land as
/// an unsearchable free-text blob.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// Accepted; no failure.
    Accepted,
    /// FITNESS 5: held-out loss worse than the champion by more than the allowed margin. The
    /// candidate never reached a gate, so no game evidence exists about it.
    SurrogateFilter,
    /// The gate could resolve, and the candidate's interval did not clear 0.5.
    LostOnGames,
    /// Beat the champion but was resolved WORSE against the fixed anchor, so the promotion was
    /// vetoed. This is the intransitive case, and it is a distinct outcome from losing the
    /// champion match: measured 2026-09-08, ep_1 beat champion_long 0.545 +/- 0.018 head to head
    /// while scoring 0.834 against the origin where champion_long scored 0.864. Without its own
    /// reason code these would be filed as NoEvidence and the cycle would stay invisible.
    AnchorRegression,
    /// Gates could NOT resolve (all-draw or near it), and the surrogate did not show a
    /// significant improvement either. No evidence in either direction.
    NoEvidence,
    /// Won on equal cost, lost on equal time. FITNESS 10's named degenerate case.
    LostOnClock,
    /// Won on equal time but not on equal cost — the reverse, which usually means the win was
    /// a speed artefact rather than eval quality.
    LostOnCost,
    /// Regressed against the champion on a non-regression guard.
    Regression,
    /// PROGRAM class only: the candidate disagreed with the full-width reference search, i.e.
    /// it returned a move the reference says is not optimal. Distinct from SurrogateFilter,
    /// which is about held-out LOSS -- a PROGRAM candidate has no held-out loss, and labelling
    /// its correctness failure with the net arm's explanation puts a false sentence in an
    /// evidence file.
    FailedOracle,
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::Accepted => "accepted",
            Reason::SurrogateFilter => "surrogate_filter",
            Reason::LostOnGames => "lost_on_games",
            Reason::AnchorRegression => "anchor_regression",
            Reason::NoEvidence => "no_evidence",
            Reason::LostOnClock => "lost_on_clock",
            Reason::LostOnCost => "lost_on_cost",
            Reason::Regression => "regression",
            Reason::FailedOracle => "failed_oracle",
        }
    }
    /// One sentence a human can read without the schema in front of them.
    pub fn explain(self) -> &'static str {
        match self {
            Reason::Accepted => "cleared every check it was subject to",
            Reason::SurrogateFilter =>
                "held-out loss was worse than the champion's, so it never reached a gate",
            Reason::LostOnGames =>
                "the gate could resolve and its interval did not clear 0.5",
            Reason::AnchorRegression =>
                "beat the champion but was resolved WORSE against the fixed anchor: an \
                 intransitive candidate, stronger than its parent and weaker than the parent \
                 is against a third opponent",
            Reason::NoEvidence =>
                "the gate could not resolve and the surrogate showed nothing significant, \
                 so there is no evidence either way",
            Reason::LostOnClock =>
                "won on equal nodes and lost on equal time: an eval too expensive for what \
                 it knows (FITNESS 10)",
            Reason::LostOnCost =>
                "won on equal time but not equal nodes, so the win looks like speed rather \
                 than eval quality",
            Reason::Regression => "regressed against the champion on a non-regression guard",
            Reason::FailedOracle =>
                "disagreed with the full-width reference search, so it is not computing the \
                 value it claims to -- 'cheaper' is trivial if you are allowed to be wrong",
        }
    }
}

/// One decision. Fields that do not apply to a class are simply omitted from the JSON.
pub struct Entry {
    /// Field is `generation`, not `gen`: `gen` is a reserved keyword in this edition. The
    /// JSON key stays "gen" so the on-disk schema is unaffected.
    pub generation: usize,
    /// FITNESS 1's candidate classes: NET, ARCH, PROGRAM, ...
    pub class: &'static str,
    /// What changed, in the engine's own terms ("width 128 -> 256", "epoch 3 on 412 samples").
    pub what: String,
    pub reason: Reason,
    /// Gate evidence. `None` where a gate was never run (surrogate filter).
    pub gates: Vec<GateEvidence>,
    /// Surrogate evidence: (name, value) pairs, e.g. held-out loss and the paired z.
    pub surrogate: Vec<(&'static str, f64)>,
    /// SCHEMAS 8 `earned.e1_at_acceptance` -- "bar provably cleared; same quantity the bandit
    /// uses". `None` where no bound was in force.
    pub e1: Option<f64>,
    /// SCHEMAS 8 `where_it_mattered.top_disagreements[0].fen_or_board_hash`: the position where
    /// this candidate most changed the evaluation. `None` when the caller had no position set in
    /// hand -- omitted rather than faked, since an invented FEN is worse than an absent one.
    pub top_disagreement_fen: Option<String>,
}

pub struct GateEvidence {
    pub name: &'static str,
    pub pent: [u32; 5],
    pub rate: f64,
    pub ci95: f64,
    /// Whether this gate had enough resolution for its verdict to mean anything. A gate that
    /// could not resolve is recorded WITH that fact, because "0.500" from an all-draw match
    /// and "0.500" from a decisive one are not the same observation.
    pub resolved: bool,
}

fn esc(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            '"' => vec!['\\', '"'],
            '\\' => vec!['\\', '\\'],
            '\n' => vec!['\\', 'n'],
            c => vec![c],
        })
        .collect()
}

fn num(x: f64) -> String {
    if x.is_finite() { format!("{x:.6}") } else { "null".into() }
}

/// PROQUINT of a 32-bit value -- MASTER_PLAN "Versioning", via SCHEMAS 7: `name` = proquint of the
/// first 32 bits of `ledger_hash`. Two syllables of consonant-vowel-consonant-vowel-consonant.
///
/// The point of the scheme is that a champion is referred to by a pronounceable name nobody chose,
/// derived from the hash of everything that produced it. A hand-picked name would be a claim; this
/// is an address.
fn proquint(mut x: u32) -> String {
    const C: [u8; 16] = *b"bdfghjklmnprstvz";
    const V: [u8; 4] = *b"aiou";
    let mut out = Vec::with_capacity(11);
    for i in 0..2 {
        if i > 0 { out.push(b'-'); }
        let w = (x >> 16) as u16;
        x <<= 16;
        out.push(C[(w >> 12 & 0xF) as usize]);
        out.push(V[(w >> 10 & 0x3) as usize]);
        out.push(C[(w >> 6 & 0xF) as usize]);
        out.push(V[(w >> 4 & 0x3) as usize]);
        out.push(C[(w & 0xF) as usize]);
    }
    String::from_utf8(out).expect("ascii")
}

pub struct Ledger {
    path: String,
    /// Entry count and running hash, recovered from the file on construction so a resumed run
    /// continues the same series rather than restarting it.
    index: std::cell::Cell<usize>,
    hash: std::cell::Cell<u64>,
}

impl Ledger {
    pub fn new(path: &str) -> Self {
        // Recover the series from disk. A fresh `Ledger` on a resumed run must not restart the
        // index at 0, or two different champions would carry the same identity string.
        let (mut idx, mut h) = (0usize, 0xcbf29ce484222325u64);
        if let Ok(txt) = std::fs::read_to_string(path) {
            for line in txt.lines().filter(|l| !l.trim().is_empty()) {
                idx += 1;
                for b in line.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            }
        }
        Ledger { path: path.to_string(), index: std::cell::Cell::new(idx), hash: std::cell::Cell::new(h) }
    }

    /// Append one decision. Failure to write is reported but never fatal: losing the ledger
    /// must not kill a training run, and a silent failure would be worse than either.
    pub fn record(&self, e: &Entry) {
        let mut s = String::with_capacity(512);
        s.push('{');
        s.push_str(&format!("\"gen\":{},", e.generation));
        s.push_str(&format!("\"class\":\"{}\",", esc(e.class)));
        s.push_str(&format!("\"what\":\"{}\",", esc(&e.what)));
        s.push_str(&format!("\"verdict\":\"{}\",", if e.reason == Reason::Accepted { "accept" } else { "reject" }));
        s.push_str(&format!("\"reason\":\"{}\",", e.reason.as_str()));
        s.push_str(&format!("\"why\":\"{}\"", esc(e.reason.explain())));
        if !e.surrogate.is_empty() {
            s.push_str(",\"surrogate\":{");
            for (i, (k, v)) in e.surrogate.iter().enumerate() {
                if i > 0 { s.push(','); }
                s.push_str(&format!("\"{}\":{}", esc(k), num(*v)));
            }
            s.push('}');
        }
        if !e.gates.is_empty() {
            s.push_str(",\"gates\":[");
            for (i, g) in e.gates.iter().enumerate() {
                if i > 0 { s.push(','); }
                s.push_str(&format!(
                    "{{\"name\":\"{}\",\"pent\":[{},{},{},{},{}],\"rate\":{},\"ci95\":{},\"resolved\":{}}}",
                    esc(g.name), g.pent[0], g.pent[1], g.pent[2], g.pent[3], g.pent[4],
                    num(g.rate), num(g.ci95), g.resolved
                ));
            }
            s.push(']');
        }
        if let Some(e1) = e.e1 {
            s.push_str(&format!(",\"earned\":{{\"e1_at_acceptance\":{}}}", num(e1)));
        }
        if let Some(fen) = &e.top_disagreement_fen {
            s.push_str(&format!(
                ",\"where_it_mattered\":{{\"top_disagreements\":[{{\"fen_or_board_hash\":\"{}\"}}]}}",
                esc(fen)));
        }
        // IDENTITY STRING, rendered here and never typed (SCHEMAS 7). Appended last so it hashes
        // over the same content every reader sees.
        let idx = self.index.get() + 1;
        let name = proquint((self.hash.get() >> 32) as u32);
        let cleared = match e.e1 { Some(v) => format!(" (cleared e1={v:.1})"), None => String::new() };
        s.push_str(&format!(",\"identity_string\":\"Existence {} {} — {}{}\"",
                            idx, name, esc(&e.what), cleared));
        s.push_str("}\n");
        self.index.set(idx);
        {
            let mut h = self.hash.get();
            for b in s.as_bytes() { h ^= *b as u64; h = h.wrapping_mul(0x100000001b3); }
            self.hash.set(h);
        }

        match std::fs::OpenOptions::new().create(true).append(true).open(&self.path) {
            Ok(mut f) => {
                if let Err(err) = f.write_all(s.as_bytes()) {
                    eprintln!("  WARN ledger write failed ({}): {err}", self.path);
                }
            }
            Err(err) => eprintln!("  WARN ledger open failed ({}): {err}", self.path),
        }
    }
}
