import { pathToFileURL } from 'node:url';
import { join,isAbsolute } from 'node:path';
import { createEgressProxy } from './egress-proxy.mjs';

export function locatorFor(page,locator) {
  switch(locator.kind){
    case 'role':return page.getByRole(locator.role,{name:locator.name,exact:true});
    case 'label':return page.getByLabel(locator.text,{exact:true});
    case 'test_id':return page.getByTestId(locator.value);
    default:throw new Error('Unsupported locator');
  }
}

/** Fixed packaged runtime; no PATH, npx, personal profile or debugging listener. */
export async function openSession({bundleRoot,browserExecutable,policy,roles,signal,onBlocked}) {
  const {chromium}=await import(pathToFileURL(join(bundleRoot,'playwright-core','index.mjs')).href);
  const proxy=await createEgressProxy(policy);let browser;
  const contexts=new Map();
  try{
    browser=await chromium.launch({executablePath:isAbsolute(browserExecutable)?browserExecutable:join(bundleRoot,browserExecutable),headless:false,chromiumSandbox:true,
      proxy:{server:proxy.server,username:proxy.username,password:proxy.password,bypass:'<-loopback>'},
      args:['--disable-quic','--disable-background-networking','--disable-component-update','--disable-sync','--no-first-run',
        '--force-webrtc-ip-handling-policy=disable_non_proxied_udp','--disable-features=MediaRouter,OptimizationHints,AutofillServerCommunication,DnsOverHttps,WebTransport','--host-resolver-rules=MAP * ~NOTFOUND, EXCLUDE 127.0.0.1']});
    const close=()=>{void browser.close()};signal?.addEventListener('abort',close,{once:true});
    const denied=()=>{policy.suspend();onBlocked()};
    for(const role of roles){
      const context=await browser.newContext({acceptDownloads:false,serviceWorkers:'block',permissions:[],
        proxy:{server:proxy.server,username:proxy.username,password:proxy.password,bypass:'<-loopback>'}});
      context.setDefaultTimeout(30000);context.setDefaultNavigationTimeout(30000);
      let page;
      await context.route('**/*',async route=>{
        const request=route.request();
        if(signal?.aborted || !page || request.frame().page()!==page || !policy.checkRequest(request.url(),request.method())){denied();await route.abort('blockedbyclient');return}
        await route.continue();
      });
      await context.routeWebSocket('**/*',socket=>{denied();socket.close({code:1008,reason:'WebSockets are not part of this permission'})});
      page=await context.newPage();
      context.on('page',popup=>{if(popup!==page){denied();void popup.close()}});
      page.on('download',download=>{denied();void download.cancel()});
      page.on('filechooser',denied);page.on('dialog',dialog=>void dialog.dismiss());
      contexts.set(role,{context,page});
    }
    return {contexts,async close(){signal?.removeEventListener('abort',close);try{await browser.close()}finally{await proxy.close()}}};
  }catch{try{await browser?.close()}finally{await proxy.close()}throw new Error('Packaged test browser unavailable')}
}
