// Real browser touches; reads observe the simulation, never set recognizer state.
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {chromium} from '../../tooling/exact2/node_modules/playwright-core/index.mjs';
const out=resolve(import.meta.dir,'../artifacts/elements');mkdirSync(out,{recursive:true});
const browser=await chromium.launch({headless:true,executablePath:chromium.executablePath(),args:['--enable-unsafe-webgpu','--no-sandbox','--use-mock-keychain','--password-store=basic']});
try {
 const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const el=id=>page.getByTestId(id);
 const ready=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-gpu-input]');return e&&exact.gpu.wantsInput(Number(e.dataset.view))},null,{timeout:60000});
 const state=()=>page.evaluate(()=>{const e=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(e.dataset.view),{op:'state',resources:true}).world});
 const cdp=await page.context().newCDPSession(page);
 const touch=(type,p)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints:p?[{...p,id:1,radiusX:2,radiusY:2,force:1}]:[]});
 await page.goto(process.argv[2]??'http://127.0.0.1:8768');await ready();await el('gesture-lab').tap();await ready();
 const bounds=await el('world').boundingBox();const cx=bounds.x+bounds.width/2,cy=bounds.y+bounds.height/2,unit=bounds.height/2;
 const p=(x=0,y=0)=>({x:cx+x*unit,y:cy+y*unit});
 const phase=label=>page.waitForFunction(label=>document.querySelector('[data-testid="lab-contact"]')?.textContent===label,label,{timeout:8000});
 await el('element-fire').tap();await el('lab-note').fill('Hold steady, prepare slowly, then accelerate to ignite.');await el('lab-record').tap();
 await touch('touchStart',p());await phase('DRAW SLOWLY · THEN STRIKE');
 await touch('touchMove',p(.03));await page.waitForTimeout(90);await touch('touchMove',p(.33));
 await phase('IGNITED');await touch('touchEnd');
 const fire=await state();assert.equal(fire.resources.Elements.triggers,1);assert(fire.resources.Elements.energy>.5);
 await page.screenshot({path:resolve(out,'fire.png')});await el('lab-stop').tap();
 const take=(await state()).resources.Lab;assert.equal(take.source_element,1);assert(take.frames.some(f=>f.effect_phase===3));
 await el('lab-replay').tap();await phase('IGNITED');
 await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='TAKE READY');
 assert.equal((await state()).resources.Elements.triggers,1);
 console.log('PASS fire: physical hold/preparation/strike, response track and replayed ignition');

 await el('element-earth').tap();await touch('touchStart',p(-.35,.4));
 for(let i=1;i<=30;i++){await touch('touchMove',p(-.35+.25*Math.sin(i*Math.PI/30),.4-.85*i/30));await page.waitForTimeout(12);}
 await phase('TURN · DRIVE DOWN');await page.screenshot({path:resolve(out,'earth-guide.png')});
 for(let i=1;i<=30;i++){await touch('touchMove',p(-.35,.4-.85*(1-i/30)));await page.waitForTimeout(12);}
 await phase('RAISED');await touch('touchEnd');assert.equal((await state()).resources.Elements.triggers,1);await page.screenshot({path:resolve(out,'earth.png')});
 console.log('PASS earth: curved upstroke outside cube, downward return, no fixed holds');

 await el('element-water').tap();await touch('touchStart',p());await touch('touchMove',p(.2));
 for(let i=1;i<=55;i++){const a=i*Math.PI*2/48;await touch('touchMove',p(Math.cos(a)*.2,Math.sin(a)*.2));await page.waitForTimeout(12);}
 const water=await state();assert.equal(water.resources.Elements.triggers,1);assert(water.resources.Elements.energy>.1);
 await page.screenshot({path:resolve(out,'water.png')});await touch('touchEnd');
 console.log('PASS water: sustained winding activates the particle body');

 await el('element-air').tap();await touch('touchStart',p());
 for(let i=1;i<=24;i++){await touch('touchMove',p(Math.sin(i*.35)*.4));await page.waitForTimeout(12);}
 const air=await state();assert(air.resources.Elements.energy>.1);assert(air.resources.Elements.particles.some(v=>Math.hypot(...v.velocity)>.05));
 await page.screenshot({path:resolve(out,'air.png')});await touch('touchEnd');
 await page.setViewportSize({width:320,height:640});
 for(const id of ['element-motion','element-fire','element-water','element-earth','element-air','lab-record','lab-replay','lab-reset']){
  const b=await el(id).boundingBox();assert(b&&b.x>=0&&b.x+b.width<=320.5&&b.height>=44,id);
 }
 await page.screenshot({path:resolve(out,'small-phone.png')});
 assert.deepEqual(errors,[]);writeFileSync(resolve(out,'responses.json'),JSON.stringify({fire:fire.resources.Elements,water:water.resources.Elements,air:air.resources.Elements}));
 console.log('PASS air: continuous fanning, moving field, 320-point controls, no runtime errors');
}finally{await browser.close()}
