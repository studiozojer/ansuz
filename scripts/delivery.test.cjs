const { test } = require('node:test');
const assert = require('node:assert/strict');
const { spawnSync } = require('node:child_process');
const { mkdtempSync, mkdirSync, writeFileSync, readFileSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const signing = require('../.eas/build/sign-exact/index.cjs');
const config = require('../app.json').expo;
const exact = require('../ansuz/app.json').app;

test('EAS and Exact name the same independent app', () => {
  assert.equal(config.ios.bundleIdentifier, exact.id);
  assert.equal(config.name, exact.name);
  assert.equal(config.owner, 'studiozojer');
  assert.equal(config.slug, 'ansuz');
});

test('EAS workflow refuses local execution before using credentials', () => {
  const result = spawnSync('bash', ['scripts/build-exact-eas.sh', 'device'], { encoding:'utf8', env:{ ...process.env, EAS_BUILD:'' } });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /This workflow runs on EAS/);
});

test('signing function refuses local execution and incomplete signing targets', async () => {
  await assert.rejects(signing.default({}, { inputs:{}, env:{ EAS_BUILD:'' } }), /only allowed on EAS/);
  assert.throws(() => signing.targetCredentials({}), /exactly one/);
  assert.throws(() => signing.targetCredentials({ a:{}, b:{} }), /exactly one/);
  assert.throws(() => signing.targetCredentials({ a:{ provisioningProfileBase64:'fake' } }), /Incomplete/);
  await assert.rejects(signing.default({}, { inputs:{}, env:{ EAS_BUILD:'true', EXACT_GITHUB_TOKEN:'' } }), /EXACT_GITHUB_TOKEN/);
});

test('artifact verification rejects wrong app, platform, version and missing phone signing', () => {
  const root = mkdtempSync(join(tmpdir(), 'ansuz-artifact-test-'));
  const checker = resolve('scripts/check-apple-artifact.py');
  writeFileSync(join(root,'app.json'), JSON.stringify({expo:config}));
  const fixture = `import json,plistlib,zipfile,sys\nfrom pathlib import Path\nc=json.loads(Path('app.json').read_text())['expo']\np={'CFBundleIdentifier':c['ios']['bundleIdentifier'],'CFBundleShortVersionString':c['version'],'CFBundleVersion':c['ios']['buildNumber'],'CFBundleSupportedPlatforms':['iPhoneSimulator']}\np.update(json.loads(sys.argv[1]))\nPath('exact-artifacts/Ansuz.app').mkdir(parents=True,exist_ok=True)\nPath('exact-artifacts/Ansuz.app/Info.plist').write_bytes(plistlib.dumps(p))\nwith zipfile.ZipFile('exact-artifacts/Ansuz.ipa','w') as z: z.writestr('Payload/Ansuz.app/Info.plist',plistlib.dumps(p))`;
  const create = fields => assert.equal(spawnSync('python3',['-c',fixture,JSON.stringify(fields)],{cwd:root}).status,0);
  const check = mode => spawnSync('python3',[checker,mode],{cwd:root,encoding:'utf8'});
  try {
    create({});assert.equal(check('simulator').status,0);
    for (const fields of [{CFBundleIdentifier:'studio.zojer.zhouyi'},{CFBundleVersion:'999'},{CFBundleSupportedPlatforms:['iPhoneOS']}]) {
      create(fields);assert.notEqual(check('simulator').status,0);
    }
    create({CFBundleSupportedPlatforms:['iPhoneOS']});assert.match(check('device').stderr,/missing its provisioning profile or signature/);
  } finally { rmSync(root,{recursive:true,force:true}); }
});

test('signing keeps the matching profile through build and cleans up after success or failure', async () => {
  const { createHash } = require('node:crypto');
  for (const exitCode of [0, 9]) {
    const root = mkdtempSync(join(tmpdir(), 'ansuz-signing-test-'));
    const commands = join(root, 'commands');
    const cert = Buffer.from('test certificate');
    const identity = createHash('sha1').update(cert).digest('hex').toUpperCase();
    const profileResult = spawnSync('python3', ['-c', `import plistlib,sys; sys.stdout.buffer.write(plistlib.dumps({'Entitlements':{'application-identifier':'TEAM.${config.ios.bundleIdentifier}'},'ProvisionedDevices':['test-phone'],'DeveloperCertificates':[b'test certificate']}))`]);
    assert.equal(profileResult.status, 0);
    mkdirSync(join(root,'bin'));mkdirSync(join(root,'scripts'));
    writeFileSync(join(root,'app.json'),JSON.stringify({expo:config}));
    writeFileSync(join(root,'bin/security'),`#!${process.execPath}\nconst fs=require('node:fs');const args=process.argv.slice(2);fs.appendFileSync(${JSON.stringify(commands)},args[0]+'\\n');if(args[0]==='cms')process.stdout.write(Buffer.from(${JSON.stringify(profileResult.stdout.toString('base64'))},'base64'));if(args[0]==='list-keychains'&&!args.includes('-s'))console.log('"/tmp/existing-test.keychain"');if(args[0]==='find-identity')console.log(${JSON.stringify(identity)});`,{mode:0o700});
    writeFileSync(join(root,'scripts/build-exact-eas.sh'),`#!/bin/bash\n[[ -f "$EXACT_PROFILE" ]] || exit 3\nprintf '%s\\n%s' "$EXACT_PROFILE" "$EXACT_IDENTITY" > signing-result\nexit ${exitCode}\n`);
    const logs=[];
    const call=()=>signing.default({workingDirectory:root,logger:{info:message=>logs.push(message)}},{
      inputs:{credentials:{value:{Ansuz:{provisioningProfileBase64:profileResult.stdout.toString('base64'),distributionCertificate:{dataBase64:'dGVzdA==',password:'test-password'}}}}},
      env:{EAS_BUILD:'true',EXACT_GITHUB_TOKEN:'test-token',PATH:join(root,'bin')+':'+process.env.PATH},
    });
    // execFileSync uses process.env for the short signing commands; isolate only
    // their executable lookup. No real security/keychain or native build is run.
    const previousPath=process.env.PATH;
    process.env.PATH=join(root,'bin')+':'+previousPath;
    try {
      if(exitCode===0) await call(); else await assert.rejects(call(),/Exact build failed \(9\)/);
      const [profile, actualIdentity]=readFileSync(join(root,'signing-result'),'utf8').split('\n');
      assert.equal(actualIdentity,identity);
      assert.equal(require('node:fs').existsSync(profile),false);
      assert.equal(readFileSync(commands,'utf8').trim().split('\n').at(-1),'delete-keychain');
      assert(!logs.join('\n').includes('test-password'));
      assert(!logs.join('\n').includes('test-token'));
    } finally {process.env.PATH=previousPath;rmSync(root,{recursive:true,force:true});}
  }
});
