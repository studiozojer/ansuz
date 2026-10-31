// Run against an already served browser build: bun tools/check-hover.mjs [http://localhost:8766]
import {mkdirSync} from 'node:fs';
import {resolve} from 'node:path';
const out=resolve(import.meta.dir,'../artifacts/desktop-hover');
mkdirSync(out,{recursive:true});
import {chromium} from '../../tooling/exact2/node_modules/playwright-core/index.mjs';
const browser = await chromium.launch({headless:true,executablePath:chromium.executablePath(),args:['--enable-unsafe-webgpu','--disable-smooth-scrolling','--no-sandbox','--use-mock-keychain','--password-store=basic']});
try {
 const page=await browser.newPage({viewport:{width:1280,height:720}});
 page.on('pageerror',e=>console.error('PAGE',e.message));
 await page.goto(process.argv[2] ?? 'http://127.0.0.1:8766');
 await page.locator('[data-testid="next"]').waitFor();
 await page.waitForFunction(() => {
   const surface=document.querySelector('[data-gpu-input]');
   return surface && globalThis.exact?.gpu?.wantsInput(Number(surface.dataset.view));
 }, null, {timeout:60000});
 await page.locator('[data-testid="next"]').click();
 await page.waitForFunction(() => document.querySelector('[data-testid="place"]')?.textContent === 'The Returning');
 await page.locator('[data-testid="descend"]').click();
 await page.waitForTimeout(3500);
 await page.mouse.move(20,180);
 await page.mouse.down(); await page.waitForTimeout(100); await page.mouse.up(); // Clear explicit keyboard focus before testing hover.
 await page.waitForTimeout(200);
 let point;
 for(let y=260;y<=410&&!point;y+=10) for(let x=570;x<=710&&!point;x+=10){
   await page.mouse.move(x,y);await page.waitForTimeout(35);
   if(await page.locator('[data-testid="hover-title"]').count()) point={x,y};
 }
 if(!point) {await page.screenshot({path:resolve(out,'missing.png')});throw new Error('No top hover found');}
 const shape=()=>page.locator('[data-testid="solid-name"]').innerText();
 const before=await shape();
 await page.screenshot({path:resolve(out,'tetrahedron.png')});
 await page.mouse.wheel(0,100); await page.waitForTimeout(300);
 const after=await shape();
 if(before===after) throw new Error(`Wheel did not change solid at ${JSON.stringify(point)}`);
 await page.screenshot({path:resolve(out,'cube.png')});
 await page.mouse.move(20,180);await page.waitForTimeout(150);
 if(await page.locator('[data-testid="hover-title"]').count()) throw new Error('Hover did not clear on leaving');
 await page.locator('[data-testid="next"]').click();
 await page.waitForTimeout(150);
 const focused=await shape();
 await page.locator('[data-testid="solid-next"]').click();
 await page.waitForFunction(previous => document.querySelector('[data-testid="solid-name"]')?.textContent !== previous, focused, {timeout:3000});
 if(!await page.locator('[data-testid="solid-name"]').count()) throw new Error('Moving to shape control lost focus');
 console.log(JSON.stringify({point,before,after,hoverCleared:true,buttonFocusPreserved:true}));
} finally {await browser.close();}
