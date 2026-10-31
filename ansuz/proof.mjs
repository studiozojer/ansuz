#!/usr/bin/env bun
// Eight operations, each answers a different question:
// tree: find UI controls and named world entities.
// screenshot: inspect pixels, or capture a complete world save.
// tap: press a visible control through real hit testing.
// type: hold/release physical keys (world().hold is this operation).
// state: inspect components, pending assets, held input and busy reasons.
// layout: see world/placed boxes, camera visibility and line-of-sight picks.
// logs: read gameplay events and refusals, including reload/carry failures.
// clock: advance deterministic ticks; settle reports what keeps moving.
// The proof ends with the look: the world drawn offscreen (look(), and at each
// session's close), its tells (a near-black or clipped frame, particles that
// cover nothing, a shadowed sun with no caster) failing the look result, and a
// picture to open once before reporting, which the tells cannot replace.
import {resolve} from 'node:path';
import {readFileSync} from 'node:fs';
import { proof, axNames, completedWorldFrame, worldPixels } from "../tooling/exact2/game/proof.mjs";

// How a fresh session reaches the running world: change it with the title screen.
const play = async session => {
  return session.world('world');
};

if (import.meta.main) await proof(import.meta, async ({open, check, out, host, pin, pinSave, look, say}) => {
  if (process.argv.includes('--screenshot-only')) {
    check('screenshot uses web', host === 'web');
    const shot = await open();
    await play(shot);
    await shot.screenshot(resolve(out, 'game.png'));
    await shot.close();
    return;
  }

  // Engine facts: they hold for any game, so keep them as they are.
  const first = await open();
  const world = await play(first);
  pin(0, await world.snapshot());
  await world.run(500);
  // The GPU-less Linux host draws no world; every other host's renderer must report
  // itself ready and its canvas, outside the HUD, must not be one cleared colour.
  let shown;
  if (host !== 'linux') {
    const shot = await first.screenshot(resolve(out, 'game.png'));
    shown = worldPixels(readFileSync(resolve(out, 'game.png')), shot, await first.layout(), await first.tree());
    const frame = completedWorldFrame(shown, (await first.state()).world?.[0]);
    if (frame.fallback) say('NOTE the HUD covers the whole canvas: the frame fact rests on ready and the first-draw report');
    check('the GPU host presents a completed frame of the world', frame.ok, frame);
  }
  await world.save(resolve(out, 'checkpoint.world'));
  const checkpoint = await world.snapshot();
  await world.run(500);
  const continued = await world.snapshot();
  await world.save(resolve(out, 'continued.world'));
  pinSave('continuation', resolve(out, 'continued.world'));
  await first.close();
  const restored = await open({fresh:true, world:resolve(out, 'checkpoint.world')});
  const loaded = await play(restored);
  check('a fresh process restores the save', JSON.stringify(await loaded.snapshot()) === JSON.stringify(checkpoint));
  await loaded.run(500);
  check('a fresh process continues identically', JSON.stringify(await loaded.snapshot()) === JSON.stringify(continued));
  await loaded.save(resolve(out, 'restored.world'));
  check('continuation saves are byte-identical', readFileSync(resolve(out, 'continued.world')).equals(readFileSync(resolve(out, 'restored.world'))));
  await restored.close();

  if (shown) check('landscape has visible geometry', shown.share > 0.1, shown);
  const journey = await open();
  const scene = await play(journey);
  const start = await scene.local_position('camera');
  const records = JSON.stringify((await scene.resources()).Dataset);
  check('authored records are present independently of geometry', records !== undefined);
  await journey.tap('next');
  await scene.run(100);
  check('selection does not move the camera', JSON.stringify(await scene.local_position('camera')) === JSON.stringify(start));
  const firstShape = (await scene.resources()).Holograms?.shapes?.[0];
  await journey.tap('solid-next');
  await scene.run(100);
  const secondShape = (await scene.resources()).Holograms?.shapes?.[0];
  check('the shape control changes the focused hologram', firstShape !== undefined && secondShape !== firstShape, {firstShape, secondShape});
  check('changing shape preserves records and provenance', JSON.stringify((await scene.resources()).Dataset) === records);
  check('changing shape preserves the viewpoint', JSON.stringify(await scene.local_position('camera')) === JSON.stringify(start));
  await journey.tap('descend');
  await scene.run(3000);
  const low = await scene.local_position('camera');
  check('descent reaches the surface', Math.abs(Math.hypot(...low) - 134.0) < 0.01, low);
  await journey.screenshot(resolve(out, 'clearing.png'));
  await journey.tap('ascend');
  await scene.run(3000);
  const returned = await scene.local_position('camera');
  check('ascent restores the viewpoint', returned.every((n, i) => Math.abs(n - start[i]) < 0.001), returned);
  check('navigation preserves records and provenance', JSON.stringify((await scene.resources()).Dataset) === records);
  await journey.resize(390, 844);
  await journey.screenshot(resolve(out, 'portrait.png'));
  await journey.close();
});
