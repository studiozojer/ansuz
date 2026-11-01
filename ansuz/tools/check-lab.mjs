import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {chromium} from '../../tooling/exact2/node_modules/playwright-core/index.mjs';
const out=resolve(import.meta.dir,'../artifacts/lab');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({headless:true,executablePath:chromium.executablePath(),args:['--enable-unsafe-webgpu','--no-sandbox','--use-mock-keychain','--password-store=basic']});
try {
 const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true,permissions:['clipboard-read','clipboard-write']});
 const page=await context.newPage();const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const el=id=>page.getByTestId(id);const ready=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-gpu-input]');return e&&exact.gpu.wantsInput(Number(e.dataset.view))},null,{timeout:60000});
 const state=()=>page.evaluate(()=>{const e=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(e.dataset.view),{op:'state',resources:true}).world});
 const frames=async(n=2)=>page.evaluate(n=>new Promise(resolve=>{const next=()=>--n<=0?resolve():requestAnimationFrame(next);requestAnimationFrame(next)}),n);
 const near=(a,b)=>a.every((v,i)=>Math.abs(v-b[i])<.00001);
 await page.goto(process.argv[2]??'http://127.0.0.1:8768');await ready();await el('gesture-lab').tap();await ready();await el('lab-record').waitFor();
 await page.waitForFunction(()=>!document.querySelector('[data-testid="lab-record"]')?.disabled);
 await el('lab-note').fill('Catch immediately; a tiny drag then a still finger should stay still.');
 await el('lab-record').tap();await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='RECORDING');
 const b=await el('world').boundingBox();const start={x:b.x+b.width*.4,y:b.y+b.height*.5};
 const cdp=await context.newCDPSession(page);const touch=(type,p)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:p?[{...p,id:1,radiusX:2,radiusY:2,force:1}]:[]});
 await cdp.send('Input.synthesizeScrollGesture',{...start,xDistance:-70,yDistance:0,speed:1000,gestureSourceType:'touch',preventFling:false});await frames(2);
 await touch('touchStart',start);await frames(2);const caught=await state();await frames(8);
 assert(near(caught.resources.Navigation.orientation,(await state()).resources.Navigation.orientation));
 await touch('touchMove',{x:start.x+2,y:start.y});await frames(2);const tiny=await state();await frames(10);
 assert(near(tiny.resources.Navigation.orientation,(await state()).resources.Navigation.orientation));await touch('touchEnd');
 await page.waitForFunction(()=>Number(document.querySelector('[data-testid="lab-duration"]')?.textContent.replace('s',''))>=9,null,{timeout:20000});
 await el('lab-stop').tap();await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='TAKE READY');await frames(3);
 const stopped=await state();const lab=stopped.resources.Lab;assert(lab.frames.length>1000);assert(lab.observed.length>4);
 assert(lab.observed.some(e=>e.phase===0)&&lab.observed.some(e=>e.phase===2));
 const expected=lab.frames.at(-1);await page.screenshot({path:resolve(out,'recorded-2d.png')});
 for(const target of ['lab-flat','lab-3d']) {
  await el(target).tap();await el('lab-replay').tap();
  await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='REPLAYING');
  await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='TAKE READY',null,{timeout:20000});
  const replay=await state();assert(near(replay.resources.Navigation.orientation,expected.orientation));assert(near(replay.resources.Lab.position,expected.position));
 }
 await page.screenshot({path:resolve(out,'replay-3d.png')});
 await el('lab-library').tap();await page.getByText(/Take 1 ·/).waitFor();await page.getByRole('button',{name:'Copy JSON',exact:true}).tap();
 const text=await page.evaluate(()=>navigator.clipboard.readText());const exported=JSON.parse(text);assert(text.length>65536,'Exercise loading beyond the engine message limit');assert(exported.expected.includes('tiny drag'));assert.equal(exported.frames.length,lab.frames.length);
 writeFileSync(resolve(out,'browser-take.json'),text);
 await page.getByRole('button',{name:'Load',exact:true}).tap();await frames(5);assert.equal((await state()).resources.Lab.frames.length,lab.frames.length);
 await page.reload();await ready();await el('gesture-lab').tap();await ready();await el('lab-library').tap();await page.getByText(/Take 1 ·/).waitFor();
 await page.getByRole('button',{name:'Load',exact:true}).tap();await frames(5);assert.equal((await state()).resources.Lab.frames.length,lab.frames.length);
 await page.setViewportSize({width:320,height:640});await frames(5);for(const id of ['lab-record','lab-replay','lab-reset','lab-flat','lab-3d','lab-back','lab-library']) {const r=await el(id).boundingBox();assert(r&&r.x>=0&&r.x+r.width<=320.5&&r.height>=44,id)}
 await page.screenshot({path:resolve(out,'small-phone.png')});assert.deepEqual(errors,[]);
 console.log('PASS gesture lab: physical browser input, observer + response tracks, 2D/3D replay, JSON export, large take reload, persistence, narrow controls');
}finally{await browser.close()}
