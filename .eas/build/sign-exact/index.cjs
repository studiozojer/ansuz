// Dependency-free EAS custom function. Credentials stay on the disposable worker.
const { execFileSync, spawn } = require('node:child_process');
const { mkdtempSync, writeFileSync, rmSync } = require('node:fs');
const { tmpdir } = require('node:os');
const { join, resolve } = require('node:path');
const { randomBytes } = require('node:crypto');

function targetCredentials(value) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Missing iOS build credentials. Run eas credentials -p ios.');
  const targets = Object.values(value);
  if (targets.length !== 1) throw new Error('Ansuz expects exactly one signing target.');
  const target = targets[0];
  if (!target?.provisioningProfileBase64 || !target?.distributionCertificate?.dataBase64 || typeof target.distributionCertificate.password !== 'string') {
    throw new Error('Incomplete iOS distribution certificate or provisioning profile.');
  }
  return target;
}
function run(file, args, options = {}) {
  try { return execFileSync(file, args, { encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'], ...options }); }
  catch { throw new Error(`${file} failed while preparing signing. Check the EAS distribution certificate and profile.`); }
}
async function signExact(ctx, { inputs, env = {} }) {
  const buildEnv = { ...process.env, ...env };
  if (buildEnv.EAS_BUILD !== 'true') throw new Error('Signing is only allowed on EAS workers.');
  if (!buildEnv.EXACT_GITHUB_TOKEN) throw new Error('Add preview Secret EXACT_GITHUB_TOKEN with read access to expo/snapback.');
  const credentials = targetCredentials(inputs.credentials.value);
  const root = ctx.workingDirectory;
  const config = require(resolve(root, 'app.json')).expo;
  const scratch = mkdtempSync(join(tmpdir(), 'ansuz-signing-'));
  const keychain = join(scratch, 'signing.keychain-db');
  const password = randomBytes(32).toString('hex');
  const profile = join(scratch, 'app.mobileprovision');
  const certificate = join(scratch, 'distribution.p12');
  let keychainCreated = false;
  let previous = [];
  try {
    writeFileSync(profile, Buffer.from(credentials.provisioningProfileBase64, 'base64'), { mode: 0o600 });
    writeFileSync(certificate, Buffer.from(credentials.distributionCertificate.dataBase64, 'base64'), { mode: 0o600 });
    const plist = run('security', ['cms', '-D', '-i', profile]);
    const decoded = JSON.parse(run('python3', ['-c', 'import sys,plistlib,json,hashlib; p=plistlib.loads(sys.stdin.buffer.read()); print(json.dumps({"app":p["Entitlements"]["application-identifier"],"certificates":[hashlib.sha1(c).hexdigest().upper() for c in p["DeveloperCertificates"]],"devices":len(p.get("ProvisionedDevices",[]))}))'], { input: plist, stdio: ['pipe', 'pipe', 'pipe'] }));
    if (!decoded.app.endsWith('.' + config.ios.bundleIdentifier) || decoded.devices === 0) throw new Error('Expected an ad-hoc Ansuz profile with at least one registered iPhone.');
    previous = [...run('security', ['list-keychains', '-d', 'user']).matchAll(/"([^"]+)"/g)].map(match => match[1]);
    run('security', ['create-keychain', '-p', password, keychain]);
    keychainCreated = true;
    run('security', ['set-keychain-settings', '-lut', '21600', keychain]);
    run('security', ['unlock-keychain', '-p', password, keychain]);
    run('security', ['import', certificate, '-k', keychain, '-P', credentials.distributionCertificate.password, '-T', '/usr/bin/codesign', '-T', '/usr/bin/security']);
    run('security', ['set-key-partition-list', '-S', 'apple-tool:,apple:,codesign:', '-s', '-k', password, keychain]);
    run('security', ['list-keychains', '-d', 'user', '-s', keychain, ...previous]);
    const valid = run('security', ['find-identity', '-v', '-p', 'codesigning', keychain]);
    const identity = decoded.certificates.find(sha => valid.includes(sha));
    if (!identity) throw new Error('The signing certificate does not match the Ansuz provisioning profile.');
    ctx.logger.info('Signing credentials prepared. Building the pinned Exact iPhone app.');
    await new Promise((resolveBuild, reject) => {
      const child = spawn('bash', ['scripts/build-exact-eas.sh', 'device'], {
        cwd: root,
        env: { ...buildEnv, EXACT_IDENTITY: identity, EXACT_PROFILE: profile },
        stdio: ['ignore', 'pipe', 'pipe'],
      });
      child.stdout.on('data', chunk => ctx.logger.info(chunk.toString()));
      child.stderr.on('data', chunk => ctx.logger.info(chunk.toString()));
      child.on('error', () => reject(new Error('Could not launch the Exact build.')));
      child.on('close', code => code === 0 ? resolveBuild() : reject(new Error(`Exact build failed (${code}).`)));
    });
  } finally {
    if (keychainCreated) {
      try { run('security', ['list-keychains', '-d', 'user', '-s', ...previous]); }
      finally { try { run('security', ['delete-keychain', keychain]); } finally { rmSync(scratch, { recursive: true, force: true }); } }
    } else { rmSync(scratch, { recursive: true, force: true }); }
  }
}
exports.default = signExact;
exports.targetCredentials = targetCredentials;
