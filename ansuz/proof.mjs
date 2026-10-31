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
  await session.clock('data');
  const world = await session.world('world');
  await session.clock('data');
  return world;
};

if (import.meta.main) await proof(import.meta, async ({open, check, out, host, pin, pinSave, look, say}) => {
  if (process.argv.includes('--screenshot-only')) {
    check('screenshot uses web', host === 'web');
    const shot = await open({storage:'ansuz-surface-proof', fresh:true});
    await play(shot);
    await shot.screenshot(resolve(out, 'game.png'));
    await shot.close();
    return;
  }

  // Engine facts: they hold for any game, so keep them as they are.
  const first = await open({storage:'ansuz-surface-proof', fresh:true});
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
  const restored = await open({storage:'ansuz-surface-proof', fresh:true, world:resolve(out, 'checkpoint.world')});
  const loaded = await play(restored);
  check('a fresh process restores the save', JSON.stringify(await loaded.snapshot()) === JSON.stringify(checkpoint));
  await loaded.run(500);
  check('a fresh process continues identically', JSON.stringify(await loaded.snapshot()) === JSON.stringify(continued));
  await loaded.save(resolve(out, 'restored.world'));
  check('continuation saves are byte-identical', readFileSync(resolve(out, 'continued.world')).equals(readFileSync(resolve(out, 'restored.world'))));
  await restored.close();

  // The empty top-down terrain contrasts mainly through thin grid lines (~6%).
  // Its inspected dark face fill is intentional; the old seeded cubes are gone.
  if (shown) check('empty terrain has visible grid geometry', shown.share > 0.03, shown);
  const journey = await open({storage:'ansuz-surface-proof', fresh:true});
  const scene = await play(journey);
  await scene.run(100);
  const start = await scene.local_position('camera');
  check('camera starts in the atmospheric landscape', Math.abs(start[0]) < 0.001 && Math.abs(start[1]) < 0.001 && Math.abs(start[2]-152) < 0.001,start);
  await journey.tap('descend');
  await scene.run(1000);
  const top = await scene.local_position('camera');
  check('descent opens the top-down subview',Math.abs(top[1]-148)<0.001 && Math.abs(top[2])<0.001,top);
  const records = (await scene.resources()).Dataset;
  check('fresh surface has no seeded objects', records.objects.length === 0);
  await journey.tap('select-center');
  await scene.run(100);
  check('centre selection resolves an actual terrain face', (await scene.resources()).Journey.selected != null);
  check('selection leaves records empty', (await scene.resources()).Dataset.objects.length === 0);
  check('selection leaves camera still', JSON.stringify(await scene.local_position('camera')) === JSON.stringify(top));
  await journey.screenshot(resolve(out, 'surface-selected.png'));
  await journey.tap('add-note');
  await scene.run(100);
  await journey.screenshot(resolve(out, 'surface-editor.png'));
  await journey.tap('cancel');
  await journey.tap('deselect');
  await journey.tap('ascend');
  await scene.run(1000);
  check('ascent returns to the atmospheric viewpoint', JSON.stringify(await scene.local_position('camera')) === JSON.stringify(start));
  await journey.resize(390, 844);
  await journey.screenshot(resolve(out, 'portrait.png'));
  await journey.close();
});
