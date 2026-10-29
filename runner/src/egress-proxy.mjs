import http from 'node:http';
import net from 'node:net';
import { lookup } from 'node:dns/promises';
import { randomBytes, timingSafeEqual } from 'node:crypto';
import { checkedDestination } from './policy.mjs';

export async function createEgressProxy(policy) {
  const secret=randomBytes(24).toString('hex');
  const authorization='Basic '+Buffer.from(`ownvero:${secret}`).toString('base64');
  const sockets=new Set();
  const tunnelOrigins=new WeakMap();
  const authenticated=request=>{
    if(tunnelOrigins.has(request.socket))return true;
    const actual=Buffer.from(request.headers['proxy-authorization']??'');
    const expected=Buffer.from(authorization);
    return actual.length===expected.length && timingSafeEqual(actual,expected);
  };
  const deny=response=>{response.writeHead(403,{'Content-Type':'text/plain'});response.end('Preview connection blocked');};
  const server=http.createServer({maxHeaderSize:8192},async(request,response)=>{
    if (!authenticated(request)) {response.writeHead(407,{'Proxy-Authenticate':'Basic realm="Ownvero"'});response.end();return}
    try {
      const tunnelOrigin=tunnelOrigins.get(request.socket);
      const url=tunnelOrigin?new URL(request.url,tunnelOrigin):new URL(request.url);
      if(tunnelOrigin && url.origin!==tunnelOrigin){deny(response);return}
      if(Number(request.headers['content-length']??0)>65536){deny(response);return}
      if (url.protocol!=='http:' || !policy.checkRequest(url.href,request.method)) {deny(response);return}
      const target=await checkedDestination(url.hostname.replace(/^\[|\]$/g,''),Number(url.port||80),policy,lookup);
      const headers={...request.headers,host:url.host};delete headers['proxy-authorization'];delete headers['proxy-connection'];
      const upstream=http.request({hostname:target.address,port:Number(url.port||80),method:request.method,path:url.pathname,headers,timeout:30000},incoming=>{
        if(Number(incoming.headers['content-length']??0)>2*1024*1024){incoming.destroy();deny(response);return}
        let size=0;incoming.on('data',chunk=>{size+=chunk.length;if(size>2*1024*1024){incoming.destroy();response.destroy()}});
        response.writeHead(incoming.statusCode,incoming.headers);incoming.pipe(response);
      });
      upstream.on('socket',socket=>{sockets.add(socket);socket.on('close',()=>sockets.delete(socket))});
      upstream.on('timeout',()=>upstream.destroy());
      upstream.on('error',()=>{if(!response.headersSent)deny(response);else response.destroy()});
      let sent=0;request.on('data',chunk=>{sent+=chunk.length;if(sent>65536){upstream.destroy();request.destroy()}});
      request.on('aborted',()=>upstream.destroy());request.pipe(upstream);
    }catch{if(!response.headersSent)deny(response);else response.destroy()}
  });
  server.on('connect',async(request,client,head)=>{
    if (!authenticated(request)) {client.end('HTTP/1.1 407 Proxy Authentication Required\r\nProxy-Authenticate: Basic realm="Ownvero"\r\n\r\n');return}
    let upstream;
    try {
      if (head.length || !/^(?:[a-zA-Z0-9.-]+|\[::1\]):[0-9]{1,5}$/.test(request.url)) throw new Error('CONNECT authority');
      const authority=new URL('http://'+request.url);const host=authority.hostname.replace(/^\[|\]$/g,'');const port=Number(authority.port||80);
      // Playwright APIRequestContext uses CONNECT for HTTP too. Terminate its
      // local tunnel in this HTTP parser so every method/path is still checked.
      const localOrigin=authority.origin;
      if(!tunnelOrigins.has(client) && policy.origins.includes(localOrigin)){
        await checkedDestination(host,port,policy,lookup);tunnelOrigins.set(client,localOrigin);
        client.write('HTTP/1.1 200 Connection Established\r\n\r\n');server.emit('connection',client);return;
      }
      if (!policy.checkOrigin(`https://${host}:${port}`)) throw new Error('CONNECT origin');
      const target=await checkedDestination(host,port,policy,lookup);
      upstream=net.connect({host:target.address,port});sockets.add(upstream);
      upstream.setTimeout(30000,()=>upstream.destroy());
      upstream.once('connect',()=>{client.write('HTTP/1.1 200 Connection Established\r\n\r\n');client.pipe(upstream);upstream.pipe(client)});
      upstream.on('close',()=>sockets.delete(upstream));upstream.on('error',()=>client.destroy());client.on('error',()=>upstream.destroy());client.on('close',()=>upstream.destroy());
    }catch{upstream?.destroy();client.end('HTTP/1.1 403 Forbidden\r\n\r\n')}
  });
  server.on('upgrade',(_request,socket)=>socket.destroy());
  server.on('clientError',(_error,socket)=>socket.destroy());
  server.on('connection',socket=>{sockets.add(socket);socket.on('error',()=>socket.destroy());socket.on('close',()=>sockets.delete(socket))});
  server.requestTimeout=30000;server.headersTimeout=10000;server.maxConnections=64;
  await new Promise((resolve,reject)=>{server.once('error',reject);server.listen(0,'127.0.0.1',resolve)});
  return {server:`http://127.0.0.1:${server.address().port}`,username:'ownvero',password:secret,
    async close(){for(const socket of sockets)socket.destroy();await new Promise(resolve=>server.close(resolve))}};
}
