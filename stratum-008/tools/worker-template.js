const prefix = 'ansuz-008:' + self.registration.scope + ':';
const cacheName = prefix + '__VERSION__';
const assets = __FILES__.map(path => new URL(path, self.registration.scope).href);
const indexUrl = new URL('index.html', self.registration.scope).href;
const scopeUrl = new URL(self.registration.scope);
const installed = new Set(assets);

self.addEventListener('install', event => {
  event.waitUntil(caches.open(cacheName).then(cache =>
    cache.addAll(assets.map(url => new Request(url, { cache: 'reload' })))));
});

self.addEventListener('activate', event => {
  event.waitUntil((async () => {
    for (const name of await caches.keys()) {
      if (name.startsWith(prefix) && name !== cacheName) await caches.delete(name);
    }
    await self.clients.claim();
  })());
});

self.addEventListener('fetch', event => {
  if (event.request.method !== 'GET') return;
  const url = new URL(event.request.url);
  if (url.origin !== scopeUrl.origin) return;
  const key = url.pathname === scopeUrl.pathname ? indexUrl : url.origin + url.pathname;
  if (!installed.has(key)) return;
  event.respondWith(caches.open(cacheName).then(async cache =>
    (await cache.match(key)) ?? fetch(event.request)));
});
