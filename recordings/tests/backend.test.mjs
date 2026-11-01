import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, copyFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { createHash } from 'node:crypto';
import { RecordingClient, intentId } from '../scripts/client.mjs';
const root = new URL('../', import.meta.url).pathname;
async function start(dir) {
 const p=spawn(resolve(root,'node_modules/.bin/snapback4'),['dev','--port','0','--no-watch'],{cwd:dir,stdio:['ignore','pipe','pipe']});
 let out='',err='';p.stderr.on('data',x=>err+=x);
 const url=await new Promise((yes,no)=>{const timer=setTimeout(()=>{p.kill();no(Error(err))},30000);p.once('error',no);p.once('exit',()=>{clearTimeout(timer);no(Error(err))});p.stdout.on('data',x=>{out+=x;const m=out.match(/http:\/\/127\.0\.0\.1:\d+/);if(m){clearTimeout(timer);yes(m[0])}})});
 return {url,async stop(){const done=once(p,'exit');p.kill();await done}};
}
async function account(url,email){const r=await new RecordingClient(url).request('/auth/signup',{email,password:'Ansuz recording proof password 2026'});return {principal:r.session.principal,client:new RecordingClient(url,r.session.token)}}
async function upload(c,text){const r=await fetch(c.origin+'/assets?as=file&name=gesture.json',{method:'POST',headers:{authorization:`Bearer ${c.token}`,'content-type':'application/json'},body:text});const j=await r.json();assert(r.ok,JSON.stringify(j));return j.asset;}
async function download(c,id){return fetch(c.origin+'/assets/'+id,{headers:{authorization:`Bearer ${c.token}`}})}
test('real Snapback: immutable trace bytes, retry, private history, read grant, revocation, restart', {timeout:120000}, async t=>{
 const dir=await mkdtemp(resolve(tmpdir(),'ansuz-recordings-'));await mkdir(resolve(dir,'snapback'));await copyFile(resolve(root,'snapback/schema.q'),resolve(dir,'snapback/schema.q'));
 let server=await start(dir);t.after(async()=>{await server?.stop();await rm(dir,{recursive:true,force:true})});
 const owner=await account(server.url,'owner@ansuz.test'),agent=await account(server.url,'agent@ansuz.test'),other=await account(server.url,'other@ansuz.test');
 const c=owner.client, a=agent.client, o=other.client;
 const original=JSON.stringify({schema:'ansuz.gesture-take',version:3,hz:120,expected:'Hold, then press the earth upward — 土',frames:Array.from({length:2400},(_,i)=>Array.from({length:22},(_,j)=>i*.001+j))});
 assert(original.length>200000);
 const digest=createHash('sha256').update(original).digest('hex');
 const asset=await upload(c,original);
 const args={digest,label:'Earth intention',expected:'Hold, then lift',element:3,captured:1793500000000,frames:2400,model:'ansuz.elements/1',trace:{id:asset.id}};
 const write=intentId();const record=(await c.mutate('archive',args,write)).result;await c.mutate('archive',args,write);
 assert.equal((await c.query('history',{owner:owner.principal})).data.length,1);
 assert.equal((await c.query('findTake',{owner:owner.principal,digest})).data.id,record.id);
 assert.deepEqual((await a.query('history',{owner:owner.principal})).data,[]);
 assert.equal((await download(a,asset.id)).status,404);
 await c.mutate('allowReader',{reader:agent.principal});
 assert.equal((await a.query('history',{owner:owner.principal})).data[0].digest,digest);
 assert.equal(await (await download(a,asset.id)).text(),original);
 await assert.rejects(a.mutate('archive',args)); // A reader cannot steal/place the owner's asset.
 await a.mutate('allowReader',{reader:other.principal}); // Only shares the agent's own empty library.
 assert.deepEqual((await o.query('history',{owner:owner.principal})).data,[]);
 await c.mutate('removeReader',{reader:agent.principal});
 assert.deepEqual((await a.query('history',{owner:owner.principal})).data,[]);
 assert.equal((await download(a,asset.id)).status,404);
 await server.stop();server=await start(dir);for(const client of [c,a,o])client.origin=server.url;
 assert.equal(await (await download(c,asset.id)).text(),original);
 assert.equal((await c.query('history',{owner:owner.principal})).data.length,1);
 // A duplicate content key cannot replace original bytes or metadata.
 const replacement=await upload(c,'{"different":true}');await assert.rejects(c.mutate('archive',{...args,trace:{id:replacement.id},expected:'rewritten'}));
 assert.equal((await c.query('findTake',{owner:owner.principal,digest})).data.expected,args.expected);
});
