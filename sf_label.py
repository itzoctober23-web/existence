#!/usr/bin/env python3
"""CONTROL 2, part 1: label positions with STOCKFISH's eval instead of self-play outcome.

WHY. The absolute ruler says the champion is ~1182 Elo and flat across 1200 generations. That is
consistent with two very different failures, and they need different fixes:

  * the TRAINER / net / feature encoding is broken -> no labels would help
  * the trainer is fine and the SELF-PLAY LABELS carry no signal -> the fix is datagen (deeper
    search, longer games), not the learner

Substituting a known-good label source separates them. If the same architecture trained on SF-labeled
positions jumps well above 1182, the pipeline works and the labels are the problem. If it also sits
at ~1182, the learner itself is broken.

SPEC POSITION. This is METHODOLOGY, not training input for the real engine -- the same standing as
using Stockfish as a ruler. The net produced here is a diagnostic artefact and must never be shipped
or fed back into the loop. MASTER_PLAN's tabula-rasa constraint is about what the ENGINE learns from,
and this net is a test instrument, like the perft oracle.

WHAT IT WRITES. One line per position: `fen<TAB>root_cp` where root_cp is SF's score in centipawns
from the MOVER's point of view -- matching `Sample.root`, which trainer.rs converts with
`root_white = if stm == White { root } else { -root }`. Getting that sign convention wrong would
train the net to invert, which is exactly the kind of silent defect this control exists to detect,
so the sign is taken straight from python-chess's PovScore relative to the side to move.
"""
import argparse, random, sys, time
import chess, chess.engine

SF = "/usr/bin/stockfish"


def positions(n, seed, min_ply=4, max_ply=60):
    """Random legal walks from startpos, the same way the mate sets are mined.

    Deliberately NOT the loop's own datagen output: those positions were produced by a near-random
    self-play engine, and reusing them would confound "different labels" with "different positions".
    A uniform random walk is a neutral position source for both arms.
    """
    rng = random.Random(seed)
    out = []
    while len(out) < n:
        b = chess.Board()
        depth = rng.randint(min_ply, max_ply)
        for _ in range(depth):
            ms = list(b.legal_moves)
            if not ms:
                break
            b.push(rng.choice(ms))
        if b.is_game_over(claim_draw=True):
            continue
        out.append(b.fen())
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--n", type=int, default=30000)
    ap.add_argument("--nodes", type=int, default=20000)
    ap.add_argument("--seed", type=int, default=77)
    ap.add_argument("--out", default="sf_labels.tsv")
    a = ap.parse_args()

    fens = positions(a.n, a.seed)
    eng = chess.engine.SimpleEngine.popen_uci(SF)
    eng.configure({"Threads": 1, "Hash": 64})
    t0 = time.time()
    written = skipped = 0
    with open(a.out, "w") as f:
        for i, fen in enumerate(fens):
            b = chess.Board(fen)
            try:
                info = eng.analyse(b, chess.engine.Limit(nodes=a.nodes))
            except chess.engine.EngineError:
                skipped += 1
                continue
            sc = info["score"].relative          # MOVER-relative, matching Sample.root
            cp = sc.score(mate_score=30000)
            if cp is None:
                skipped += 1
                continue
            f.write(f"{fen}\t{cp}\n")
            written += 1
            if (i + 1) % 2000 == 0:
                el = time.time() - t0
                print(f"  {i+1}/{len(fens)}  {written} written  {skipped} skipped  "
                      f"{el:.0f}s  ({(i+1)/el:.0f} pos/s)", flush=True)
    eng.quit()
    print(f"  DONE {written} labelled, {skipped} skipped, {time.time()-t0:.0f}s -> {a.out}")


main()
