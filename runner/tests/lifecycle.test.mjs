import test from 'node:test';import assert from 'node:assert/strict';
import {Readable} from 'node:stream';
import {setImmediate as tick} from 'node:timers/promises';
import {createAuthority} from '../src/session-authority.mjs';
const request={protocol:1,run_id:'12345678-1234-4234-9234-123456789012',nonce:'a'.repeat(64),manifest_digest:'b'.repeat(64),consent:{capture_screenshots:false}};
function fixture(captures=false){
  const input=new Readable({objectMode:true,read(){}});const abort=new AbortController();const events=[];
  const session={contexts:new Map([['staff',{page:{screenshot:async()=>Buffer.from('raw-credential-canary')}}]])};
  const authority=createAuthority({input:input[Symbol.asyncIterator](),request:{...request,consent:{capture_screenshots:captures}},session,abort,emit:async(type,payload)=>events.push({type,...payload})});
  const send=(type,payload={})=>input.push({...request,consent:undefined,type,...payload});
  // Control messages contain only fixed identity and the type's allowed fields.
  const control=(type,payload={})=>{const {consent,...identity}=request;input.push({...identity,type,...payload})};
  return {input,abort,events,authority,control};
}
test('each exact action waits for a fresh parent release and EOF revokes waiting work',async()=>{
  const f=fixture();f.control('continue');await f.authority.waitForLogin();
  const step={id:'next-step'};let released=false;
  const waiting=f.authority.authorize({id:'scenario',revision:1},step).then(()=>released=true);
  await tick();assert.equal(released,false);assert.equal(f.events[0].type,'before_step');
  f.control('allow_step',{step_id:step.id});await waiting;assert.equal(released,true);
  const cancelled=f.authority.authorize({id:'scenario',revision:1},{id:'another-step'});
  f.input.push(null);await assert.rejects(cancelled,/Permission ended/);assert.equal(f.abort.signal.aborted,true);
});
test('wrong nonce, unknown fields, unsolicited release and capture without consent fail closed',async()=>{
  for(const frame of [{type:'continue',nonce:'c'.repeat(64)},{type:'continue',extra:'page text'},{type:'allow_step',step_id:'other'},
    {type:'capture',capture_id:'12345678-1234-4234-9234-123456789012',role:'staff'}]){
    const f=fixture();const waiting=f.authority.waitForLogin();f.control(frame.type,frame);
    await assert.rejects(waiting,/Permission ended/);assert.equal(f.abort.signal.aborted,true);assert.equal(f.events.some(event=>event.type==='capture'),false);f.input.destroy();
  }
});
test('capture requires explicit request and finish drains its response before completion',async()=>{
  const f=fixture(true);await tick();assert.equal(f.events.length,0);
  f.control('capture',{capture_id:'12345678-1234-4234-9234-123456789012',role:'staff'});await tick();
  await f.authority.finish();assert.equal(f.events.length,1);assert.equal(f.events[0].type,'capture');assert.equal(f.events[0].role,'staff');
  f.input.push(null);await tick();assert.equal(f.abort.signal.aborted,false);
});
