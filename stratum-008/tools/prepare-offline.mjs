// Package the complete release (including the engine and lens) as one offline
// installation. A failed new installation leaves the previous worker active.
import { createHash } from 'node:crypto';
import { opendir, readFile, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = resolve(process.argv[2] ?? 'dist');
const workerName = 'offline-worker.js';
const registrationName = 'register-offline.js';
const tag = '<script src="./register-offline.js" defer></script>';
for (const page of ['index.html', 'shell.html']) {
  const file = join(root, page);
  const html = await readFile(file, 'utf8');
  if (html.includes(tag)) continue;
  const charset = '<meta charset="utf-8">';
  if (!html.includes(charset)) throw Error('Missing release HTML charset in ' + page);
  await writeFile(file, html.replace(charset, charset + '\n' + tag));
}
await writeFile(join(root, registrationName),
  "if ('serviceWorker' in navigator) navigator.serviceWorker.register('./offline-worker.js').catch(console.error);\n");

async function filesIn(folder = '') {
  const files = [];
  for await (const entry of await opendir(join(root, folder))) {
    if (entry.name.startsWith('.')) continue;
    const path = join(folder, entry.name);
    if (entry.isDirectory()) files.push(...await filesIn(path));
    else if (entry.isFile() && path !== workerName && !path.endsWith('.map')) files.push(path);
  }
  return files.sort();
}

const files = await filesIn();
const source = await readFile(fileURLToPath(new URL('./worker-template.js', import.meta.url)), 'utf8');
const hash = createHash('sha256').update(source);
for (const file of files) hash.update(file).update('\0').update(await readFile(join(root, file)));
const worker = source.replace('__VERSION__', hash.digest('hex')).replace('__FILES__', JSON.stringify(files));
await writeFile(join(root, workerName), worker);
console.log('Packaged ' + files.length + ' assets for offline use.');
