import random, math
def e2s(e): return 1.0/(1.0+10.0**(-e/400.0))
def llr(pent, e0, e1):
    n=sum(pent)
    if n<2: return 0.0
    N=float(n)
    mean=sum((i/4.0)*c for i,c in enumerate(pent))/N
    var=sum((i/4.0-mean)**2*c for i,c in enumerate(pent))/(N-1)
    if var<=0.0: return 0.0
    p0,p1=e2s(e0),e2s(e1)
    return (N/(2.0*var))*((mean-p0)**2-(mean-p1)**2)
BOUND=2.944; DRAW=0.806; CAP=400; ZV=30
def run(true_elo,e0,e1,rng):
    s=e2s(true_elo); pw=s-DRAW/2.0; pl=1.0-DRAW-pw
    if pw<0 or pl<0: pw=max(pw,0.0); pl=max(1.0-DRAW-pw,0.0)
    pent=[0]*5
    for p in range(1,CAP+1):
        h=0
        for _ in range(2):
            r=rng.random()
            if r<pw: h+=2
            elif r<pw+DRAW: h+=1
        pent[min(h,4)]+=1
        if p>=ZV and sum(1 for c in pent if c>0)==1: return("ZEROVAR",p)
        L=llr(pent,e0,e1)
        if L>=BOUND: return("ACCEPT",p)
        if L<=-BOUND: return("REJECT",p)
    return("CAP",CAP)
print("  true_elo  bounds     ACCEPT%  REJECT%  CAP%   median_pairs(decided)")
for e0,e1 in [(0,10),(0,30),(0,50),(0,100)]:
    for te in [0,25,50,100]:
        rng=random.Random(1234+te+e1)
        res=[run(te,e0,e1,rng) for _ in range(400)]
        acc=sum(1 for v,_ in res if v=="ACCEPT"); rej=sum(1 for v,_ in res if v=="REJECT")
        cap=sum(1 for v,_ in res if v in("CAP","ZEROVAR"))
        dec=sorted(p for v,p in res if v in("ACCEPT","REJECT"))
        med=dec[len(dec)//2] if dec else -1
        print(f"  {te:>7}   [{e0},{e1}]{'':<4} {100*acc/len(res):6.1f} {100*rej/len(res):8.1f} {100*cap/len(res):6.1f}   {med}")

# ---------------------------------------------------------------------------------------------
# TRANSCRIPTION VERIFIED against crates/pipeline/src/gate.rs, 2026-09-09. Every bounds decision in
# gate_bounds_RESULT.md rests on this file reproducing the Rust exactly, and "I typed it carefully"
# is not a check. Line by line:
#
#   elo_to_score      gate.rs:444   1.0 / (1.0 + 10^(-elo/400))          identical
#   n < 2 -> 0.0      gate.rs:457   guard on too-few pairs                identical
#   var <= 0 -> 0.0   gate.rs:465   zero-variance guard, not a div-by-0   identical
#   LLR_BOUND         gate.rs:441   2.944                                 identical (BOUND)
#   ZERO_VAR_GIVE_UP  gate.rs:220   30 pairs                              identical (ZV)
#   llr formula       gate.rs:472   (n/2var)*((mean-p0)^2-(mean-p1)^2)    identical
#
# 2.944 is ln(0.95/0.05) = 2.944439, i.e. alpha = beta = 0.05 -- which FITNESS 7.2 fixes as human,
# not engine-chosen. So the STOPPING RULE here is spec-compliant; only the BOUNDS were not.
#
# What is NOT verified, and cannot be from this file: the draw rate 0.806 and the per-pair outcome
# model are a MODEL of the arena, not the arena. The simulation bounds pair COUNTS; it does not
# predict wall-clock, and every wall-clock figure quoted from it is marked as derived.
