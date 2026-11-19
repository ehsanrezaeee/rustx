// Build-time placeholders are replaced by the RustX CLI before deployment.
const CACHE = '__RUSTX_CACHE__';
const FILES = __RUSTX_FILES__;

self.addEventListener('install', (event) => {
  event.waitUntil(
    caches
      .open(CACHE)
      .then((cache) => cache.addAll(FILES.map((url) => new Request(url, { cache: 'reload' })))),
  );
});

self.addEventListener('activate', (event) => {
  event.waitUntil(
    caches
      .keys()
      .then((keys) =>
        Promise.all(
          keys
            .filter((key) => key.startsWith('rustx-') && key !== CACHE)
            .map((key) => caches.delete(key)),
        ),
      )
      .then(() => self.clients.claim()),
  );
});

self.addEventListener('fetch', (event) => {
  const url = new URL(event.request.url);
  if (
    event.request.method !== 'GET' ||
    url.origin !== location.origin ||
    url.pathname.startsWith('/api/') ||
    url.pathname.startsWith('/__rustx/')
  ) {
    return;
  }

  const navigation = event.request.mode === 'navigate';
  if (!navigation && !FILES.includes(url.pathname)) return;

  const cached = () =>
    caches.open(CACHE).then((cache) => cache.match(navigation ? '/index.html' : event.request));
  if (!navigation && /\.[a-f0-9]{12}\./.test(url.pathname)) {
    event.respondWith(cached().then((response) => response || fetch(event.request)));
    return;
  }
  event.respondWith(
    fetch(event.request).catch(async (error) => {
      const response = await cached();
      if (response) return response;
      throw error;
    }),
  );
});
