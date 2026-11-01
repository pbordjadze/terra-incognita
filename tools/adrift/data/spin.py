"""Measures whether drifters loop, for the spinning eddies (rules.rs `spin`, SOURCES.md › The
spinning eddies): the bundled drifters' daily velocities less a 15-day running mean, in 10-day
windows, the coherence and rate of their day-to-day turning; then the same statistic on plain
random flights with no spin (the null) and one that spins, sampled the same way.

    python3 tools/adrift/data/spin.py        (from the repository root; reads adrift/data/drifters.bin)
"""
import struct, numpy as np
b=open('adrift/data/drifters.bin','rb').read()
assert b[:4]==b'ADRF'; ver,count=struct.unpack_from('<II',b,4); o=12
R=6371000.0
rots=[];cons=[];spd=[];lats=[];allspd=[]
for _ in range(count):
    i,day0,n,fl=struct.unpack_from('<QfHH',b,o); o+=16
    a=np.frombuffer(b,'<i2',2*n,o).reshape(n,2)/180.0; o+=4*n
    if n<40: continue
    lon,lat=a[:,0],a[:,1]
    dl=(np.diff(lon)+180)%360-180
    u=np.radians(dl)*R*np.cos(np.radians(lat[:-1]))/86400; v=np.radians(np.diff(lat))*R/86400
    w=u+1j*v
    k=15; ker=np.ones(k)/k
    if len(w)<k+12: continue
    m=np.convolve(w,ker,'same'); an=(w-m)[k:-k]
    lt=lat[k:-k-1]
    for s in range(0,len(an)-10,5):
        x=an[s:s+10]; c=np.sum(np.conj(x[:-1])*x[1:]); d=np.sum(np.abs(x[:-1])*np.abs(x[1:]))
        if d==0: continue
        rots.append(np.angle(c)); cons.append(abs(c)/d); spd.append(np.sqrt(np.mean(np.abs(x)**2))); lats.append(lt[s])
rots=np.array(rots);cons=np.array(cons);spd=np.array(spd);lats=np.array(lats)
print('windows',len(rots))
for th in (0.6,0.7,0.8):
    L=(cons>th)&(np.abs(rots)>0.25)
    print(f'consistency>{th}: loopers {L.mean()*100:.1f}%  |spin| median {np.median(np.abs(rots[L])):.2f} rad/day (p25 {np.percentile(np.abs(rots[L]),25):.2f}, p75 {np.percentile(np.abs(rots[L]),75):.2f}); speed loopers {np.median(spd[L]):.3f} vs all {np.median(spd):.3f} m/s; anticyclonic share {np.mean(np.sign(rots[L])==-np.sign(lats[L])):.2f}')

# the null: plain and spinning random flights, quarter-day steps, sampled a day at a time
rng = np.random.default_rng(1)


def window_stats(w):
    k = 15
    an = (w - np.convolve(w, np.ones(k) / k, 'same'))[k:-k]
    r, c = [], []
    for s in range(0, len(an) - 10, 5):
        x = an[s:s + 10]
        cc = np.sum(np.conj(x[:-1]) * x[1:])
        r.append(np.angle(cc))
        c.append(abs(cc) / np.sum(np.abs(x[:-1]) * np.abs(x[1:])))
    return np.array(r), np.array(c)


for T, om in ((3, 0), (1.5, 0), (3, 0.58)):
    dt = 0.25
    a = np.exp(-dt / T)
    e = np.zeros(16000, complex)
    for i in range(1, len(e)):
        e[i] = a * np.exp(1j * om * dt) * e[i - 1] + np.sqrt(1 - a * a) * (rng.normal() + 1j * rng.normal()) / np.sqrt(2)
    r, c = window_stats(e.reshape(-1, 4).mean(1))
    L = (c > 0.7) & (np.abs(r) > 0.25)
    print(f'null T={T} d, spin {om}: loopers {L.mean() * 100:.1f}%, |spin| median {np.median(np.abs(r[L])):.2f} rad/day')
