#!/usr/bin/env python3
"""Find "reachable but inert" stores: a collection field that is DECLARED and READ but that nothing
ever WRITES.

WHY THIS EXISTS. `Interp::tables_nd` was exactly this shape and it made ladder rung 7 inert while
looking implemented: `TRead` resolves through `tables_nd` first and falls back to the scalar
`tables`, so with `tables_nd` permanently empty every tread index was silently ignored. That cost a
mutation operator being written, enabled, measured, and parked again -- and the audit that would
have caught it beforehand was one command: grep the OTHER side of the store.

LIVES IN THE REPO, not in a session scratchpad. Thirty-six scripts in this project already point at
/tmp/claude-<session-uuid>/ paths that die with the session, which is how a check stops being run.

RESULT 2026-09-10: across 101 .rs files and 25 collection fields, `tables_nd` is the ONLY one. The
class has exactly one member and it is documented and parked (mutate::PARKED_OPS, GRAMMAR 9).

ON SOUNDNESS, because a NEGATIVE result is what this is used for. v1 flagged six fields and three
were false positives -- writes via `&mut self.f` arguments, `extend_from_slice`, and indexed
assignment `self.f[i] = `. False positives are harmless here; the dangerous direction is a false
NEGATIVE, where a field name collides with a local variable, that local's assignment is counted as
a write, and a genuinely inert store is hidden. So every read and write is scoped to a RECEIVER
(`.field`), never to the bare name, and the write idioms are enumerated explicitly.

The four v1 false positives are kept as POSITIVE CONTROLS and printed on every run: if any of them
ever reports zero writes, this scanner has regressed and its silence means nothing.

Usage: python3 inert_scan.py
"""
import re, pathlib

root = pathlib.Path('/home/maswabe/existence/crates')
files = [p for p in root.rglob('*.rs') if 'target' not in p.parts]
src = {str(p): p.read_text(errors='replace') for p in files}
allsrc = "\n".join(src.values())

FIELD = re.compile(r'^\s*(?:pub )?([a-z_][a-z0-9_]*)\s*:\s*(?:Vec|HashMap|BTreeMap|HashSet|BTreeSet)<')
fields = {}
for path, text in src.items():
    for ln in text.splitlines():
        m = FIELD.match(ln)
        if m and not ln.strip().startswith('//'):
            fields.setdefault(m.group(1), set()).add(path)

MUT_METHODS = r'(push|insert|extend|extend_from_slice|append|push_str|resize|clear|truncate|entry|remove|retain|sort|drain|swap|fill|get_mut|iter_mut|last_mut|split_off)'

report = []
for name, decl_files in sorted(fields.items()):
    f = re.escape(name)
    # every access through a receiver, which is how a struct field is actually touched
    accesses = len(re.findall(rf'\.{f}\b', allsrc))
    if accesses < 2:
        continue
    writes = 0
    writes += len(re.findall(rf'\.{f}\s*\.\s*{MUT_METHODS}\s*\(', allsrc))   # self.f.push(..)
    writes += len(re.findall(rf'&mut\s+[a-z_]+\.{f}\b', allsrc))            # &mut self.f
    writes += len(re.findall(rf'\.{f}\s*(\[[^\]]*\])?\s*=\s*[^=]', allsrc)) # self.f = / self.f[i] =
    writes += len(re.findall(rf'&mut\s+[a-z_]+\.{f}\s*\[', allsrc))         # &mut self.f[i]
    # struct-literal construction with a NON-empty initialiser
    for v in re.findall(rf'(?<![.\w]){f}\s*:\s*([^,\n]+)', allsrc):
        v = v.strip()
        if v.startswith(('Vec<','HashMap<','BTreeMap<','HashSet<','BTreeSet<')): continue   # declaration
        if 'new()' in v or v.startswith('vec![]') or 'default()' in v: continue             # empty init
        writes += 1
    report.append((writes, accesses, name, sorted(decl_files)))

inert = [r for r in report if r[0] == 0]
print(f"scanned {len(files)} .rs files; {len(report)} collection fields with >=2 receiver accesses")
print()
print("NEVER WRITTEN (declared, read, but nothing populates it):")
for w, a, name, fs in inert:
    print(f"  {name:<18} {a:>3} accesses, {w} writes   {[p.split('/crates/')[-1] for p in fs]}")
print()
print("POSITIVE CONTROL — these must show writes, they were v1's false positives:")
for w, a, name, fs in report:
    if name in ('slots','feat_a','feat_b','saved'):
        print(f"  {name:<18} {a:>3} accesses, {w:>3} writes  {'OK' if w>0 else '*** STILL FLAGGED ***'}")
