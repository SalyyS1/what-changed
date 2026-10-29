/** The native parent releases login, individual actions and explicit captures. */
export function createAuthority({input,request,emit,session,abort}) {
  let loginResolve;let loginConfirmed=false;let pending=null;let captureWork=Promise.resolve();let terminal=false;
  const captureIds=new Set();
  const login=new Promise(resolve=>{loginResolve=resolve});
  const check=frame=>{
    const fields={continue:[],allow_step:['step_id'],capture:['capture_id','role']};
    if(!frame || frame.protocol!==1 || frame.run_id!==request.run_id || frame.nonce!==request.nonce || frame.manifest_digest!==request.manifest_digest
      || !Object.hasOwn(fields,frame.type) || Object.keys(frame).some(key=>!['protocol','run_id','nonce','manifest_digest','type',...fields[frame.type]].includes(key)))throw new Error('Control identity mismatch');
  };
  void (async()=>{
    try {for await(const frame of {[Symbol.asyncIterator]:()=>input}){
      check(frame);if(terminal)continue;
      if(frame.type==='continue'){
        if(loginConfirmed)throw new Error('Repeated login release');loginConfirmed=true;loginResolve();
      } else if(frame.type==='allow_step'){
        if(!loginConfirmed || !pending || frame.step_id!==pending.id)throw new Error('Unexpected action release');
        const current=pending;pending=null;current.resolve();
      } else {
        if(!request.consent.capture_screenshots || !session.contexts.has(frame.role) || !/^[0-9a-f-]{36}$/i.test(frame.capture_id??'')
          || captureIds.has(frame.capture_id) || captureIds.size>=50)throw new Error('Capture was not authorized');
        captureIds.add(frame.capture_id);
        captureWork=captureWork.then(async()=>{
          const identity={capture_id:frame.capture_id,role:frame.role};
          try {
            const image=await session.contexts.get(frame.role).page.screenshot({type:'png',fullPage:false,animations:'disabled',timeout:10000});
            if(image.length>512*1024 || abort.signal.aborted)throw new Error('Capture limit');
            await emit('capture',{...identity,data_url:'data:image/png;base64,'+image.toString('base64')});
          }catch{if(!abort.signal.aborted)await emit('capture_unavailable',identity)}
        });
      }
    }} catch {abort.abort()} finally {if(!terminal)abort.abort()}
  })();
  const wait=async promise=>{
    if(abort.signal.aborted)throw new Error('Permission ended');
    let stopped;try{await Promise.race([promise,new Promise((_,reject)=>{stopped=()=>reject(new Error('Permission ended'));abort.signal.addEventListener('abort',stopped,{once:true})})])}
    finally{abort.signal.removeEventListener('abort',stopped)}
  };
  return {
    waitForLogin:()=>wait(login),
    async authorize(scenario,step){
      if(pending||!loginConfirmed)throw new Error('Action authority unavailable');
      const permission=new Promise(resolve=>{pending={id:step.id,resolve}});
      await emit('before_step',{scenario_id:scenario.id,scenario_revision:scenario.revision,step_id:step.id});await wait(permission);
    },
    async finish(){terminal=true;await captureWork},
  };
}
