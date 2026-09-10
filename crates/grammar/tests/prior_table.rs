//! GRAMMAR 6 states this project's central claim -- the direction and size of the declared prior --
//! in a table of node counts labelled **MEASURED**. Nothing verified that label, and it drifted.
//!
//! Found 2026-09-10: the table said `capture extension (rung 6) | 80 | +9` while the parser printed
//! **84 / +13**. Commit 064112f ("rung 6 fixed to extend AT THE HORIZON") had deliberately grown the
//! program by 4 nodes on 09-08 20:29; the table row was last written at 09-08 17:08 and never
//! followed. Nine of ten rows were right, which is what made it invisible.
//!
//! The document had ALREADY been bitten by this once and had already tried to fix it. Its own text
//! says four of the nine programs "were quoted only as prose in section 9, where they could drift
//! from the counter without anything failing", and the response was to move them INTO the table.
//! That relocated the drift, it did not gate it -- see the standing rule that a check must GATE, not
//! decorate. `cargo run --example prior` printing the truth does not help if nothing compares its
//! output to what the document claims.
//!
//! So this test parses the GRAMMAR 6 table and demands EXACT agreement with `reference::all()`.
//!
//! WHY THE DOC USES THE CANONICAL NAMES. The obvious implementation matches doc rows to programs by
//! fuzzy name ("UCT MCTS (declared, sum selection)" vs `UCT-style MCTS`) through an alias map. That
//! is a hand-kept list of names, which is the exact defect that has now bitten this workspace three
//! separate ways in one day: a hardcoded arm-name alternation in tick.sh missed a probe; a whitelist
//! grep dropped four alarms and had to be deleted; and a `pgrep -x evolve` enumerator reported 5 arms
//! while 6 ran because one had been renamed. An alias map here would fail the same way -- silently,
//! and in the direction of "everything matches".
//!
//! Instead the table carries the names `reference::all()` returns, and matching is exact string
//! equality. A rename in the code then FORCES a doc edit, which is the gate we want.

use std::collections::BTreeMap;

/// Pull `(name, nodes)` out of the GRAMMAR 6 markdown table.
///
/// Rows look like `| depth-one (purity seed) | 9 | -62 | faithful (purity seed) |`, and any cell may
/// be wrapped in `**` for emphasis, so both name and number are stripped of asterisks before use.
/// ANCHOR ON THE HEADER, NOT ON THE SECTION. The first version of this took every `|`-row inside
/// section 6, and section 6 contains a SECOND table -- the UCT budget sweep, whose rows are
/// `| 16 | 1 | ... |`. Those parsed happily as "a program named 1 with 16 nodes", and the test
/// failed on the corrected document with six bogus STALE rows.
///
/// That mistake is the reason this file has a two-armed verifier: arm A (expect PASS on the true
/// document) caught it immediately. Had I only run the drift control -- arm B, which correctly
/// reported `DRIFTED: capture extension -- table says 80, parser says 84` -- I would have seen a
/// working gate and shipped a broken one.
fn doc_rows(md: &str) -> BTreeMap<String, usize> {
    let mut out = BTreeMap::new();
    let mut lines = md.lines().skip_while(|l| !l.contains("| Nodes (MEASURED) |"));
    let header = lines.next();
    assert!(
        header.is_some(),
        "GRAMMAR.md has no table header containing '| Nodes (MEASURED) |' -- the prior table was \
         renamed or removed. Refusing to report an empty table as agreement."
    );
    for line in lines {
        let line = line.trim();
        // Consume the CONTIGUOUS run of table rows and stop at the first line that is not one. This
        // is what confines the parse to a single table.
        if !line.starts_with('|') {
            break;
        }
        let cells: Vec<&str> = line.trim_matches('|').split('|').map(str::trim).collect();
        if cells.len() < 3 {
            continue;
        }
        let name = cells[0].replace("**", "");
        let nodes = cells[1].replace("**", "");
        // The `|---|` separator's cells do not parse as a number, so it drops out here rather than
        // by position -- a table gaining a row would shift a positional skip.
        if let Ok(n) = nodes.trim().parse::<usize>() {
            out.insert(name.trim().to_string(), n);
        }
    }
    out
}

#[test]
fn grammar6_table_matches_the_parser() {
    let md_path = concat!(env!("CARGO_MANIFEST_DIR"), "/../../docs/GRAMMAR.md");
    let md = std::fs::read_to_string(md_path)
        .unwrap_or_else(|e| panic!("cannot read {md_path}: {e}"));

    let measured: BTreeMap<String, usize> = grammar::reference::all()
        .into_iter()
        .map(|(n, p)| (n.to_string(), p.size()))
        .collect();
    let documented = doc_rows(&md);

    // A parse that finds nothing is a broken parser, not an empty table. Without this, a heading
    // rename would make the whole test pass vacuously -- the failure mode where a check reports
    // "all clear" precisely because it looked at nothing.
    assert!(
        documented.len() >= measured.len(),
        "parsed only {} rows from the GRAMMAR 6 table but {} reference programs exist -- the parser \
         is broken (heading or table format changed), NOT the table",
        documented.len(),
        measured.len()
    );

    let mut problems = Vec::new();
    for (name, want) in &measured {
        match documented.get(name) {
            None => problems.push(format!(
                "  MISSING from the GRAMMAR 6 table: {name:?} (parser says {want} nodes).\n    \
                 The table must use the exact names reference::all() returns."
            )),
            Some(got) if got != want => problems.push(format!(
                "  DRIFTED: {name:?} -- table says {got}, parser says {want}"
            )),
            Some(_) => {}
        }
    }
    for name in documented.keys() {
        if !measured.contains_key(name) {
            problems.push(format!(
                "  STALE row in the GRAMMAR 6 table: {name:?} is not a program reference::all() \
                 returns (renamed or removed?)"
            ));
        }
    }

    assert!(
        problems.is_empty(),
        "GRAMMAR 6 is labelled MEASURED but disagrees with `cargo run --release --example prior \
         -p grammar`:\n{}\n\nFix the DOCUMENT to match the parser, never the reverse.",
        problems.join("\n")
    );
}

/// The prior's *direction* is the claim GRAMMAR 6 actually makes, and it is what a future change to
/// the reference programs could quietly invert. Counts agreeing row-by-row does not assert it.
#[test]
fn every_alpha_beta_variant_is_nearer_the_seed_than_either_rival_paradigm() {
    let progs = grammar::reference::all();
    let size = |needle: &str| -> i64 {
        progs
            .iter()
            .find(|(n, _)| *n == needle)
            .unwrap_or_else(|| panic!("reference::all() has no program named {needle:?}"))
            .1
            .size() as i64
    };

    let seed = size("bare alpha-beta (main seed)");
    let variants = [
        "capture extension (rung 6)",
        "table reduction (rung 7)",
        "alpha-beta + iterative deepening",
    ];
    let rivals = ["UCT-style MCTS", "proof-number search"];

    let worst_variant = variants.iter().map(|n| size(n) - seed).max().unwrap();
    let nearest_rival = rivals.iter().map(|n| size(n) - seed).min().unwrap();

    assert!(
        worst_variant < nearest_rival,
        "GRAMMAR 6 claims the grammar is skewed toward alpha-beta, but the furthest alpha-beta \
         variant is +{worst_variant} from the seed while the nearest rival paradigm is only \
         +{nearest_rival}. The skew claim no longer holds and GRAMMAR 6 must be rewritten."
    );
}
