import { createServer } from 'node:http';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { NodeOAuthClient, requestLocalLock } from '@atproto/oauth-client-node';
import { JoseKey } from '@atproto/jwk-jose';
import { AuthStore } from './store.mjs';
import { createHandler } from './service.mjs';
import { createAccountSearch } from './accounts.mjs';

process.umask(0o077);
const directory = resolve(process.env.ANSUZ_AUTH_DATA ?? '.local/auth');
await mkdir(directory, { recursive: true, mode: 0o700 });
const publicOrigin = process.env.ANSUZ_PUBLIC_ORIGIN ?? 'https://ansuz.jera.ink';
const callbacks = (process.env.ANSUZ_AUTH_CALLBACKS ?? 'studio.zojer.ansuz:/oauth').split(',');
const keyPath = resolve(directory, 'signing-key.json');
let key;
try { key = await JoseKey.fromJWK(await readFile(keyPath, 'utf8')); }
catch (error) {
  if (error.code !== 'ENOENT') throw error;
  key = await JoseKey.generate(['ES256'], 'ansuz-1');
  await writeFile(keyPath, JSON.stringify(key.privateJwk), { mode: 0o600, flag: 'wx' });
}
const store = new AuthStore(resolve(directory, 'auth.sqlite'));
const client = new NodeOAuthClient({
  clientMetadata: {
    client_id: `${publicOrigin}/oauth/client-metadata.json`, client_name: 'Ansuz',
    redirect_uris: [`${publicOrigin}/oauth/callback`], scope: 'atproto',
    grant_types: ['authorization_code', 'refresh_token'], response_types: ['code'], application_type: 'web',
    token_endpoint_auth_method: 'private_key_jwt', token_endpoint_auth_signing_alg: 'ES256',
    dpop_bound_access_tokens: true, jwks_uri: `${publicOrigin}/oauth/jwks.json`,
  },
  keyset: [key], requestLock: requestLocalLock, stateStore: store.sdk('oauth-state', 10 * 60000), sessionStore: store.sdk('oauth-session'),
});
const serviceToken = (await readFile(process.env.ANSUZ_SERVICE_TOKEN_FILE ?? resolve(directory, 'service-token'), 'utf8')).trim();
if (!serviceToken) throw new Error('Missing Snapback service token.');
const server = createServer(createHandler({ client, store, publicOrigin, callbacks, serviceToken,
  searchAccounts: createAccountSearch({ pdsOrigin: process.env.ANSUZ_PDS_ORIGIN ?? 'https://pds.jera.ink', searchBluesky: true }),
  snapbackOrigin: process.env.ANSUZ_SNAPBACK_ORIGIN ?? 'http://127.0.0.1:4431' }));
server.requestTimeout = 30000; server.headersTimeout = 10000;
server.listen(Number(process.env.PORT ?? 4433), '127.0.0.1', () => console.log('Ansuz ATproto authorization ready on loopback.'));
const timer = setInterval(() => store.prune(), 60000); timer.unref();
for (const signal of ['SIGTERM', 'SIGINT']) process.on(signal, () => server.close(() => { store.close(); process.exit(0); }));
