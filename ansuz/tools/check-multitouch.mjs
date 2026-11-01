// Real browser contacts -> delivered journal -> local save -> reload -> replay.
import assert from 'node:assert/strict';
import {mkdirSync} from 'node:fs';
import {resolve} from 'node:path';
import {chromium} from '../../tooling/exact2/node_modules/playwright-core/index.mjs';
const browser=await chromium.launch({headless:true,executablePath:chromium.executablePath(),args:['--enable-unsafe-webgpu','--no-sandbox','--use-mock-keychain','--password-store=basic']});
try {
 const page=await browser.newPage({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const el=id=>page.getByTestId(id);
 const ready=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-gpu-input]');return e&&exact.gpu.wantsInput(Number(e.dataset.view))},null,{timeout:60000});
 const lab=()=>page.evaluate(()=>{const e=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(e.dataset.view),{op:'state',resources:true}).world.resources.Lab});
 await page.goto(process.argv[2]??'http://127.0.0.1:8768');await ready();await el('gesture-lab').tap();await ready();
 await el('lab-record').tap();await page.waitForTimeout(100);
 const b=await el('world').boundingBox();const p=(id,x,y)=>({id,x:b.x+b.width*x,y:b.y+b.height*y,radiusX:2,radiusY:2,force:1});
 const cdp=await page.context().newCDPSession(page);const touch=(type,touchPoints)=>cdp.send('Input.dispatchTouchEvent',{type,touchPoints});
 await touch('touchStart',[p(11,.3,.5),p(22,.7,.5)]);await page.waitForTimeout(150);
 assert.equal((await lab()).pointers.active.length,2);
 for(let i=1;i<=15;i++){await touch('touchMove',[p(11,.3-i*.006,.5-i*.005),p(22,.7+i*.006,.5+i*.005)]);await page.waitForTimeout(20);}
 const out=resolve(import.meta.dir,'../artifacts/multitouch');mkdirSync(out,{recursive:true});
 // Keep capture timing independent of screenshot transport latency.
 const second=(await lab()).pointers.active.reduce((a,b)=>a.position[0]>b.position[0]?a:b);
 await touch('touchEnd',[p(11,.21,.425)]);await page.waitForTimeout(100);
 assert.equal((await lab()).pointers.active.length,1,'remaining finger stays down');
 assert.equal((await lab()).pointers.active[0].id,second.id,'the second finger keeps its identity');
 await touch('touchMove',[p(22,.8,.65)]);await page.waitForTimeout(120);
 await touch('touchCancel',[]);await page.waitForTimeout(100);
 assert.equal((await lab()).pointers.active.length,0);
 await el('lab-stop').tap();await page.waitForTimeout(250);
 const captured=await lab();assert.equal(captured.format_version,4);assert(captured.pointers.events.some(e=>e.phase===3));
 assert.equal(new Set(captured.pointers.events.filter(e=>e.phase===0).map(e=>e.id)).size,2);
 await el('lab-library').tap();await page.getByText(/Take 1 ·/).waitFor();
 await page.reload();await ready();await el('gesture-lab').tap();await ready();await el('lab-library').tap();await page.getByText(/Take 1 ·/).waitFor();
 await page.getByRole('button',{name:'Load',exact:true}).tap();await page.waitForTimeout(150);
 assert.deepEqual((await lab()).pointers.events,captured.pointers.events,'all event bytes survive persistence');
 await el('lab-replay').tap();await page.waitForFunction(()=>{const e=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(e.dataset.view),{op:'state',resources:true}).world.resources.Lab.pointers.active.length===2});
 await page.screenshot({path:resolve(out,'replay.png')});
 await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='TAKE READY');
 assert.equal((await lab()).pointers.active.length,0);assert.deepEqual(errors,[]);
 console.log('PASS multitouch: simultaneous contacts, independent lift, cancellation, save/reload, all-contact replay');
} finally {await browser.close()}
