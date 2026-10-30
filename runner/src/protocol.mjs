import { once } from 'node:events';
export const PROTOCOL=1;
export const MAX_FRAME=1024*1024;

export async function* frames(stream) {
  let pending=Buffer.alloc(0);
  for await(const chunk of stream){
    pending=Buffer.concat([pending,chunk]);
    while(true){
      const end=pending.indexOf(10);
      if(end<0){if(pending.length>MAX_FRAME)throw new Error('Frame too large');break}
      if(end>MAX_FRAME)throw new Error('Frame too large');
      const frame=pending.subarray(0,end);pending=pending.subarray(end+1);
      if(!frame.length)throw new Error('Empty frame');
      const parsed=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(frame));
      yield parsed;
    }
  }
  if(pending.length)throw new Error('Truncated frame');
}

export function writer(stream,identity) {
  let sequence=0;
  return async(type,payload={})=>{
    const frame=JSON.stringify({...payload,protocol:PROTOCOL,run_id:identity.run_id,nonce:identity.nonce,manifest_digest:identity.manifest_digest,sequence:++sequence,type})+'\n';
    if(Buffer.byteLength(frame)>MAX_FRAME)throw new Error('Output too large');
    if(!stream.write(frame))await once(stream,'drain');
  };
}

export function validateIdentity(request) {
  if(request?.protocol!==PROTOCOL || request.type!=='run' || !/^[0-9a-f-]{36}$/i.test(request.run_id??'')
    || !/^[0-9a-f]{64}$/.test(request.nonce??'') || !/^[0-9a-f]{64}$/.test(request.manifest_digest??''))throw new Error('Invalid run identity');
}
