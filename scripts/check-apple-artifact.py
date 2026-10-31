"""Refuse a wrong-platform, wrong-identity, or unsigned phone artifact before upload."""
import json
import plistlib
import sys
import zipfile
from pathlib import Path

mode = sys.argv[1]
config = json.loads(Path('app.json').read_text())['expo']
if mode == 'device':
    with zipfile.ZipFile('exact-artifacts/Ansuz.ipa') as archive:
        plists = [name for name in archive.namelist() if name.startswith('Payload/') and name.count('/') == 2 and name.endswith('.app/Info.plist')]
        if len(plists) != 1:
            raise SystemExit('Expected one app inside the IPA.')
        prefix = plists[0].removesuffix('Info.plist')
        if prefix + 'embedded.mobileprovision' not in archive.namelist() or prefix + '_CodeSignature/CodeResources' not in archive.namelist():
            raise SystemExit('Phone archive is missing its provisioning profile or signature.')
        info = plistlib.loads(archive.read(plists[0]))
    platform = 'iPhoneOS'
elif mode == 'simulator':
    info = plistlib.loads(Path('exact-artifacts/Ansuz.app/Info.plist').read_bytes())
    platform = 'iPhoneSimulator'
else:
    raise SystemExit('Expected device or simulator.')
expected = {
    'CFBundleIdentifier': config['ios']['bundleIdentifier'],
    'CFBundleShortVersionString': config['version'],
    'CFBundleVersion': config['ios']['buildNumber'],
}
for key, value in expected.items():
    if info.get(key) != value:
        raise SystemExit(f'Unexpected {key}: {info.get(key)!r}; expected {value!r}')
if platform not in info.get('CFBundleSupportedPlatforms', []):
    raise SystemExit(f'Artifact is not built for {platform}.')
print(f'Artifact verified: {expected["CFBundleIdentifier"]}, {platform}, version {config["version"]}, build {config["ios"]["buildNumber"]}.')
