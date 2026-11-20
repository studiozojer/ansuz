// Exercise the shipped web bundle with an empty persistent browser profile.
// Only installation is online. The static server is gone before the first cast.
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { createInterface } from 'node:readline';
import { fileURLToPath } from 'node:url';

const app = fileURLToPath(new URL('../', import.meta.url));
const sdk = process.env.EXACT || join(app, '.exact/checkout');
const { chromium } = createRequire(resolve(sdk, 'package.json'))('playwright-core');
const profile = await mkdtemp(join(tmpdir(), 'ansuz-offline-'));
const host = spawn('python3', ['-u', '-m', 'http.server', '--bind', '127.0.0.1', '--directory', join(app, 'dist'), '0'], { stdio: ['ignore', 'pipe', 'inherit'] });
async function stopHost() {
  if (host.exitCode !== null || host.signalCode !== null) return;
  const exited = once(host, 'exit');
  host.kill();
  await exited;
}
const open = () => chromium.launchPersistentContext(profile, { headless: true, viewport: { width: 390, height: 844 } });
let context;
try {
  const lines = createInterface({ input: host.stdout });
  const [address] = await once(lines, 'line');
  lines.close();
  const port = address.match(/port (\d+)/)?.[1];
  assert.ok(port, address);
  const url = 'http://127.0.0.1:' + port;
  context = await open();
  let page = await context.newPage();
  page.setDefaultTimeout(12000);
  await page.goto(url);
  await page.waitForFunction(() => Boolean(navigator.serviceWorker.controller));
  await stopHost();
  await context.setOffline(true);
  await page.clock.install({ time: new Date('2026-06-12T21:51:40Z') });
  await page.clock.pauseAt(new Date('2026-06-12T21:51:54Z'));
  await page.reload();
  const el = id => page.getByTestId(id);
  await el('question').fill('Where shall we meet?');
  await el('cast').click({ force: true });
  await page.clock.resume();
  assert.match(await el('primary').innerText(), /45/);
  assert.match(await el('resulting').innerText(), /16/);
  await el('disclose-5').click();
  const passage = await el('commentary-5').innerText();
  assert.ok(passage.trim());
  await el('notes').fill('A note kept on this device.');
  await el('save-notes').click();
  await el('notes-saved').waitFor();
  await el('continue').click();
  await el('question').fill('What follows from this?');
  await el('cast').click();
  await page.getByText('What follows from this?', { exact: true }).first().waitFor();
  await el('parent').waitFor(); // The child write and selection have finished.
  await context.close();
  context = await open();
  await context.setOffline(true);
  page = await context.newPage();
  page.setDefaultTimeout(12000);
  await page.goto(url);
  assert.equal(await el('reading-question').innerText(), 'What follows from this?');
  await el('parent').click();
  await el('commentary-5').waitFor();
  assert.equal(await el('reading-question').innerText(), 'Where shall we meet?');
  assert.equal(await el('commentary-5').innerText(), passage);
  assert.equal(await el('notes').inputValue(), 'A note kept on this device.');
  const child = page.getByRole('button', { name: /What follows from this\?/ });
  await child.click();
  await el('parent').waitFor();
  assert.equal(await el('reading-question').innerText(), 'What follows from this?');
  console.log('Offline release passed: first cast, passage, note, continuation and browser restart with the origin stopped.');
} finally {
  await context?.close();
  await stopHost();
  await rm(profile, { recursive: true, force: true });
}
