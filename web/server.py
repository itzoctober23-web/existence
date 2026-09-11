#!/usr/bin/env python3
"""Existence loader — play and analyse against the Existence engine.

The 2-player counterpart of the Tetrarch 4PC loader. Same API names, same board, same eval bar,
so anything built against the 4PC one keeps working:

    POST /api/new       start a game            GET /api/state    board + eval + history
    POST /api/move      play a move             GET /api/pgn      export
    POST /api/resign    resign                  POST /api/import  load a PGN
    GET  /api/analyse   engine eval of one ply  GET /api/review   eval of every ply (the graph)

ONE DELIBERATE DIFFERENCE FROM THE 4PC LOADER. There, `web/server.py:6` records that "the engine is
BOTH the rules oracle (legal moves) and the opponent", because 4-player chess has no library. Here
the rules oracle is python-chess and the engine is ONLY the opponent. That is not a shortcut: it
means an engine bug can lose a game but cannot produce an illegal position, and it gives legal-move
highlighting, SAN, threefold, insufficient material and the fifty-move rule for free -- all of which
would otherwise be code that could disagree with the engine.

ASSETS. `pieces.js` is the cburnett set from the 4PC GUI, copied here with his agreement ("its just
pieces thats fine") -- this repo is public, so that was his call to make and not mine. Only the piece
SVGs and the board palette are shared; none of the 4PC game logic is.

CORES. Engine processes are pinned to 6-11 (the Existence half) and niced. 12-15 are his desktop's
and are never touched, and the production trainer on the same cores is niced lower so an analysis
burst cannot starve it.
"""
import json
import os
import random
import re
import subprocess
import threading
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import urlparse, parse_qs

import chess
import chess.pgn

ROOT = os.path.dirname(os.path.abspath(__file__))
EXIST = "/home/maswabe/existence"
ENGINE = os.path.join(EXIST, "target/release/engine")
# The CURRENT champion, not the engine's built-in default. `engine/src/main.rs:120` falls back to
# "champion.net", which on 2026-09-11 was a stale 08:56 copy with a different hash from the live
# champion -- playing it would have been playing an old net without saying so.
NET = os.path.join(EXIST, "p1_champion.net")
PORT = int(os.environ.get("PORT", "8800"))
CORES = os.environ.get("EXIST_CORES", "6-11")
# Floor on how long a reply takes, seconds. Not a handicap -- the search is already done.
MIN_THINK = float(os.environ.get("MIN_THINK", "0.75"))


class Engine:
    """One UCI process, serialised by a lock.

    Restarts itself if the process dies: a crashed analysis engine should degrade to "no eval",
    never to a 500 that makes the board look broken.
    """

    def __init__(self, name, nice="10"):
        self.name, self.nice = name, nice
        self.lock = threading.Lock()
        self.p = None
        self._start()

    def _start(self):
        env = dict(os.environ, EXISTENCE_NET=NET)
        self.p = subprocess.Popen(
            ["nice", "-n", self.nice, "taskset", "-c", CORES, ENGINE],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            text=True, bufsize=1, env=env, cwd=EXIST,
        )
        self._cmd("uci", until="uciok")

    def _cmd(self, line, until=None, timeout=60.0):
        self.p.stdin.write(line + "\n")
        self.p.stdin.flush()
        if not until:
            return []
        out, deadline = [], time.time() + timeout
        while time.time() < deadline:
            ln = self.p.stdout.readline()
            if not ln:
                break
            out.append(ln.strip())
            if ln.startswith(until):
                return out
        return out

    def analyse(self, moves, depth):
        """(score_cp_white_pov, bestmove_uci, nodes) for startpos + `moves`.

        Scores come back MOVER-relative, as UCI specifies. They are flipped to white's point of view
        here, once, so every caller and the eval bar agree -- a sign convention applied in two places
        is a sign convention that will eventually disagree with itself.
        """
        with self.lock:
            if self.p.poll() is not None:
                self._start()
            try:
                self._cmd("ucinewgame")
                self._cmd(f"setoption name Depth value {depth}")
                pos = "position startpos" + (" moves " + " ".join(moves) if moves else "")
                self._cmd(pos)
                lines = self._cmd(f"go depth {depth}", until="bestmove", timeout=120.0)
            except (BrokenPipeError, OSError):
                self._start()
                return None, None, 0
        score, best, nodes = None, None, 0
        for ln in lines:
            m = re.search(r"score cp (-?\d+)", ln)
            if m:
                score = int(m.group(1))
            m = re.search(r"\bnodes (\d+)", ln)
            if m:
                nodes = int(m.group(1))
            m = re.search(r"^bestmove (\S+)", ln)
            if m:
                best = m.group(1)
        if score is not None and len(moves) % 2 == 1:   # black to move -> flip to white POV
            score = -score
        return score, best, nodes


PLAY = Engine("play", nice="5")
ANA = Engine("ana", nice="12")


class Game:
    def __init__(self):
        self.reset()

    def reset(self, human_white=True, depth=4):
        self.board = chess.Board()
        self.moves = []          # uci strings from startpos
        self.san = []
        self.human_white = human_white
        self.depth = depth
        self.result = None       # None while playing
        self.evals = {}          # ply -> score (white POV)
        self.version = 0

    def push(self, uci):
        # from_uci RAISES on a malformed square ("e2e9"), it does not return None. Uncaught, that
        # propagates out of the request handler and the client sees a dropped connection rather than
        # a refusal -- measured: curl reported http 000. A bad move must be a 400, not a crash.
        try:
            mv = chess.Move.from_uci(uci)
        except (ValueError, TypeError):
            return False
        if mv not in self.board.legal_moves:
            return False
        self.san.append(self.board.san(mv))
        self.board.push(mv)
        self.moves.append(uci)
        self.version += 1
        self._check_over()
        return True

    def _check_over(self):
        if self.board.is_checkmate():
            self.result = "0-1" if self.board.turn == chess.WHITE else "1-0"
        elif self.board.is_stalemate():
            self.result = "1/2-1/2 (stalemate)"
        elif self.board.is_insufficient_material():
            self.result = "1/2-1/2 (insufficient material)"
        elif self.board.is_fifty_moves():
            self.result = "1/2-1/2 (fifty-move rule)"
        elif self.board.is_repetition(3):
            self.result = "1/2-1/2 (threefold)"

    def engine_turn(self):
        return self.result is None and (self.board.turn == chess.WHITE) != self.human_white

    def pgn(self):
        g = chess.pgn.Game()
        g.headers["Event"] = "Existence loader"
        g.headers["White"] = "Human" if self.human_white else "Existence"
        g.headers["Black"] = "Existence" if self.human_white else "Human"
        g.headers["Result"] = (self.result or "*").split()[0]
        node = g
        b = chess.Board()
        for u in self.moves:
            mv = chess.Move.from_uci(u)
            node = node.add_variation(mv)
            b.push(mv)
        return str(g)


GAME = Game()
GLOCK = threading.Lock()


def state_dict():
    b = GAME.board
    return {
        "fen": b.fen(),
        "turn": "w" if b.turn == chess.WHITE else "b",
        "humanWhite": GAME.human_white,
        "moves": GAME.moves,
        "san": GAME.san,
        "legal": [m.uci() for m in b.legal_moves],
        "lastMove": GAME.moves[-1] if GAME.moves else None,
        "check": b.is_check(),
        "result": GAME.result,
        "engineTurn": GAME.engine_turn(),
        "depth": GAME.depth,
        "eval": GAME.evals.get(len(GAME.moves)),
        "version": GAME.version,
        "ply": len(GAME.moves),
    }


class H(BaseHTTPRequestHandler):
    def log_message(self, *a):
        pass

    def _send(self, code, body, ctype="application/json"):
        data = body.encode() if isinstance(body, str) else body
        self.send_response(code)
        self.send_header("Content-Type", ctype)
        self.send_header("Content-Length", str(len(data)))
        self.send_header("Cache-Control", "no-store")
        self.end_headers()
        self.wfile.write(data)

    def _file(self, name, ctype):
        try:
            with open(os.path.join(ROOT, name), "rb") as f:
                self._send(200, f.read(), ctype)
        except OSError:
            self._send(404, "not found", "text/plain")

    def do_GET(self):
        try:
            self._get()
        except Exception as e:
            self._send(500, json.dumps({"error": type(e).__name__ + ": " + str(e)}))

    def _get(self):
        u = urlparse(self.path)
        q = parse_qs(u.query)
        if u.path in ("/", "/index.html"):
            return self._file("index.html", "text/html; charset=utf-8")
        if u.path == "/pieces.js":
            return self._file("pieces.js", "application/javascript")
        if u.path == "/style.css":
            return self._file("style.css", "text/css")
        if u.path == "/app.js":
            return self._file("app.js", "application/javascript")
        if u.path == "/api/state":
            with GLOCK:
                return self._send(200, json.dumps(state_dict()))
        if u.path == "/api/pgn":
            with GLOCK:
                return self._send(200, GAME.pgn(), "text/plain; charset=utf-8")
        if u.path == "/api/analyse":
            ply = int(q.get("ply", ["0"])[0])
            depth = int(q.get("depth", ["6"])[0])
            with GLOCK:
                mv = list(GAME.moves[:ply])
            sc, best, nodes = ANA.analyse(mv, depth)
            return self._send(200, json.dumps(
                {"ply": ply, "depth": depth, "score": sc, "best": best, "nodes": nodes}))
        if u.path == "/api/review":
            # Eval of EVERY position, for the graph. Depth is deliberately low by default: this is
            # O(plies) engine calls, and a 60-move game at depth 6 would take minutes.
            depth = int(q.get("depth", ["4"])[0])
            with GLOCK:
                allmv = list(GAME.moves)
            out = []
            for i in range(len(allmv) + 1):
                sc, best, _ = ANA.analyse(allmv[:i], depth)
                out.append({"ply": i, "score": sc, "best": best})
                with GLOCK:
                    GAME.evals[i] = sc
            return self._send(200, json.dumps({"depth": depth, "evals": out}))
        return self._send(404, "not found", "text/plain")

    def do_POST(self):
        try:
            self._post()
        except Exception as e:                      # never drop the connection on the board
            self._send(500, json.dumps({"error": type(e).__name__ + ": " + str(e)}))

    def _post(self):
        u = urlparse(self.path)
        n = int(self.headers.get("Content-Length") or 0)
        body = self.rfile.read(n).decode() if n else ""
        if u.path == "/api/new":
            try:
                cfg = json.loads(body) if body else {}
            except ValueError:
                cfg = {}
            with GLOCK:
                GAME.reset(human_white=bool(cfg.get("white", True)),
                           depth=int(cfg.get("depth", 4)))
                need = GAME.engine_turn()
            if need:
                self._engine_move()
            with GLOCK:
                return self._send(200, json.dumps(state_dict()))
        if u.path == "/api/move":
            uci = body.strip()
            with GLOCK:
                ok = GAME.push(uci)
                need = ok and GAME.engine_turn()
            if not ok:
                with GLOCK:
                    return self._send(400, json.dumps({"error": "illegal", **state_dict()}))
            if need:
                self._engine_move()
            with GLOCK:
                return self._send(200, json.dumps(state_dict()))
        if u.path == "/api/resign":
            with GLOCK:
                GAME.result = "0-1 (resigned)" if GAME.human_white else "1-0 (resigned)"
                return self._send(200, json.dumps(state_dict()))
        if u.path == "/api/import":
            import io
            try:
                g = chess.pgn.read_game(io.StringIO(body))
                if g is None:
                    raise ValueError("no game")
                with GLOCK:
                    GAME.reset(human_white=True, depth=GAME.depth)
                    for mv in g.mainline_moves():
                        GAME.push(mv.uci())
                    GAME.result = GAME.result or "*"
                    return self._send(200, json.dumps(state_dict()))
            except Exception as e:
                return self._send(400, json.dumps({"error": str(e)}))
        return self._send(404, "not found", "text/plain")

    def _engine_move(self):
        # MINIMUM THINK TIME. At depth 4 the engine answers in well under a second, so the reply
        # landed the instant the move was released -- it reads as a canned response rather than a
        # search, and it gives the board no time to finish animating the human's move (the glide is
        # 150ms). Waiting out the remainder costs nothing: the search has already happened.
        t0 = time.time()
        with GLOCK:
            mv = list(GAME.moves)
            depth = GAME.depth
        sc, best, _ = PLAY.analyse(mv, depth)
        # Scale a little with depth so a deeper setting feels like it is working harder, and add a
        # small jitter so the cadence is not metronomic.
        want = MIN_THINK + 0.12 * max(0, depth - 3) + random.uniform(0, 0.25)
        left = want - (time.time() - t0)
        if left > 0:
            time.sleep(left)
        with GLOCK:
            if best and best != "(none)":
                GAME.push(best)
            GAME.evals[len(GAME.moves)] = sc


if __name__ == "__main__":
    print(f"Existence loader on http://127.0.0.1:{PORT}  net={os.path.basename(NET)} cores={CORES}")
    ThreadingHTTPServer(("0.0.0.0", PORT), H).serve_forever()
