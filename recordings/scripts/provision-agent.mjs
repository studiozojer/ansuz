import { mkdir, writeFile, readFile } from 'node:fs/promises';
import { dirname } from 'node:path';
import { randomBytes } from 'node:crypto';
import { RecordingClient } from './client.mjs';
const file=process.env.ANSUZ_CREDENTIALS_FILE;
if(!file)throw Error('Set ANSUZ_CREDENTIALS_FILE to a private destination.');
let config;
try{config=JSON.parse(await readFile(file,'utf8'));}
catch(e){if(e.code!=='ENOENT')throw e;}
if(!config){
 config={origin:'https://ansuz.jera.ink',email:`studio-agent-${randomBytes(6).toString('hex')}@ansuz.local`,password:randomBytes(32).toString('base64url')};
 await mkdir(dirname(file),{recursive:true,mode:0o700});
 // Keep recovery credentials before attempting a write with an uncertain network outcome.
 await writeFile(file,JSON.stringify(config,null,2)+'\n',{flag:'wx',mode:0o600});
}
const client=new RecordingClient(config.origin);
let result;
try{result=await client.request('/auth/login',{email:config.email,password:config.password});}
catch{result=await client.request('/auth/signup',{email:config.email,password:config.password});}
if(!result.session?.token)throw Error('No agent session returned');
config.session=result.session;await writeFile(file,JSON.stringify(config,null,2)+'\n',{mode:0o600});
console.log(config.session.principal);
