// Run the two clients against an isolated synthetic store with the real schema.
import { spawn, execFileSync } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, mkdir, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

const app = fileURLToPath(new URL('../', import.meta.url));
if (!process.env.JERA_SNAPBACK_DIR) throw Error('Set JERA_SNAPBACK_DIR to the jera-snapback checkout with npm ci installed.');
const backend = resolve(process.env.JERA_SNAPBACK_DIR);
const { JeraClient, intentId } = await import(pathToFileURL(join(backend, 'scripts/client.mjs')));
const cli = join(backend, 'node_modules/.bin/snapback4');
const schema = await readFile(join(backend, 'snapback/schema.q'), 'utf8');
const dir = await mkdtemp(join(tmpdir(), 'ansuz-reading-'));
let database, web;
function stop() { web?.kill(); database?.kill(); }
process.on('SIGINT', stop);
process.on('SIGTERM', stop);
try {
  await mkdir(join(dir, 'snapback'));
  await writeFile(join(dir, 'snapback/schema.q'), 'use personas reader\n' + schema);
  database = spawn(cli, ['dev', '--port', '0', '--no-watch'], { cwd: dir, stdio: ['ignore', 'pipe', 'inherit'] });
  const origin = await new Promise((yes, no) => {
    let buffer = '';
    const timeout = setTimeout(() => no(Error('Snapback readiness timed out')), 20000);
    database.once('error', error => { clearTimeout(timeout); no(error); });
    database.once('exit', code => { clearTimeout(timeout); no(Error('Snapback exited: ' + code)); });
    database.stdout.on('data', chunk => {
      buffer = (buffer + chunk).slice(-4096);
      const match = buffer.match(/http:\/\/127\.0\.0\.1:\d+(?=\s)/);
      if (match) { clearTimeout(timeout); yes(match[0]); }
    });
  });
  const service = execFileSync(cli, ['token', '--service', 'reading-fixture'], { cwd: dir, encoding: 'utf8', stdio: ['ignore','pipe','pipe'] }).trim();
  const { session } = await new JeraClient(origin, service).request('/auth/session', { principal: 'did:plc:aaaaaaaaaaaaaaaaaaaaaaaa', days: 1 });
  const writer = new JeraClient(origin, session.token);
  const intent = intentId();
  const { result } = await writer.mutate('publishPost', { intent, body: 'What is ready to take root?', visibility: 'public', audience: null, parent: null }, intent);
  const original = await writer.readRecord(result.id);
  if (original?.body !== result.body) throw Error('Jera could not read the seeded post.');
  console.log('Jera read shared post ' + result.id + ' from ' + origin);
  web = spawn(join(app, 'exact.mjs'), ['web', ...process.argv.slice(2)], {
    cwd: app, stdio: 'inherit', env: { ...process.env, ANSUZ_READING_ID: result.id, ANSUZ_PREVIEW_ORIGIN: origin }
  });
  const [code, signal] = await once(web, 'exit');
  if (code) process.exitCode = code;
  else if (signal && signal !== 'SIGTERM' && signal !== 'SIGINT') process.exitCode = 1;
} finally {
  if (database && database.exitCode === null && database.signalCode === null) {
    const ended = once(database, 'exit');
    database.kill();
    await ended;
  }
  await rm(dir, { recursive: true, force: true });
}
