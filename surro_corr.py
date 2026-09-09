# Every (surrogate, VERIFY) pair on record, deduplicated by (lineage, gen, surrogate).
# VERIFY = 96 pairs on an INDEPENDENT seed, so it is the strength reading, not the gate's 6.
MAIN_SEED, MCTS_SEED = 0.002490, 0.001406
rows=[("MAIN",0.002817,0.430),("MAIN",0.002924,0.422),("MAIN",0.003045,0.430),("MAIN",0.004618,0.258),
      ("MCTS",0.001506,0.505),("MCTS",0.001811,0.492),("MCTS",0.001848,0.490)]
def rank(v):
    s=sorted(range(len(v)),key=lambda i:v[i]); r=[0]*len(v)
    for pos,i in enumerate(s): r[i]=pos+1
    return r
def spearman(x,y):
    rx,ry=rank(x),rank(y); n=len(x)
    mx,my=sum(rx)/n,sum(ry)/n
    num=sum((rx[i]-mx)*(ry[i]-my) for i in range(n))
    dx=sum((rx[i]-mx)**2 for i in range(n))**.5; dy=sum((ry[i]-my)**2 for i in range(n))**.5
    return num/(dx*dy)
print("  lineage  surrogate   gain_vs_seed   VERIFY   verdict")
for lin,s,v in rows:
    seed = MAIN_SEED if lin=="MAIN" else MCTS_SEED
    worse = "RESOLVED WORSE" if v+0.041<0.5 else "not resolved"
    print(f"    {lin:<5} {s:.6f}   {100*(s/seed-1):+6.1f}%      {v:.3f}   {worse}")
gains=[s/(MAIN_SEED if l=='MAIN' else MCTS_SEED) for l,s,_ in rows]
ver=[v for _,_,v in rows]
print(f"\n  Spearman(surrogate gain, VERIFY) pooled = {spearman(gains,ver):+.3f}   (n={len(rows)})")
for lin in ("MAIN","MCTS"):
    g=[s/(MAIN_SEED if lin=='MAIN' else MCTS_SEED) for l,s,_ in rows if l==lin]
    v=[x for l,_,x in rows if l==lin]
    print(f"  Spearman within {lin} = {spearman(g,v):+.3f}   (n={len(g)})")
