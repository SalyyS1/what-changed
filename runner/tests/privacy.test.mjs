import test from 'node:test';import assert from 'node:assert/strict';
import {readRevisionMarker} from '../src/revision-marker.mjs';
import {makePolicy} from '../src/policy.mjs';
const origin='http://127.0.0.1:5185';
const manifest={environment:{id:'environment',preview_url:origin},change:{app_id:'app'},candidate:{identity:'fixture-v1'}};
const marker={schemaVersion:1,appId:'app',environmentId:'environment',revision:'fixture-v1',environmentType:'local'};
const policy=makePolicy([origin],[{origin,method:'GET',path:'/.well-known/ownvero.json'}]);
function session(body,headers={}){return {contexts:new Map([['staff',{context:{request:{get:async()=>({status:()=>200,headers:()=>headers,body:async()=>Buffer.from(JSON.stringify(body)),dispose:async()=>{}})}}}]])}}
test('build currency requires bounded exact marker schema and unaged matching environment identity',async()=>{
  assert.equal(await readRevisionMarker(session(marker),manifest,policy),'fixture-v1');
  for(const body of [{...marker,revision:'old'},{...marker,environmentId:'other'},{...marker,schemaVersion:2},{...marker,instruction:'credential-canary'},[]])assert.equal(await readRevisionMarker(session(body),manifest,policy),null);
  assert.equal(await readRevisionMarker(session(marker,{age:'30'}),manifest,policy),null);
  assert.equal(await readRevisionMarker(session(marker,{'content-length':'9000'}),manifest,policy),null);
});
test('selection revokes every new route including previously approved fixture writes',()=>{
  const policy=makePolicy([origin],[{origin,method:'GET',path:'/'},{origin,method:'POST',path:'/api/bookings'}]);
  assert.equal(policy.checkRequest(origin+'/api/bookings','POST'),true);policy.suspend();
  assert.equal(policy.checkRequest(origin+'/api/bookings','POST'),false);assert.equal(policy.checkRequest(origin+'/','GET'),false);
});
