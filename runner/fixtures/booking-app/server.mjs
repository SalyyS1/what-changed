import http from 'node:http';
import { mkdir,readFile,writeFile,rename } from 'node:fs/promises';
import { join,resolve } from 'node:path';
import { randomUUID } from 'node:crypto';
import { fileURLToPath } from 'node:url';

const html=`<!doctype html><html lang="en"><meta charset="utf-8"><meta name="viewport" content="width=device-width"><title>Ownvero synthetic booking preview</title>
<style>body{font:16px system-ui;max-width:42rem;margin:3rem auto;padding:1.5rem;color:#28302c;background:#f7f7f2}label{display:block;margin:1rem 0}input,button{font:inherit;padding:.7rem;border:1px solid #87928a;border-radius:6px}button{cursor:pointer;background:#e4eade}nav{display:flex;gap:1rem}output{display:block;margin:1rem 0;overflow-wrap:anywhere}</style>
<h1>Synthetic booking preview</h1><p>Test data only. No email, payments or real customers.</p><nav><a href="/signin/staff">Staff test session</a><a href="/signin/customer">Customer test session</a></nav>
<p data-testid="session-role">Choose the intended test role in this browser window.</p><form id="booking"><label>Booking note <input name="note" aria-label="Booking note" required maxlength="200"></label><label>Test run tag <input name="run_tag" aria-label="Test run tag" required maxlength="80"></label><button>Save booking</button></form><p role="status" id="status"></p><output data-testid="ownvero-fixture-id"></output><button id="cancel" type="button" hidden>Cancel this test booking</button>
<script type="module">
const form=document.querySelector('form'),output=document.querySelector('output'),status=document.querySelector('#status'),cancel=document.querySelector('#cancel');
form.addEventListener('submit',async event=>{event.preventDefault();const values=Object.fromEntries(new FormData(form));const response=await fetch('/api/bookings',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify(values)});const body=await response.json();if(response.ok){output.textContent=body.id;status.textContent='Booking saved';cancel.hidden=false}else status.textContent='Choose the staff test session first.'});
cancel.addEventListener('click',async()=>{if(!output.textContent)return;const response=await fetch('/api/bookings/'+encodeURIComponent(output.textContent)+'/cancel',{method:'POST',headers:{'Content-Type':'application/json'},body:JSON.stringify({run_tag:form.elements.run_tag.value})});status.textContent=response.ok?'Test booking cancelled':'Cleanup unavailable; inspect this tagged test record manually.'});
fetch('/api/session').then(response=>response.json()).then(body=>document.querySelector('[data-testid="session-role"]').textContent='Isolated test role: '+(body.role??'not signed in'));
</script></html>`;

export async function startBookingFixture({root,appId,environmentId,revision='fixture-v1',port=0,losePersistence=false,leakOtherRole=false}) {
  await mkdir(root,{recursive:true,mode:0o700});
  const database=join(root,'bookings.json');let records=[];
  try{records=JSON.parse(await readFile(database,'utf8'))}catch(error){if(error.code!=='ENOENT')throw error}
  const sessions=new Map();let origin;let writeQueue=Promise.resolve();
  const json=(response,status,body)=>{response.writeHead(status,{'Content-Type':'application/json','Cache-Control':'no-store','X-Content-Type-Options':'nosniff'});response.end(JSON.stringify(body))};
  const save=()=>{writeQueue=writeQueue.then(async()=>{const temporary=join(root,`write-${randomUUID()}.json`);await writeFile(temporary,JSON.stringify(records),{flag:'wx',mode:0o600});await rename(temporary,database)});return writeQueue};
  const bodyOf=async request=>{let raw='';for await(const chunk of request){raw+=chunk;if(Buffer.byteLength(raw)>8192)throw new Error('Too large')}return JSON.parse(raw)};
  const server=http.createServer({maxHeaderSize:8192},async(request,response)=>{
    try{
      const url=new URL(request.url,origin);const session=(request.headers.cookie??'').split(';').map(part=>part.trim()).find(part=>part.startsWith('ownvero_fixture='))?.slice('ownvero_fixture='.length);const role=sessions.get(session);
      if(request.method==='GET' && /^\/signin\/(staff|customer)$/.test(url.pathname)){const id=randomUUID();sessions.set(id,url.pathname.split('/').at(-1));response.writeHead(303,{'Set-Cookie':`ownvero_fixture=${id}; HttpOnly; SameSite=Strict; Path=/`,'Location':'/','Cache-Control':'no-store'});response.end();return}
      if(request.method==='GET'&&url.pathname==='/'){response.writeHead(200,{'Content-Type':'text/html; charset=utf-8','Cache-Control':'no-store'});response.end(html);return}
      if(request.method==='GET'&&url.pathname==='/api/session'){json(response,200,{role:role??null});return}
      if(request.method==='GET'&&url.pathname==='/.well-known/ownvero.json'){json(response,200,{schemaVersion:1,appId,environmentId,revision,environmentType:'local'});return}
      if(request.method==='POST'&&url.pathname==='/api/bookings'){
        if(role!=='staff'){json(response,403,{error:'denied'});return}
        if(request.headers.origin!==origin || request.headers['content-type']!=='application/json'){json(response,403,{error:'test origin required'});return}
        const body=await bodyOf(request);
        if(typeof body.note!=='string'||body.note.length>200||!/^ownvero-[A-Za-z0-9-]{1,70}$/.test(body.run_tag??'')||Object.keys(body).some(key=>!['note','run_tag'].includes(key))){json(response,400,{error:'synthetic note and run tag required'});return}
        const record={id:randomUUID(),note:body.note,run_tag:body.run_tag,state:'saved'};
        if(!losePersistence){records.push(record);await save()}
        json(response,201,{id:record.id});return;
      }
      const match=/^\/api\/bookings\/([a-f0-9-]{36})(\/cancel)?$/.exec(url.pathname);
      if(match){
        if(role!=='staff' && !leakOtherRole){json(response,403,{error:'denied'});return}
        const record=records.find(record=>record.id===match[1]);if(!record){json(response,404,{error:'missing'});return}
        if(request.method==='GET'&&!match[2]){json(response,200,record);return}
        if(request.method==='POST'&&match[2]&&request.headers.origin===origin){const body=await bodyOf(request);if(role!=='staff'||body.run_tag!==record.run_tag){json(response,403,{error:'tagged record required'});return}record.state='cancelled';await save();json(response,200,{state:'cancelled'});return}
      }
      json(response,404,{error:'unsupported test route'});
    }catch{if(!response.headersSent)json(response,400,{error:'invalid test request'});else response.destroy()}
  });
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(port,'127.0.0.1',resolve)});origin=`http://127.0.0.1:${server.address().port}`;
  return {origin,setRevision(value){revision=value},async close(){server.closeAllConnections();await new Promise(resolve=>server.close(resolve));await writeQueue}};
}

if(process.argv[1]&&resolve(process.argv[1])===fileURLToPath(import.meta.url)){
  const [root,appId,environmentId]=process.argv.slice(2);
  if(!root||!appId||!environmentId)throw new Error('Usage: node server.mjs <isolated-test-data-folder> <app-id> <environment-id>');
  const fixture=await startBookingFixture({root:resolve(root),appId,environmentId,port:5185});
  process.stdout.write(`Synthetic booking test preview: ${fixture.origin}\n`);
  for(const signal of ['SIGINT','SIGTERM'])process.on(signal,()=>{void fixture.close().then(()=>process.exit(0))});
}
