#!/usr/bin/env python3
"""Verify every derived chain against the external RiverATLAS download.
Usage: python3 tools/water/audit_source.py <download-directory>
The input is not redistributed. Requires pyshp; no network is used.
"""
import json,sys,math
from pathlib import Path
import shapefile
ROOT=Path(__file__).resolve().parents[2]
folder=Path(sys.argv[1]);chains=json.loads((folder/'reach-audit.json').read_text())
s=(ROOT/'Water/network.js').read_text();network=json.loads(s[len('window.WATER_DATA='):-2])['rivers']
ids={i for c in chains for i in c};source={};readers=[]
for path in sorted(folder.glob('RiverATLAS_v10_sa*.shp')):
    r=shapefile.Reader(str(path));readers.append(r)
    for rec in r.iterRecords(fields=['HYRIV_ID','NEXT_DOWN','LENGTH_KM','MAIN_RIV']):
        p=rec.as_dict()
        if p['HYRIV_ID'] in ids: source[p['HYRIV_ID']]=(p,r,rec.oid)
assert len(source)==len(ids)==218146
owner={id:i for i,c in enumerate(chains) for id in c}
for i,(c,n) in enumerate(zip(chains,network)):
    assert n[6]==c[0]
    for a,b in zip(c,c[1:]): assert source[a][0]['NEXT_DOWN']==b
    last=source[c[-1]][0];assert n[0]==owner.get(last['NEXT_DOWN'],-1)
    assert (last['NEXT_DOWN']==0)==(n[0]==-1)
    assert n[3]==last['MAIN_RIV']
    assert abs(sum(source[j][0]['LENGTH_KM'] for j in c)-n[2])<=.0500001
    p,r,oid=source[c[0]];a=r.shape(oid).points[0]
    p,r,oid=source[c[-1]];b=r.shape(oid).points[-1]
    assert math.dist(a,n[5][:2])<.001
    assert math.dist(b,n[5][-2:])<.001
print(f'All {len(chains)} derived chains checked against {len(source)} source reaches: exact links and termini, matching basin IDs, lengths within rounding, endpoints within 0.001°.')
