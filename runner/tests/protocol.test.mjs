import test from 'node:test';
import assert from 'node:assert/strict';
import {Readable,Writable} from 'node:stream';
import {frames,writer,validateIdentity,MAX_FRAME} from '../src/protocol.mjs';
const identity={run_id:'12345678-1234-4234-9234-123456789012',nonce:'a'.repeat(64),manifest_digest:'b'.repeat(64)};
test('fragmented UTF-8 and multiple framed messages preserve identity and ordering',async()=>{
  const chunks=[];const output=new Writable({write(chunk,_encoding,done){chunks.push(Buffer.from(chunk));done()}});
  const emit=writer(output,identity);await emit('step',{observed:'Ghi chú'});await emit('complete');
  const bytes=Buffer.concat(chunks);const received=[];
  for await(const frame of frames(Readable.from([bytes.subarray(0,80),bytes.subarray(80,143),bytes.subarray(143)])))received.push(frame);
  assert.equal(received[0].observed,'Ghi chú');assert.deepEqual(received.map(frame=>frame.sequence),[1,2]);
  assert.equal(received[1].nonce,identity.nonce);
});
test('oversized, malformed, invalid UTF-8 and truncated messages fail closed',async()=>{
  for(const bytes of [Buffer.alloc(MAX_FRAME+1,65),Buffer.from('{bad}\n'),Buffer.from([0xff,10]),Buffer.from('{}')]){
    await assert.rejects(async()=>{for await(const _frame of frames(Readable.from([bytes]))){}});
  }
  assert.throws(()=>validateIdentity({...identity,protocol:2,type:'run'}));
});
