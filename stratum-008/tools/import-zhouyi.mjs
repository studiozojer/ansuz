// Read tracked objects at the reference revision without changing the source checkout.
import { execFileSync } from 'node:child_process';
import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const app = fileURLToPath(new URL('../', import.meta.url));
const repo = resolve(process.env.ZHOUYI_DIR || join(app, '../../zhouyi'));
const revision = '01eb63b30ed0ca05d5d5052a1b5e218648abc8c9';
const source = 'zhouyi-exact/';
function git(...args) {
  return execFileSync('git', ['-C', repo, ...args], { maxBuffer: 64 * 1024 * 1024 });
}
function read(path) {
  return git('show', revision + ':' + path);
}
async function save(path, contents) {
  const target = join(app, path);
  await mkdir(dirname(target), { recursive: true });
  await writeFile(target, contents);
}
try {
  git('cat-file', '-e', revision + '^{commit}');
} catch {
  throw new Error('Set ZHOUYI_DIR to a local Zhouyi clone containing ' + revision);
}

const inputs = [
  ['engine/engine-types.ts', source + 'domain/engine-types.ts'],
  ['engine/wilhelm-de.json', source + 'assets/lenses/wilhelm-de.json'],
  ['assets/fonts/PPFraktionMono-Regular.otf', 'assets/fonts/PPFraktionMono-Regular.otf'],
];
for (const path of [
  'assets/fonts/ABCWhyte-Regular.otf', 'assets/fonts/ABCWhyteInktrap-Regular.otf',
  'assets/jianyi/jianyi.js', 'assets/jianyi/jianyi_bg.wasm',
]) inputs.push([path, source + path]);
const framework = source + 'modules/apple/jianyi.xcframework';
for (const path of git('ls-tree', '-r', '--name-only', revision, '--', framework).toString().trim().split('\n')) {
  inputs.push([path.slice(source.length), path]);
}
for (const [target, path] of inputs) await save(target, read(path));

const catalog = JSON.parse(read(source + 'domain/hexagrams.json'));
const figures = catalog.map(({ number, pinyin, english, chinese, binary }) =>
  ({ number, pinyin, english, chinese, binary }));
await save('engine/figures.json', JSON.stringify(figures, null, 2) + '\n');
console.log('Prepared offline reading assets from Zhouyi ' + revision.slice(0, 8));
