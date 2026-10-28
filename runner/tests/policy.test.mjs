import test from 'node:test';import assert from 'node:assert/strict';
import http from 'node:http';
import {makePolicy,publicAddress,checkedDestination,originOf} from '../src/policy.mjs';
import {createEgressProxy} from '../src/egress-proxy.mjs';
import net from 'node:net';
test('exact origin, method and fixture path are independent grants',()=>{
  const origin='http://127.0.0.1:5185';const policy=makePolicy([origin],[{origin,method:'GET',path:'/api/bookings/{fixture_id}'}]);
  assert.equal(policy.checkRequest(origin+'/api/bookings/abc-123','GET'),true);
  for(const [url,method] of [[origin+'/api/bookings/abc?token=x','GET'],[origin+'/api/bookings/a%2fb','GET'],[origin+'/api/bookings/abc','POST'],['http://127.0.0.1:5186/api/bookings/abc','GET']])assert.equal(policy.checkRequest(url,method),false);
  assert.throws(()=>makePolicy(['https://example.com'],[]));assert.throws(()=>originOf('http://localhost:5185'));assert.throws(()=>originOf('https://user:pass@example.com'));
});
test('DNS answers are rechecked and pinned; private and mapped destinations are denied',async()=>{
  for(const ip of ['127.0.0.1','10.0.0.1','169.254.169.254','::1','::ffff:127.0.0.1','fc00::1'])assert.equal(publicAddress(ip),false);
  assert.equal(publicAddress('8.8.8.8'),true);
  const policy={origins:['https://example.com']};
  const destination=await checkedDestination('example.com',443,policy,async()=>[{address:'8.8.8.8',family:4}]);assert.equal(destination.address,'8.8.8.8');
  await assert.rejects(checkedDestination('example.com',443,policy,async()=>[{address:'8.8.8.8',family:4},{address:'10.0.0.1',family:4}]));
});
test('owned proxy forwards only authenticated approved local routes',async t=>{
  let reached=0;const target=http.createServer((_request,response)=>{reached++;response.end('fixture')});
  await new Promise(resolve=>target.listen(0,'127.0.0.1',resolve));t.after(()=>new Promise(resolve=>target.close(resolve)));
  const origin=`http://127.0.0.1:${target.address().port}`;const proxy=await createEgressProxy(makePolicy([origin],[{origin,method:'GET',path:'/'}]));t.after(()=>proxy.close());
  const request=(url,authenticated)=>new Promise((resolve,reject)=>{
    const settings=new URL(proxy.server);const request=http.request({host:settings.hostname,port:settings.port,path:url,headers:authenticated?{'Proxy-Authorization':'Basic '+Buffer.from(proxy.username+':'+proxy.password).toString('base64')}:{}},response=>{response.resume();response.on('end',()=>resolve(response.statusCode))});request.on('error',reject);request.end();
  });
  assert.equal(await request(origin+'/',false),407);assert.equal(await request(origin+'/blocked',true),403);assert.equal(await request(origin+'/',true),200);assert.equal(reached,1);
});
test('HTTP CONNECT API tunnels retain exact method/path and destination grants',async t=>{
  let reached=0;const target=http.createServer((_request,response)=>{reached++;response.end('synthetic')});
  await new Promise(resolve=>target.listen(0,'127.0.0.1',resolve));t.after(()=>new Promise(resolve=>target.close(resolve)));
  const origin=`http://127.0.0.1:${target.address().port}`;
  const proxy=await createEgressProxy(makePolicy([origin],[{origin,method:'GET',path:'/approved'}]));t.after(()=>proxy.close());
  const auth='Basic '+Buffer.from(proxy.username+':'+proxy.password).toString('base64');
  const tunnel=async(method,path,destination=new URL(origin).host)=>{
    const socket=net.connect({host:'127.0.0.1',port:Number(new URL(proxy.server).port)});let raw='';let sent=false;
    return new Promise((resolve,reject)=>{
      socket.on('connect',()=>socket.write(`CONNECT ${destination} HTTP/1.1\r\nHost: ${destination}\r\nProxy-Authorization: ${auth}\r\n\r\n`));
      socket.on('data',chunk=>{raw+=chunk.toString();if(!sent&&raw.includes('\r\n\r\n')){
        if(!raw.startsWith('HTTP/1.1 200')){socket.destroy();resolve(raw);return}
        sent=true;raw='';socket.write(`${method} ${path} HTTP/1.1\r\nHost: ${new URL(origin).host}\r\nConnection: close\r\n\r\n`);
      }});socket.on('end',()=>resolve(raw));socket.on('error',reject);
    });
  };
  assert.match(await tunnel('GET','/approved'),/^HTTP\/1.1 200/);
  assert.match(await tunnel('POST','/approved'),/^HTTP\/1.1 403/);
  assert.match(await tunnel('GET','/blocked'),/^HTTP\/1.1 403/);
  assert.match(await tunnel('GET','/approved','127.0.0.1:1'),/^HTTP\/1.1 403/);
  assert.equal(reached,1);
});
