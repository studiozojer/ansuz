// Actual browser touch/mouse input; world reads observe, never steer, the test.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {resolve} from 'node:path';
import {chromium} from '../../tooling/exact2/node_modules/playwright-core/index.mjs';
const out=resolve(import.meta.dir,'../artifacts/orbit');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({headless:true,executablePath:chromium.executablePath(),args:['--enable-unsafe-webgpu','--no-sandbox','--use-mock-keychain','--password-store=basic']});
try {
 const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
 const el=id=>page.getByTestId(id);
 const ready=()=>page.waitForFunction(()=>{const el=document.querySelector('[data-gpu-input]');return el&&globalThis.exact?.gpu?.wantsInput(Number(el.dataset.view));},null,{timeout:60000});
 const state=()=>page.evaluate(()=>{const el=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(el.dataset.view),{op:'state',resources:true}).world});
 const frames=async(n=2)=>page.evaluate(n=>new Promise(resolve=>{const next=()=>--n<=0?resolve():requestAnimationFrame(next);requestAnimationFrame(next);}),n);
 const rest=()=>page.waitForFunction(()=>document.querySelector('[data-testid="orbit-motion"]')?.textContent==='AT REST');
 const same=(a,b)=>a.every((v,i)=>Math.abs(v-b[i])<0.000001);
 const cdp=await page.context().newCDPSession(page);
 const touch=(type,point)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:point?[{x:point.x,y:point.y,id:1,radiusX:2,radiusY:2,force:1}]:[]});
 await page.goto(process.argv[2]??'http://127.0.0.1:8768');await ready();await rest();
 const bounds=await el('world').boundingBox();
 const start={x:bounds.x+bounds.width*.4,y:bounds.y+bounds.height*.58};
 const drag=()=>cdp.send('Input.synthesizeScrollGesture',{x:start.x,y:start.y,xDistance:-96,yDistance:56,speed:700,gestureSourceType:'touch',preventFling:false});
 const initial=await state();assert.equal(initial.entities,19);assert.equal(initial.assets.length,2);
 assert.equal(initial.quality.scale,1);assert.equal(initial.published.altitude,1.4);
 await page.screenshot({path:resolve(out,'phone-spin.png')});
 for(const [id,mode] of [['spin-mode',0],['fly-mode',1]]) {
  await el(id).tap();await el('orbit-reset').tap();await rest();await frames(3);
  const beforeFlick=await state();
  // Let Chromium deliver a timed touch swipe. Awaiting each CDP contact
  // separately adds transport pauses that can turn a flick into a held drag.
  await cdp.send('Input.synthesizeScrollGesture',{x:start.x,y:start.y,xDistance:-75,yDistance:40,speed:2400,gestureSourceType:'touch',preventFling:false});
  await frames(3);
  const flicked=await state();
  assert(!same(flicked.resources.Navigation.orientation,beforeFlick.resources.Navigation.orientation),'Quick flick was lost');
  assert.equal(flicked.published.motion,'DRIFTING','Quick flick lost release velocity');
  await touch('touchStart',start);await touch('touchEnd');await frames(2);
  const tapped=await state();await frames(8);
  assert(same(tapped.resources.Navigation.orientation,(await state()).resources.Navigation.orientation),'Quick tap did not stop drift');
  await el('orbit-reset').tap();await rest();await frames(3);
  const before=await state();const reticle=await el('reticle').boundingBox();
  await drag();await frames(2);const moving=await state();
  assert.equal(moving.args.mode,mode);assert(!same(moving.resources.Navigation.orientation,before.resources.Navigation.orientation),'Touch did not revolve');
  assert.notEqual(moving.published.cell,before.published.cell,'Reticle did not encounter a new cell');
  assert.equal(moving.published.motion,'DRIFTING');
  await touch('touchStart',start);await frames(2);const caught=await state();await frames(14);const held=await state();
  assert(same(caught.resources.Navigation.orientation,held.resources.Navigation.orientation),'Stationary finger failed to catch drift');
  assert.equal(held.published.motion,'IN HAND');assert.equal(held.published.altitude,1.4);
  assert.deepEqual(await el('reticle').boundingBox(),reticle,'Reticle moved');
  await touch('touchMove',{x:start.x-30,y:start.y+25});await frames(2);const resumed=await state();
  assert(!same(resumed.resources.Navigation.orientation,held.resources.Navigation.orientation),'Caught sphere did not follow');
  await touch('touchCancel');await frames(2);const cancelled=await state();await frames(10);
  assert(same(cancelled.resources.Navigation.orientation,(await state()).resources.Navigation.orientation),'Cancelled touch kept drifting');
  await page.screenshot({path:resolve(out,mode?'phone-fly.png':'phone-caught.png')});
  console.log(`PASS ${id}: quick flick, quick-tap catch, touch revolve, changing cell, stationary catch, drag again, cancel, fixed reticle/altitude`);
 }
 const cell=(await state()).published.cell;
 await el('spin-mode').tap();await frames(3);assert.equal((await state()).published.cell,cell,'Mode switch changed the target');
 await page.emulateMedia({reducedMotion:'reduce'});await frames(3);await drag();await frames(3);const reduced=await state();await frames(12);
 assert(same(reduced.resources.Navigation.orientation,(await state()).resources.Navigation.orientation),'Reduced motion drifted');
 assert.match(await el('orbit-help').innerText(),/reduced motion/);
 await el('orbit-reset').tap();await rest();await frames(3);
 assert(same((await state()).resources.Navigation.orientation,initial.resources.Navigation.orientation),'Reset did not restore start');
 const beforeButton=(await state()).resources.Navigation.orientation;await el('orbit-east').tap();await frames(3);
 assert(!same((await state()).resources.Navigation.orientation,beforeButton),'Directional button did not move');
 console.log('PASS seamless mode switch, reduced motion, reset, directional button');
 await page.setViewportSize({width:320,height:640});await frames(4);
 for(const id of ['touch-study','orbit-reset','spin-mode','fly-mode','orbit-west','orbit-north','orbit-south','orbit-east']) {
  const b=await el(id).boundingBox();assert(b&&b.x>=0&&b.x+b.width<=320.5,`${id} outside narrow screen`);assert(b.height>=44,`${id} too short`);
 }
 await page.screenshot({path:resolve(out,'small-phone.png')});
 await el('touch-study').tap();await el('handle').waitFor();await el('handle').tap();assert.match(await el('selection').innerText(),/^Selected/);
 await el('orbit-study').tap();await ready();await frames(3);
 await page.setViewportSize({width:1100,height:760});await frames(4);
 const desktop=await el('world').boundingBox();const x=desktop.x+desktop.width*.5,y=desktop.y+desktop.height*.55;
 const beforeMouse=(await state()).resources.Navigation.orientation;
 await page.mouse.move(x,y);await page.mouse.down();await page.mouse.move(x+140,y-55,{steps:12});await page.mouse.up();await frames(3);
 assert(!same(beforeMouse,(await state()).resources.Navigation.orientation),'Mouse drag did not revolve');
 await page.screenshot({path:resolve(out,'desktop.png')});
 assert.deepEqual(errors,[]);
 console.log('PASS narrow controls, retained 2D playground, GPU remount, desktop mouse, no runtime errors');
} finally {await browser.close();}
