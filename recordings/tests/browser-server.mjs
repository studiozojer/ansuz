// Disposable Exact fixture: simulated OAuth evidence, real Snapback storage.
// Production never imports this file or exposes its simulated provider.
import { spawn, execFileSync } from 'node:child_process';
import { createServer } from 'node:http';
import { Readable } from 'node:stream';
import { mkdtemp, mkdir, copyFile, rm, readFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, extname } from 'node:path';
import { fileURLToPath } from 'node:url';
import { AuthStore } from '../auth/store.mjs';
import { createHandler, digest } from '../auth/service.mjs';
import { randomBytes } from 'node:crypto';
const root = fileURLToPath(new URL('../', import.meta.url));
const directory = await mkdtemp(join(tmpdir(), 'ansuz-browser-'));
await mkdir(join(directory, 'snapback'));
await copyFile(root + 'snapback/schema.q', join(directory, 'snapback/schema.q'));
const webRoot = resolve(root, '../ansuz/target/web-dist');
const preview = createServer(async (req, res) => {
  const pathname = new URL(req.url, 'http://127.0.0.1:8768').pathname;
  const file = resolve(webRoot, '.' + (pathname === '/' ? '/index.html' : pathname));
  if (!file.startsWith(webRoot + '/')) { res.writeHead(403); return res.end(); }
  try {
    const types = {'.html':'text/html','.js':'text/javascript','.wasm':'application/wasm','.json':'application/json','.css':'text/css'};
    res.setHeader('Content-Type', pathname === '/.exact/auth/callback' ? 'text/html' : types[extname(file)] ?? 'application/octet-stream');
    res.end(await readFile(file));
  } catch { res.writeHead(404); res.end(); }
});
preview.listen(8768, '127.0.0.1');
const port = Number(process.env.PORT ?? '4435'), upstream = `http://127.0.0.1:${port + 1}`;
const child = spawn(`${root}node_modules/.bin/snapback4`, ['dev', '--port', String(port + 1), '--no-watch'], { cwd: directory, stdio: 'inherit' });
const store = new AuthStore(':memory:');
const callbacks = ['studio.zojer.ansuz:/oauth','http://127.0.0.1:8768/.exact/auth/callback'];
const client = {
  clientMetadata: { scope: 'atproto' }, jwks: { keys: [] },
  authorize: async (handle, { state }) => {
    // Exact's `authorize` driver consumes one redirect. Simulate the verified
    // provider result here; service.test.mjs exercises the actual handoff.
    const flow = store.get('flow', state), code = randomBytes(32).toString('base64url');
    const did = 'did:plc:' + (handle.startsWith('owner') ? 'a' : handle.startsWith('other') ? 'b' : 'c').repeat(24);
    store.set('flow', state, { ...flow, phase: 'complete', did, code: digest(code) }, 60000);
    const back = new URL(flow.callback); back.searchParams.set('state', state); back.searchParams.set('code', code);
    return back;
  },
  callback: async () => { throw new Error('Browser fixture completes authorization in its start route.'); },
  restore: async did => ({ did }),
};
const handler = createHandler({ client, store, publicOrigin: 'https://ansuz.jera.ink', callbacks, snapbackOrigin: upstream, serviceToken: 'fixture-only',
searchAccounts: async query => {
  const q=query.toLowerCase(), known='owner.test'.includes(q), fallback=['broken','bare'].includes(q);
  return {query,available:true,complete:true,items:known||fallback?[{did:'did:plc:'+'a'.repeat(24),handle:'owner.test',displayName:'Test Owner',avatar:q==='bare'?'':`https://cdn.bsky.app/img/avatar/plain/${q==='broken'?'broken':'owner'}.gif`}]:[]};
},
fetcher: async (url, options) => {
  if (options.headers.authorization === 'Bearer fixture-only') {
    const token = execFileSync(`${root}node_modules/.bin/snapback4`, ['token', '--service', 'test-auth'], { cwd: directory, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
    options.headers.authorization = `Bearer ${token}`;
  }
  return fetch(url, options);
} });
const server = createServer(async (req, res) => {
  if (req.url.startsWith('/oauth/') || req.url === '/auth/refresh') {
    // Discovery runs before /oauth/start. Admit only the ephemeral loopback
    // test carrier here; production keeps its explicit callback allowlist.
    if (typeof req.headers.origin === 'string') {
      try {
        const origin = new URL(req.headers.origin);
        if (origin.protocol === 'http:' && ['127.0.0.1', 'localhost'].includes(origin.hostname)) {
          const testCallback = `${origin.origin}/.exact/auth/callback`;
          if (!callbacks.includes(testCallback)) callbacks.push(testCallback);
        }
      } catch { /* The real handler rejects malformed or unlisted origins. */ }
    }
    const callback = new URL(req.url, `http://127.0.0.1:${port}`).searchParams.get('callback');
    if (callback) {
      const parsed = new URL(callback);
      // The web test carrier chooses its own ephemeral loopback port.
      if (parsed.protocol === 'http:' && ['127.0.0.1', 'localhost'].includes(parsed.hostname) && parsed.pathname === '/.exact/auth/callback' && !callbacks.includes(callback)) callbacks.push(callback);
    }
    return handler(req, res);
  }
  try {
    const chunks = []; for await (const chunk of req) chunks.push(chunk);
    const response = await fetch(upstream + req.url, { method: req.method, headers: Object.fromEntries(Object.entries(req.headers).filter(([key]) => !['host', 'connection', 'content-length', 'transfer-encoding', 'expect'].includes(key))), body: ['GET', 'HEAD'].includes(req.method) ? undefined : Buffer.concat(chunks) });
    res.writeHead(response.status, Object.fromEntries([...response.headers].filter(([key]) => !['content-encoding', 'content-length', 'transfer-encoding'].includes(key))));
    if (response.body) Readable.fromWeb(response.body).pipe(res); else res.end();
  } catch (error) { console.error('Fixture proxy:', error.message); res.writeHead(503); res.end(); }
});
for (let attempt = 0; attempt < 100; attempt++) {
  try { if ((await fetch(upstream + '/auth/config')).ok) break; } catch {}
  await new Promise(resolve => setTimeout(resolve, 100));
}
server.listen(port, '127.0.0.1',()=>console.log('ANSUZ_FIXTURE_READY'));
for (const signal of ['SIGTERM', 'SIGINT']) process.on(signal, () => { preview.closeAllConnections(); preview.close(); server.closeAllConnections(); server.close(); child.kill(signal); });
child.on('error', error => { console.error(error.message); process.exitCode = 1; });
child.on('exit', async code => { preview.close(); server.close(); store.close(); await rm(directory, { recursive: true, force: true }); process.exit(code ?? 0); });
