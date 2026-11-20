import { expect, test } from 'bun:test';
import { readFile } from 'node:fs/promises';
import init, { cast as jianyi } from '../assets/jianyi/jianyi.js';
import { cast, describe } from './cast.ts';

test('bundled Jianyi reproduces the Zhouyi reference cast with no network', async () => {
  const wasm = await readFile(new URL('../assets/jianyi/jianyi_bg.wasm', import.meta.url));
  await init({ module_or_path: wasm });
  const native = {
    available: true,
    async later(request: { op: 'cast'; seed: string }) {
      return JSON.parse(jianyi(JSON.stringify({ timestamp: request.seed })));
    },
  };
  const seed = '2026-06-12T21:51:54+00:00';
  const result = await cast(native, seed);
  expect(result.seed).toBe(seed);
  expect(result.primary.number).toBe(45);
  expect(result.resulting?.number).toBe(16);
  expect(result.nuclear.number).toBe(53);
  expect(result.changing_lines).toEqual([5]);
  expect(result.fingerprint).toBe('v2_ich_45.c5');
});

test('every cast has a bundled passage and six readable lines', () => {
  for (let n = 1; n <= 64; n++) {
    const reading = describe(n);
    expect(reading.glyph).toBe(String.fromCodePoint(0x4dc0 + n - 1));
    expect(reading.judgment.length).toBeGreaterThan(0);
    if (n === 2) expect(reading.image).toBe(''); // The source omits this passage.
    else expect(reading.image.length).toBeGreaterThan(0);
    expect(reading.lines.map(l => l.position)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(reading.lines.every(l => l.text.length > 0)).toBe(true);
  }
  const fifth = describe(45).lines[4];
  expect(fifth.text.length).toBeGreaterThan(0);
  expect(fifth.commentary.length).toBeGreaterThan(0);
  expect(describe(45).attribution).toContain('machine-assisted and unreviewed');
  expect(() => describe(65)).toThrow();
});
