#!/usr/bin/env python3
"""Existence Arena — a DESKTOP loader for 2-player chess, the counterpart of play.py.

WHY THIS EXISTS AND THE WEB PAGE DOES NOT COUNT. The request was a 2PC version of the 4PC LOADER.
The 4PC loader is `~/maswabe/play.py`: a pygame window, 1460x920, resizable, a 410px sidebar, drag
to move, a UCI Engine class and an Analyzer thread. A browser tab is a different kind of object. This
is the same kind of object.

WHAT IT SHARES WITH play.py ON PURPOSE
  * the cburnett piece art, extracted verbatim into existence_pieces.py rather than re-drawn, and
    recolored by swapping the body fill exactly as play.py does;
  * the window shape and sidebar width, so it feels like the same program;
  * one UCI process per role (play, analyse) serialised by a lock, restarted if it dies.

WHAT IS DIFFERENT, AND WHY
  * RULES COME FROM python-chess, not from hand-written movegen. 4PC has no reference implementation
    so play.py must own its own Referee; 2PC has one, and an arena that disagrees with it about
    legality is worse than useless for analysis.
  * The engine is ~/existence/target/release/engine driven by EXISTENCE_NET.

THE BOT DOES NOT INSTANT-MOVE. A reply that arrives in 15ms reads as a bug even when the move is
right, so the engine's answer is held until MIN_THINK has passed. That is presentation, not a
handicap: the search already ran to the requested depth.
"""
import io
import os
import subprocess
import sys
import threading
import time

import chess
import chess.pgn
import pygame

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from existence_pieces import PIECE_SVG, BODY_FILL          # noqa: E402

EXIST = os.path.expanduser("~/existence")
ENGINE = os.environ.get("EXISTENCE_ENGINE", os.path.join(EXIST, "target/release/engine"))
NET = os.environ.get("EXISTENCE_NET", os.path.join(EXIST, "p1_champion.net"))
# Cores 6-11 are Existence's half. 12-15 are HIS and are never taken.
CORES = os.environ.get("MAS_CORES_EXIST", "6-11")
MIN_THINK = float(os.environ.get("MIN_THINK", "0.75"))

LIGHT, DARK = (240, 217, 181), (181, 136, 99)
SEL, LAST, HINT = (255, 230, 120), (205, 210, 106), (20, 85, 30)
BG, PANEL, INK, DIM = (32, 32, 34), (44, 44, 48), (235, 235, 235), (150, 150, 155)
WHITE_FILL, BLACK_FILL = 'fill="#f8f8f8"', 'fill="#2b2b2b"'


class Engine:
    """One UCI process, serialised by a lock, restarted if it dies.

    A crashed analysis engine must degrade to "no eval", never take the board down with it.
    """

    def __init__(self, nice="10"):
        self.nice = nice
        self.lock = threading.Lock()
        self.p = None
        self._start()

    def _start(self):
        env = dict(os.environ, EXISTENCE_NET=NET)
        self.p = subprocess.Popen(
            ["nice", "-n", self.nice, "taskset", "-c", CORES, ENGINE],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL,
            text=True, bufsize=1, env=env, cwd=EXIST)
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
        """(score_white_pov, best_uci, nodes, pv). UCI scores are MOVER-relative; the flip to
        white's point of view happens HERE, once, so the eval bar and the move list cannot
        disagree about a sign."""
        with self.lock:
            if self.p.poll() is not None:
                self._start()
            try:
                self._cmd("ucinewgame")
                pos = "position startpos" + (" moves " + " ".join(moves) if moves else "")
                self._cmd(pos)
                lines = self._cmd(f"go depth {depth}", until="bestmove", timeout=120.0)
            except (BrokenPipeError, OSError):
                try:
                    self._start()
                except Exception:
                    pass
                return None, None, 0, ""
        score = best = None
        nodes, pv = 0, ""
        for ln in lines:
            t = ln.split()
            if ln.startswith("info") and "score" in t:
                try:
                    i = t.index("score")
                    if t[i + 1] == "cp":
                        score = int(t[i + 2])
                    elif t[i + 1] == "mate":
                        m = int(t[i + 2])
                        score = 100000 if m > 0 else -100000
                except (ValueError, IndexError):
                    pass
                if "nodes" in t:
                    try:
                        nodes = int(t[t.index("nodes") + 1])
                    except (ValueError, IndexError):
                        pass
                if "pv" in t:
                    pv = " ".join(t[t.index("pv") + 1:])
            elif ln.startswith("bestmove") and len(t) > 1:
                best = t[1]
        if score is not None and len(moves) % 2 == 1:
            score = -score          # black to move -> flip to white POV
        return score, best, nodes, pv


def piece_surface(symbol, size):
    """Rasterise one cburnett glyph at `size`. pygame-ce loads SVG through SDL_image, so no
    cairosvg dependency and no pre-baked PNGs to drift out of sync with the 4PC loader."""
    svg = PIECE_SVG[symbol.upper()]
    fill = WHITE_FILL if symbol.isupper() else BLACK_FILL
    svg = svg.replace(BODY_FILL, fill)
    if not symbol.isupper():
        svg = svg.replace('stroke="#000"', 'stroke="#f0f0f0"')
    svg = svg.replace("<svg ", f'<svg width="{size}" height="{size}" ', 1)
    return pygame.image.load(io.BytesIO(svg.encode()), "p.svg").convert_alpha()


class Arena:
    SIDEBAR = 410

    def __init__(self):
        pygame.init()
        pygame.display.set_caption("Existence Arena — 2 Player Chess")
        self.W, self.H = 1460, 920
        self.screen = pygame.display.set_mode((self.W, self.H), pygame.RESIZABLE)
        self.clock = pygame.time.Clock()
        self.f = pygame.font.SysFont("dejavusans,segoeui,arial", 15)
        self.fb = pygame.font.SysFont("dejavusans,segoeui,arial", 15, bold=True)
        self.fs = pygame.font.SysFont("dejavusans,segoeui,arial", 12)
        self.fbig = pygame.font.SysFont("dejavusans,segoeui,arial", 20, bold=True)
        self.fmono = pygame.font.SysFont("dejavusansmono,consolas,monospace", 13)

        self.board = chess.Board()
        self.human_white = True
        self.depth = 6
        self.flip = False
        self.sel = None
        self.drag = None          # (square, surface, mouse_xy)
        self.last = None
        self.sprites = {}
        self.status = "Your move."
        self.evals = {}
        self.score = None
        self.pv = ""
        self.nodes = 0
        self.thinking = False
        self.sans = []

        self.play_eng = Engine("10")
        self.ana_eng = Engine("15")
        self.relayout()
        threading.Thread(target=self._ana_loop, daemon=True).start()

    # ---- layout -----------------------------------------------------------------------------
    def relayout(self):
        avail_w = self.W - self.SIDEBAR - 40
        self.sq = max(40, min(avail_w, self.H - 80) // 8)
        self.bx = 20
        self.by = (self.H - self.sq * 8) // 2
        self.sprites = {}

    def sprite(self, sym):
        if sym not in self.sprites:
            self.sprites[sym] = piece_surface(sym, int(self.sq * 0.86))
        return self.sprites[sym]

    def sq_at(self, mx, my):
        c = (mx - self.bx) // self.sq
        r = (my - self.by) // self.sq
        if not (0 <= c < 8 and 0 <= r < 8):
            return None
        if self.flip:
            c, r = 7 - c, 7 - r
        return chess.square(int(c), int(7 - r))

    def sq_rect(self, square):
        c, r = chess.square_file(square), 7 - chess.square_rank(square)
        if self.flip:
            c, r = 7 - c, 7 - r
        return pygame.Rect(self.bx + c * self.sq, self.by + r * self.sq, self.sq, self.sq)

    # ---- engine -----------------------------------------------------------------------------
    def _ana_loop(self):
        """Background analysis. Runs at a LOWER nice than the player engine so a deep analysis
        cannot starve the opponent's reply."""
        last_key = None
        while True:
            key = self.board.board_fen() + str(self.board.turn)
            if key != last_key and not self.thinking:
                mv = [m.uci() for m in self.board.move_stack]
                sc, _, nodes, pv = self.ana_eng.analyse(mv, self.depth)
                if sc is not None:
                    self.score, self.nodes, self.pv = sc, nodes, pv
                    self.evals[len(mv)] = sc
                last_key = key
            time.sleep(0.35)

    def engine_move(self):
        if self.board.is_game_over():
            return
        self.thinking = True
        self.status = "Existence is thinking…"

        def work():
            t0 = time.time()
            mv = [m.uci() for m in self.board.move_stack]
            _, best, _, _ = self.play_eng.analyse(mv, self.depth)
            # Hold the reply so it does not appear instantly. The search already ran.
            dt = MIN_THINK - (time.time() - t0)
            if dt > 0:
                time.sleep(dt)
            if best:
                try:
                    m = chess.Move.from_uci(best)
                    if m in self.board.legal_moves:
                        self.push(m)
                except ValueError:
                    pass
            self.thinking = False
            self.status = self.game_status()

        threading.Thread(target=work, daemon=True).start()

    def push(self, move):
        self.sans.append(self.board.san(move))
        self.board.push(move)
        self.last = (move.from_square, move.to_square)

    def game_status(self):
        if self.board.is_checkmate():
            return "Checkmate — " + ("black wins." if self.board.turn else "white wins.")
        if self.board.is_stalemate():
            return "Stalemate."
        if self.board.is_insufficient_material():
            return "Draw — insufficient material."
        if self.board.can_claim_fifty_moves():
            return "Draw — fifty-move rule."
        if self.board.is_check():
            return "Check!"
        return "Your move." if self.board.turn == self.human_white else "Existence to move."

    def try_move(self, frm, to):
        m = chess.Move(frm, to)
        pc = self.board.piece_at(frm)
        if pc and pc.piece_type == chess.PAWN and chess.square_rank(to) in (0, 7):
            m = chess.Move(frm, to, promotion=chess.QUEEN)
        if m in self.board.legal_moves:
            self.push(m)
            self.status = self.game_status()
            if not self.board.is_game_over():
                self.engine_move()
            return True
        return False

    def new_game(self, human_white=True):
        self.board = chess.Board()
        self.human_white = human_white
        self.sans, self.evals, self.last = [], {}, None
        self.score, self.pv, self.nodes = None, "", 0
        self.flip = not human_white
        self.status = self.game_status()
        if not human_white:
            self.engine_move()

    def undo(self):
        if self.thinking:
            return
        for _ in range(2):
            if self.board.move_stack:
                self.board.pop()
                if self.sans:
                    self.sans.pop()
        self.last = None
        self.status = self.game_status()

    def pgn_text(self):
        g = chess.pgn.Game()
        g.headers["Event"] = "Existence Arena"
        g.headers["White"] = "Human" if self.human_white else "Existence"
        g.headers["Black"] = "Existence" if self.human_white else "Human"
        node = g
        for m in self.board.move_stack:
            node = node.add_variation(m)
        return str(g)

    # ---- drawing ----------------------------------------------------------------------------
    def draw_board(self):
        legal_to = set()
        if self.sel is not None:
            legal_to = {m.to_square for m in self.board.legal_moves if m.from_square == self.sel}
        for sq in chess.SQUARES:
            r = self.sq_rect(sq)
            lightsq = (chess.square_file(sq) + chess.square_rank(sq)) % 2 == 1
            pygame.draw.rect(self.screen, LIGHT if lightsq else DARK, r)
            if self.last and sq in self.last:
                s = pygame.Surface((self.sq, self.sq), pygame.SRCALPHA)
                s.fill((*LAST, 130))
                self.screen.blit(s, r.topleft)
            if sq == self.sel:
                s = pygame.Surface((self.sq, self.sq), pygame.SRCALPHA)
                s.fill((*SEL, 140))
                self.screen.blit(s, r.topleft)
        # legal-move dots, drawn after every square so they are never painted over
        for sq in legal_to:
            r = self.sq_rect(sq)
            s = pygame.Surface((self.sq, self.sq), pygame.SRCALPHA)
            if self.board.piece_at(sq):
                pygame.draw.circle(s, (*HINT, 120), (self.sq // 2, self.sq // 2),
                                   self.sq // 2 - 3, 6)
            else:
                pygame.draw.circle(s, (*HINT, 120), (self.sq // 2, self.sq // 2), self.sq // 6)
            self.screen.blit(s, r.topleft)
        # pieces
        for sq in chess.SQUARES:
            pc = self.board.piece_at(sq)
            if not pc or (self.drag and self.drag[0] == sq):
                continue
            spr = self.sprite(pc.symbol())
            r = self.sq_rect(sq)
            self.screen.blit(spr, spr.get_rect(center=r.center))
        # coordinates
        for i in range(8):
            f = 7 - i if self.flip else i
            t = self.fs.render("abcdefgh"[f], True, (90, 70, 55))
            self.screen.blit(t, (self.bx + i * self.sq + 3, self.by + 8 * self.sq - 15))
            rk = i if self.flip else 7 - i
            t = self.fs.render(str(rk + 1), True, (90, 70, 55))
            self.screen.blit(t, (self.bx + 4, self.by + i * self.sq + 3))
        if self.drag:
            spr, (mx, my) = self.drag[1], self.drag[2]
            self.screen.blit(spr, spr.get_rect(center=(mx, my)))

    def draw_evalbar(self, x, y, h):
        w = 18
        pygame.draw.rect(self.screen, (25, 25, 25), (x, y, w, h))
        sc = self.score if self.score is not None else 0
        frac = 1 / (1 + pow(10, -sc / 400.0))          # logistic, same curve lichess uses
        wh = int(h * frac)
        pygame.draw.rect(self.screen, (238, 238, 238), (x, y + h - wh, w, wh))
        pygame.draw.line(self.screen, (120, 120, 120), (x, y + h // 2), (x + w, y + h // 2))
        lab = "—" if self.score is None else f"{sc/100:+.2f}"
        t = self.fs.render(lab, True, INK)
        self.screen.blit(t, (x - 4, y + h + 6))

    def draw_sidebar(self):
        x = self.bx + self.sq * 8 + 24
        pygame.draw.rect(self.screen, PANEL, (x - 12, 0, self.W - x + 12, self.H))
        self.draw_evalbar(x, 20, self.sq * 8 - 60)
        tx = x + 40
        self.screen.blit(self.fbig.render("Existence Arena", True, INK), (tx, 20))
        self.screen.blit(self.fs.render(
            f"net {os.path.basename(NET)}   cores {CORES}", True, DIM), (tx, 48))
        self.screen.blit(self.f.render(self.status, True, INK), (tx, 74))
        d = f"depth {self.depth}   nodes {self.nodes:,}" if self.nodes else f"depth {self.depth}"
        self.screen.blit(self.fs.render(d, True, DIM), (tx, 98))
        if self.pv:
            self.screen.blit(self.fs.render("pv " + self.pv[:46], True, DIM), (tx, 116))

        # move list, two columns of SAN
        y0 = 146
        self.screen.blit(self.fb.render("Moves", True, INK), (tx, y0))
        y = y0 + 22
        rows = (len(self.sans) + 1) // 2
        start = max(0, rows - 18)
        for i in range(start, rows):
            wm = self.sans[2 * i] if 2 * i < len(self.sans) else ""
            bm = self.sans[2 * i + 1] if 2 * i + 1 < len(self.sans) else ""
            line = f"{i+1:3d}. {wm:<8s} {bm}"
            self.screen.blit(self.fmono.render(line, True, INK), (tx, y))
            y += 17

        # buttons
        self.buttons = []
        # 5 buttons x 30px must clear the footer hint at H-24. Starting at H-150 put the last
        # button at H-30..H-4, straight through the hint text -- visible in the first render.
        by = self.H - 200
        for label, key in (("New (white)", "n"), ("New (black)", "b"), ("Flip", "f"),
                           ("Undo", "u"), ("Copy PGN", "p")):
            r = pygame.Rect(tx, by, 170, 26)
            pygame.draw.rect(self.screen, (62, 62, 68), r, border_radius=5)
            self.screen.blit(self.f.render(label, True, INK), (r.x + 10, r.y + 4))
            self.buttons.append((r, key))
            by += 30
        self.screen.blit(self.fs.render("drag a piece to move  ·  keys: n b f u p", True, DIM),
                         (tx, self.H - 24))

    def do_key(self, k):
        if k == "n":
            self.new_game(True)
        elif k == "b":
            self.new_game(False)
        elif k == "f":
            self.flip = not self.flip
        elif k == "u":
            self.undo()
        elif k == "p":
            try:
                subprocess.run(["wl-copy"], input=self.pgn_text(), text=True, timeout=3)
                self.status = "PGN copied."
            except Exception:
                path = "/tmp/existence_arena.pgn"
                open(path, "w").write(self.pgn_text())
                self.status = f"PGN written to {path}"

    def run(self):
        while True:
            for e in pygame.event.get():
                if e.type == pygame.QUIT:
                    pygame.quit()
                    return
                elif e.type == pygame.VIDEORESIZE:
                    self.W, self.H = e.w, e.h
                    self.screen = pygame.display.set_mode((self.W, self.H), pygame.RESIZABLE)
                    self.relayout()
                elif e.type == pygame.KEYDOWN:
                    if e.key == pygame.K_ESCAPE:
                        pygame.quit()
                        return
                    self.do_key(pygame.key.name(e.key))
                elif e.type == pygame.MOUSEBUTTONDOWN and e.button == 1:
                    for r, k in getattr(self, "buttons", []):
                        if r.collidepoint(e.pos):
                            self.do_key(k)
                            break
                    else:
                        sq = self.sq_at(*e.pos)
                        if sq is not None and not self.thinking:
                            pc = self.board.piece_at(sq)
                            if pc and pc.color == self.board.turn and \
                               self.board.turn == self.human_white:
                                self.sel = sq
                                self.drag = (sq, self.sprite(pc.symbol()), e.pos)
                elif e.type == pygame.MOUSEMOTION and self.drag:
                    self.drag = (self.drag[0], self.drag[1], e.pos)
                elif e.type == pygame.MOUSEBUTTONUP and e.button == 1 and self.drag:
                    to = self.sq_at(*e.pos)
                    frm = self.drag[0]
                    self.drag = None
                    if to is not None and to != frm:
                        self.try_move(frm, to)
                    self.sel = None

            self.screen.fill(BG)
            self.draw_board()
            self.draw_sidebar()
            pygame.display.flip()
            self.clock.tick(60)


if __name__ == "__main__":
    if not os.path.exists(ENGINE):
        sys.exit(f"engine not found: {ENGINE}\nbuild it: cd ~/existence && cargo build --release")
    if not os.path.exists(NET):
        sys.exit(f"net not found: {NET}")
    Arena().run()
