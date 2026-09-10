#!/usr/bin/env python3
"""AN ABSOLUTE RULER FOR EXISTENCE: our net vs Stockfish, Elo on one scale.

WHY THIS EXISTS. Every number in this repo is RELATIVE -- candidate vs champion, arm vs origin,
accept vs reject, rung vs rung. None of them says what the champion is actually worth, so a
plateau at "0.83 against the frozen origin" is unreadable: it could be a real ceiling near some
rating, or a relative instrument saturating. FITNESS's P1 milestone is stated as "~2000 vs
SF-limited" and has never been measured.

DESIGN, and the parts that matter:

* OUR ENGINE IS FIXED-DEPTH. `crates/engine` ignores go parameters and searches to its `Depth`
  option, so the measurement is "our net at depth D" -- which is the project's own strength
  standard (gate_depth_cap = 4). It is NOT a time-control result and must never be quoted as one.

* STOCKFISH IS WEAKENED TWO WAYS, because one is not enough. UCI_LimitStrength floors at Elo 1320,
  and a 12k-weight net at depth 4 may be far below that -- in which case every game is a loss and
  the reading is a FLOOR, not a rating. So SF also takes a node limit, which weakens it below its
  own floor. `--calibrate` finds a node count where the match is actually contested before any
  real measurement is spent.

* COLOURS ALTERNATE and openings are shared, so each opening is played from both sides. Opening
  bias cancels rather than being averaged over.

* ELO IS REPORTED WITH ITS INTERVAL, and a score of 0 or 1 is reported as a BOUND, never as a
  number -- a clean sweep means "below/above this, unmeasured", which is exactly the floor case
  this harness is built to notice.
"""
import argparse, math, os, random, sys, time
import chess, chess.engine

ENGINE = "/tmp/claude-1000/-home-maswabe/368f9dad-1623-4171-ab55-c7e97167e24e/scratchpad/xt_cap/release/engine"
SF = "/usr/bin/stockfish"


def elo_diff(w, d, l):
    n = w + d + l
    if n == 0:
        return None, None
    s = (w + 0.5 * d) / n
    if s <= 0.0 or s >= 1.0:
        return None, None          # a sweep is a bound, not a rating
    e = -400.0 * math.log10(1.0 / s - 1.0)
    # standard error on the score, converted through the logistic slope at s
    var = (w * (1 - s) ** 2 + d * (0.5 - s) ** 2 + l * (0 - s) ** 2) / max(1, n - 1)
    se_s = math.sqrt(var / n)
    slope = 400.0 / (math.log(10) * s * (1 - s))
    return e, 1.96 * se_s * slope


def play(net, depth, sf_elo, sf_nodes, games, seed, plycap=300, movetime=None):
    os.environ["EXISTENCE_NET"] = net
    ours = chess.engine.SimpleEngine.popen_uci(ENGINE)
    ours.configure({"Depth": depth})
    sf = chess.engine.SimpleEngine.popen_uci(SF)
    sf.configure({"UCI_LimitStrength": True, "UCI_Elo": sf_elo, "Threads": 1, "Hash": 16})
    rng = random.Random(seed)
    w = d = l = 0
    try:
        for g in range(games):
            b = chess.Board()
            # Shared random opening, replayed for BOTH colours so bias cancels.
            if g % 2 == 0:
                open_moves = []
                for _ in range(rng.choice([2, 4, 6])):
                    ms = list(b.legal_moves)
                    if not ms:
                        break
                    mv = rng.choice(ms)
                    open_moves.append(mv)
                    b.push(mv)
                last_open = open_moves
            else:
                b = chess.Board()
                for mv in last_open:
                    b.push(mv)
            we_are_white = (g % 2 == 0)
            plies = 0
            while not b.is_game_over(claim_draw=True) and plies < plycap:
                mine = (b.turn == chess.WHITE) == we_are_white
                try:
                    if mine:
                        # TIME CONTROL when asked for, fixed depth otherwise.
                        #
                        # Until 2026-09-10 the engine ignored go parameters entirely, so this could
                        # only ever be a fixed-depth reading and every speed change measured 0 Elo by
                        # construction (`speed_cannot_pay_RESULT.md`). The engine now converts a
                        # movetime into a node budget and picks its depth from it, so a FASTER engine
                        # genuinely searches deeper here -- which is the only way an eval optimisation
                        # can show up as Elo.
                        r = (ours.play(b, chess.engine.Limit(time=movetime / 1000.0))
                             if movetime else ours.play(b, chess.engine.Limit(depth=depth)))
                    else:
                        r = sf.play(b, chess.engine.Limit(nodes=sf_nodes))
                except chess.engine.EngineError:
                    r = None
                if r is None or r.move is None or r.move not in b.legal_moves:
                    # A side that cannot answer forfeits; scoring it a draw would let a broken
                    # engine gate as "equal".
                    if mine:
                        l += 1
                    else:
                        w += 1
                    break
                b.push(r.move)
                plies += 1
            else:
                res = b.result(claim_draw=True)
                if res == "1/2-1/2" or plies >= plycap:
                    d += 1
                elif (res == "1-0") == we_are_white:
                    w += 1
                else:
                    l += 1
                continue
    finally:
        ours.quit()
        sf.quit()
    return w, d, l


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--net", required=True)
    ap.add_argument("--depth", type=int, default=4)
    ap.add_argument("--sf-elo", type=int, default=1320)
    ap.add_argument("--sf-nodes", type=int, default=1000)
    ap.add_argument("--games", type=int, default=40)
    ap.add_argument("--seed", type=int, default=20260910)
    ap.add_argument("--movetime", type=int, default=None,
                    help="milliseconds per move for OUR engine. Measures strength on a CLOCK rather "
                         "than at a fixed depth, which is what MASTER_PLAN line 43 specifies for the "
                         "gate and the only setting in which a speedup can become Elo.")
    ap.add_argument("--calibrate", action="store_true",
                    help="sweep SF node counts to find a contested setting before spending games")
    a = ap.parse_args()

    if a.calibrate:
        print(f"  CALIBRATION on {os.path.basename(a.net)} at depth {a.depth}: finding a contested SF node count")
        print(f"  (a sweep means the reading is a BOUND, not a rating -- that is what this avoids)")
        for nodes in (1, 4, 16, 64, 256, 1024):
            t0 = time.time()
            w, d, l = play(a.net, a.depth, a.sf_elo, nodes, 10, a.seed)
            s = (w + 0.5 * d) / max(1, w + d + l)
            print(f"    sf nodes {nodes:>5}: {w}W-{d}D-{l}L  score {s:.3f}  ({time.time()-t0:.0f}s)")
        return

    t0 = time.time()
    w, d, l = play(a.net, a.depth, a.sf_elo, a.sf_nodes, a.games, a.seed, movetime=a.movetime)
    n = w + d + l
    s = (w + 0.5 * d) / max(1, n)
    e, ci = elo_diff(w, d, l)
    print(f"  net      {os.path.basename(a.net)}")
    setting = f"movetime {a.movetime}ms" if a.movetime else f"depth {a.depth}"
    print(f"  {setting}  vs SF elo {a.sf_elo} nodes {a.sf_nodes}  {n} games  ({time.time()-t0:.0f}s)")
    print(f"  W-D-L {w}-{d}-{l}   score {s:.4f}")
    if e is None:
        print(f"  ELO: a {'clean sweep' if s in (0.0,1.0) else 'degenerate score'} -- this is a BOUND, not a rating.")
        print(f"       Our engine is {'far below' if s == 0.0 else 'far above'} this setting; re-run weaker/stronger.")
    else:
        print(f"  Elo vs this opponent: {e:+.0f} +/- {ci:.0f}")


main()
