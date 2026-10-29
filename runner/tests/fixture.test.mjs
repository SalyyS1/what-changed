import test from 'node:test';import assert from 'node:assert/strict';
import {mkdtemp,readFile} from 'node:fs/promises';import {tmpdir} from 'node:os';import {join} from 'node:path';
import {startBookingFixture} from '../fixtures/booking-app/server.mjs';
test('booking read-back survives a server restart and another role is denied',async()=>{
  const root=await mkdtemp(join(tmpdir(),'ownvero-fixture-'));
  const config={root,appId:'app',environmentId:'environment'};let fixture=await startBookingFixture(config);
  try{
    const login=await fetch(fixture.origin+'/signin/staff',{redirect:'manual'});const cookie=login.headers.get('set-cookie').split(';')[0];
    const created=await fetch(fixture.origin+'/api/bookings',{method:'POST',headers:{Cookie:cookie,Origin:fixture.origin,'Content-Type':'application/json'},body:JSON.stringify({note:'Synthetic booking',run_tag:'ownvero-test'})});
    assert.equal(created.status,201);const {id}=await created.json();assert.equal(JSON.parse(await readFile(join(root,'bookings.json'),'utf8'))[0].id,id);
    await fixture.close();fixture=await startBookingFixture(config);
    const again=await fetch(fixture.origin+'/signin/staff',{redirect:'manual'});const staff=again.headers.get('set-cookie').split(';')[0];
    const read=await fetch(fixture.origin+'/api/bookings/'+id,{headers:{Cookie:staff}});assert.equal((await read.json()).note,'Synthetic booking');
    const other=await fetch(fixture.origin+'/signin/customer',{redirect:'manual'});const customer=other.headers.get('set-cookie').split(';')[0];
    const denied=await fetch(fixture.origin+'/api/bookings/'+id,{headers:{Cookie:customer}});assert.equal(denied.status,403);assert.equal(Object.hasOwn(await denied.json(),'note'),false);
  }finally{await fixture.close()}
});
