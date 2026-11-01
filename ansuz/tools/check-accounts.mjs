// Simulated provider + production auth handler + real disposable Snapback store.
// This proves account boundaries, not real provider authorization.
import assert from 'node:assert/strict';
import {mkdirSync,readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {spawn} from 'node:child_process';
import {chromium} from '../../tooling/exact2/node_modules/playwright-core/index.mjs';
const root=resolve(import.meta.dir,'../..'),out=resolve(import.meta.dir,'../artifacts/accounts');mkdirSync(out,{recursive:true});
const server=spawn('node',['recordings/tests/browser-server.mjs'],{cwd:root,stdio:['ignore','pipe','pipe']});
let logs='';server.stdout.on('data',b=>logs+=b);server.stderr.on('data',b=>logs+=b);
await new Promise((yes,no)=>{const timer=setTimeout(()=>no(Error(logs)),30000);server.stdout.on('data',()=>{if(logs.includes('ANSUZ_FIXTURE_READY')){clearTimeout(timer);yes()}});server.on('exit',()=>{clearTimeout(timer);no(Error(logs))})});
const browser=await chromium.launch({headless:true,executablePath:chromium.executablePath(),args:['--enable-unsafe-webgpu','--no-sandbox','--use-mock-keychain','--password-store=basic']});let page,releaseAsset,releaseSearch;
try{
 const context=await browser.newContext({viewport:{width:390,height:844},isMobile:true,hasTouch:true});
 let blockUploads=false,dropArchiveAck=false,droppedAcks=0;
 let offline=false,holdAsset=false,assetRequested,holdSearch=false,searchRequested;
 let searches=0,authorizations=0;
 context.on('requestfailed',r=>console.log('request failed',new URL(r.url()).pathname,r.failure()?.errorText));
 context.on('page',p=>{p.on('console',m=>{if(m.text().startsWith('auth '))console.log(m.text())});p.on('pageerror',e=>console.log('page error',e.message));});
 // Deterministic image fixture: one transparent pixel; production uses public profiles.
 await context.route('https://cdn.bsky.app/img/avatar/**',r=>r.request().url().includes('broken')?r.fulfill({status:404,body:''}):r.fulfill({contentType:'image/gif',body:Buffer.from('R0lGODlhAQABAIAAAAAAAP///yH5BAEAAAAALAAAAAABAAEAAAIBRAA7','base64')}));
 await context.route('https://ansuz.jera.ink/**',async route=>{
  const request=route.request();const url=new URL(request.url());
  if(url.pathname==='/oauth/accounts'){searches++;assert.equal(request.headers().authorization,undefined);}
  if(url.pathname==='/oauth/start')authorizations++;
  if(offline || (blockUploads && request.method()==='POST' && url.pathname==='/assets'))return route.abort();
  const response=await route.fetch({url:'http://127.0.0.1:4435'+url.pathname+url.search,maxRedirects:0});
  if(dropArchiveAck && url.pathname==='/m/archive' && response.ok()){dropArchiveAck=false;droppedAcks++;return route.abort();}
  // Hold an already-authorized success, not a request the server can reject after logout.
  if(holdAsset&&url.pathname.startsWith('/assets/')){assert.equal(response.status(),200);await new Promise(r=>{releaseAsset=r;assetRequested?.()});}
  if(holdSearch&&url.pathname==='/oauth/accounts'&&request.postDataJSON().query==='ow'){await new Promise(r=>{releaseSearch=r;searchRequested?.()});}
  await route.fulfill({response});
 });
 await context.route('**/.exact/auth/callback?**',r=>r.fulfill({contentType:'text/html',body:readFileSync(resolve(root,'tooling/exact2/host/web/auth-callback.html'),'utf8')}));
 await context.route('**/.exact/auth/callback.js',r=>r.fulfill({contentType:'text/javascript',body:readFileSync(resolve(root,'tooling/exact2/host/web/auth-callback.js'),'utf8')}));
 page=await context.newPage();page.setDefaultTimeout(20000);const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const el=id=>page.getByTestId(id),button=name=>page.getByRole('button',{name,exact:true});
 const ready=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-gpu-input]');return e&&exact.gpu.wantsInput(Number(e.dataset.view))},null,{timeout:60000});
 const state=()=>page.evaluate(()=>{const e=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(e.dataset.view),{op:'state',resources:true}).world});
 const emptyReplay=()=>page.waitForFunction(()=>{const e=document.querySelector('[data-gpu-input]');return exact.gpu.agent(Number(e.dataset.view),{op:'state',resources:true}).world.resources.Lab.frames.length===0},null,{timeout:2000});
 const settled=()=>page.waitForFunction(()=>!document.querySelector('[data-testid="cloud-sync"]')?.disabled);
 const enter=async()=>{await ready();await el('gesture-lab').tap();await ready()};
 const library=async()=>{await el('lab-library').tap();await el('cloud-tab').tap()};
 const login=async name=>{await el('cloud-handle').fill(name+'.test');await el('cloud-login').tap();await el('account-did').waitFor();assert.equal(await el('account-did').innerText(),'did:plc:'+(name==='owner'?'a':'b').repeat(24));await settled();console.log('signed in',name)};
 const record=async note=>{await el('lab-note').fill(note);await el('lab-record').tap();await page.waitForFunction(()=>Number(document.querySelector('[data-testid="lab-duration"]')?.textContent.replace('s',''))>1);await el('lab-stop').tap();await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='TAKE READY');};
 await page.goto('http://127.0.0.1:8768');await enter();await button('Earth').tap();await record('Device-only source: Earth should rise.');await library();
 assert.equal(await page.locator('input[type="password"],input[type="email"]').count(),0);
 assert(await el('cloud-login').isDisabled());
 await el('cloud-handle').fill('o');await page.waitForTimeout(350);assert.equal(searches,0,'short input does not send public search');
 await el('cloud-handle').fill('ow');await el('suggestion-0').waitFor();await page.waitForFunction(()=>{const img=document.querySelector('[data-testid="suggestion-0"] img');return img?.complete&&img.naturalWidth>0});assert(await el('cloud-login').isDisabled());
 await page.screenshot({path:resolve(out,'suggestions.png')});await page.setViewportSize({width:320,height:640});await page.waitForTimeout(300);const popupBox=await el('account-suggestions').boundingBox();assert(popupBox.x>=0&&popupBox.x+popupBox.width<=321&&popupBox.y>=0&&popupBox.y+popupBox.height<=641,'narrow suggestion popup fits viewport');await page.screenshot({path:resolve(out,'suggestions-narrow.png')});await page.setViewportSize({width:390,height:844});
 await el('cloud-handle').press('Escape');assert.equal(await el('account-suggestions').isVisible(),false);
 await el('cloud-handle').fill('owne');await el('suggestion-0').waitFor();await el('cloud-handle').press('ArrowDown');await el('suggestion-0').press('Enter');
 assert.equal(await el('cloud-handle').inputValue(),'owner.test');assert.equal(await el('account-suggestions').isVisible(),false);assert.equal(await el('account-did').count(),0);assert.equal(authorizations,0,'selection only fills the handle');
 await el('cloud-handle').fill('broken');await el('suggestion-0').waitFor();await page.waitForFunction(()=>!document.querySelector('[data-testid="suggestion-0"] img'));assert((await el('suggestion-0').innerText()).includes('T'));
 await el('cloud-handle').fill('bare');await el('suggestion-0').waitFor();assert.equal(await el('suggestion-0').locator('img').count(),0);await el('suggestion-0').tap();assert.equal(await el('cloud-handle').inputValue(),'owner.test');
 console.log('PASS avatars: decoded public image, missing-image initial, failed-image fallback, selection preserved');

 await el('cloud-handle').fill('owner..test');assert(await el('cloud-login').isDisabled());
 await el('cloud-handle').fill('did:plc:'+'a'.repeat(24));assert.equal(await el('account-suggestions').isVisible(),false);assert.equal(await el('cloud-login').isDisabled(),false);
 offline=true;await el('cloud-handle').fill('unknown.test');await el('suggestions-error').waitFor();assert.equal(await el('cloud-login').isDisabled(),false);offline=false;
 holdSearch=true;const searchStarted=new Promise(r=>searchRequested=r);await el('cloud-handle').fill('ow');await Promise.race([searchStarted,new Promise((_,reject)=>setTimeout(()=>reject(Error('search never started')),5000))]);
 await el('cloud-handle').fill('zz');await el('suggestions-empty').waitFor();releaseSearch();holdSearch=false;await page.waitForTimeout(150);assert.equal(await el('suggestion-0').count(),0,'stale search cannot replace current input');
 await el('cloud-handle').fill('own');await el('suggestion-0').waitFor();await el('suggestion-0').tap();assert.equal(await el('cloud-handle').inputValue(),'owner.test');assert.equal(authorizations,0);
 console.log('PASS account suggestions: debounce, tap/keyboard selection, Escape, direct DID, offline fallback, stale-response isolation');

 await login('owner');await el('cloud-sync').tap();await settled();assert.equal(await button('Load for replay').count(),0,'login and Sync must not silently claim guest takes');
 await button('Device').tap();await el('unassigned-takes').tap();await button('Add to this account').tap();await page.getByText(/Take 1 ·/).waitFor();await el('cloud-tab').tap();await el('cloud-sync').tap();await button('Load for replay').waitFor();
 await button('Load for replay').tap();await el('lab-replay').waitFor();await el('lab-replay').tap();await page.waitForFunction(()=>document.querySelector('[data-testid="lab-status"]')?.textContent==='TAKE READY');await library();
 await button('Sign out').tap();await el('cloud-login').waitFor();await emptyReplay();
 await login('other');await el('cloud-sync').tap();await settled();assert.equal(await button('Load for replay').count(),0);
 await button('Device').tap();assert.equal(await page.getByText(/Take 1 ·/).count(),0);assert.equal(await el('unassigned-takes').count(),0,'A-owned takes cannot be reclaimed by B');
 await el('lab-library').tap();blockUploads=true;await record('Private account B Earth take.');await library();
 await page.reload();await enter();await library();await el('account-did').waitFor();await button('Device').tap();await page.getByText(/Take 2 ·/).waitFor();await page.getByText('Waiting to upload',{exact:true}).waitFor();
 await el('cloud-tab').tap();await button('Sign out').tap();await el('cloud-login').waitFor();await login('owner');assert.equal(await page.getByText('Private account B Earth take.').count(),0);
 await button('Sign out').tap();await el('cloud-login').waitFor();await login('other');
 blockUploads=false;dropArchiveAck=true;await page.reload();await enter();await library();
 for(let i=0;i<100 && droppedAcks===0;i++)await page.waitForTimeout(200);assert.equal(droppedAcks,1,'server committed but upload acknowledgement was lost');
 await page.reload();await enter();await library();await button('Load for replay').waitFor();assert.equal(await button('Load for replay').count(),1,'resume reconciles the existing archive without duplicating it');
 await button('Device').tap();await page.getByText('Confirmed in cloud',{exact:true}).waitFor();await el('cloud-tab').tap();
 console.log('PASS upload queue: restart during failed upload, account switch, lost acknowledgement, automatic confirmation without manual Sync');
 await button('Sign out').tap();await el('cloud-login').waitFor();await login('owner');await el('cloud-sync').tap();await button('Load for replay').waitFor();assert.equal(await page.getByText('Private account B Earth take.').count(),0);await settled();
 await page.reload();await enter();await library();await el('account-did').waitFor();assert((await el('account-did').innerText()).endsWith('a'.repeat(24)));await el('cloud-sync').tap();await button('Load for replay').waitFor();
 holdAsset=true;const assetStarted=new Promise(r=>assetRequested=r);await button('Load for replay').tap();await Promise.race([assetStarted,new Promise((_,reject)=>setTimeout(()=>reject(Error('asset download never started')),5000))]);await button('Sign out').tap();await el('cloud-login').waitFor();releaseAsset?.();await page.waitForTimeout(350);assert.equal((await state()).resources.Lab.frames.length,0,'late download cannot reopen signed-out data');holdAsset=false;
 await login('owner');offline=true;await page.reload();await enter();await library();await el('account-did').waitFor();await button('Device').tap();await page.getByText(/Take 1 ·/).waitFor();offline=false;await el('cloud-tab').tap();await el('cloud-sync').tap();await button('Load for replay').waitFor();
 await page.screenshot({path:resolve(out,'account-history.png')});await page.setViewportSize({width:320,height:640});await page.screenshot({path:resolve(out,'narrow-account.png')});assert.deepEqual(errors,[]);
 console.log('PASS account boundaries: OAuth handoff, explicit device-only adoption, DID ownership, A/B separation, signout/relaunch/offline restoration, late-download fence');
}catch(e){console.log('fixture',logs.slice(-1200));for(const c of browser.contexts())for(const p of c.pages())console.log('open page',new URL(p.url()).pathname, (await p.locator('body').innerText()).slice(0,250));if(page){await page.screenshot({path:resolve(out,'failure.png')});console.log((await page.locator('body').innerText()).slice(-2500));}throw e}
finally{releaseSearch?.();releaseAsset?.();await browser.close();server.kill('SIGTERM')}
