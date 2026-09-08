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
}

impl Reason {
    pub fn as_str(self) -> &'static str {
        match self {
            Reason::Accepted => "accepted",
            Reason::SurrogateFilter => "surrogate_filter",
            Reason::LostOnGames => "lost_on_games",
            Reason::NoEvidence => "no_evidence",
            Reason::LostOnClock => "lost_on_clock",
            Reason::LostOnCost => "lost_on_cost",
            Reason::Regression => "regression",
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

pub struct Ledger {
    path: String,
}

impl Ledger {
    pub fn new(path: &str) -> Self {
        Ledger { path: path.to_string() }
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
        s.push_str("}\n");

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
