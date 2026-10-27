#!/usr/bin/env python3
"""A source for one immutable, separately installable experimental build.
Does not read, rewrite, prune, or publish the ordinary SideStore feed.
"""
import json,sys,plistlib,datetime
from pathlib import Path
app,ipa,repo,tag,build=sys.argv[1:]
info=plistlib.loads((Path(app)/'Info.plist').read_bytes())
assert info['CFBundleIdentifier']=='com.pbordjadze.terraincognita.study2'
url=f'https://github.com/{repo}/releases/download/{tag}'
v={'version':info['CFBundleShortVersionString'],'buildVersion':info['CFBundleVersion'],'date':datetime.datetime.now(datetime.timezone.utc).isoformat(),'localizedDescription':'An independent geographic study. Open without a briefing.','downloadURL':url+'/'+Path(ipa).name,'size':Path(ipa).stat().st_size,'minOSVersion':info.get('MinimumOSVersion','26.0')}
entry={'name':'Terra Incognita · Study II','bundleIdentifier':info['CFBundleIdentifier'],'developerName':repo.split('/')[0],'subtitle':'An independent geographic study','localizedDescription':'A separately installable experimental experience. The original atlas and other experiments stay installed. Everything is bundled; nothing leaves the device.','iconURL':url+'/icon.png','tintColor':'#354F7D','category':'education','versions':[v],'version':v['version'],'versionDate':v['date'],'versionDescription':v['localizedDescription'],'downloadURL':v['downloadURL'],'size':v['size'],'appPermissions':{'entitlements':[],'privacy':{}}}
json.dump({'name':'Terra Incognita · Study II','identifier':'com.pbordjadze.terraincognita.study2.source','subtitle':'Independent test build','iconURL':entry['iconURL'],'website':f'https://github.com/{repo}','tintColor':'#354F7D','apps':[entry],'news':[]},sys.stdout,indent=2)
print()
