import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { frames,writer,validateIdentity } from './protocol.mjs';
import { makePolicy } from './policy.mjs';
import { validateRecipe,validateLocator } from './recipe-validation.mjs';
import { openSession } from './browser-session.mjs';
import { executeScenario } from './execute-recipe.mjs';
import { installLocatorPicker } from './record-locator.mjs';
import { readRevisionMarker } from './revision-marker.mjs';
import { createAuthority } from './session-authority.mjs';

const input=frames(process.stdin)[Symbol.asyncIterator]();
let session;let emit;const abort=new AbortController();
process.on('SIGTERM',()=>abort.abort());process.on('SIGINT',()=>abort.abort());
try{
  const first=await input.next();if(first.done)throw new Error('Missing request');
  const request=first.value;validateIdentity(request);emit=writer(process.stdout,request);
  if(Object.keys(request).some(key=>!['protocol','type','run_id','nonce','manifest_digest','manifest','fixture','consent','mode','fixture_id_locator'].includes(key)))throw new Error('Unsupported request');
  const manifest=request.manifest;
  if(!['run','record'].includes(request.mode))throw new Error('Unsupported session mode');
  if(!manifest || manifest.schema_version!==1 || !Array.isArray(manifest.scenarios) || !manifest.scenarios.length || manifest.scenarios.length>10)throw new Error('Unsupported manifest');
  const roles=new Set();for(const scenario of manifest.scenarios){validateRecipe(scenario.recipe);for(const role of scenario.recipe.role_refs)roles.add(role)}
  if(roles.size>2)throw new Error('Role limit');
  if(request.consent?.manifest_digest!==request.manifest_digest || !request.consent.authorized_test_environment || !request.consent.isolated_test_data
    || typeof request.consent.capture_screenshots!=='boolean' || Object.keys(request.consent).some(key=>!['manifest_digest','authorized_test_environment','isolated_test_data','routes','capture_screenshots'].includes(key)))throw new Error('Missing matching permission');
  if(!request.fixture || Object.keys(request.fixture).length>50 || Object.values(request.fixture).some(value=>typeof value!=='string'||value.length>1000))throw new Error('Fixture limit');
  const fixtureLocator=request.fixture_id_locator??{kind:'test_id',value:'ownvero-fixture-id'};validateLocator(fixtureLocator);
  const origins=manifest.environment.origins.map(origin=>`${origin.scheme}://${origin.host.includes(':')?`[${origin.host}]`:origin.host}:${origin.port}`);
  const policy=makePolicy(origins,request.consent.routes,{remoteEnabled:false});
  const bundleRoot=process.env.OWNVERO_BUNDLE_ROOT;if(!bundleRoot)throw new Error('Missing packaged runtime');
  const inventory=JSON.parse(await readFile(join(bundleRoot,'bundle-inventory.json'),'utf8'));
  if(inventory.playwrightVersion!=='1.62.1' || inventory.chromiumRevision!=='156.0.8078.47-1.1' || inventory.browserExecutable!=='browsers/chromium-156.0.8078.47-1.1/chrome.exe')throw new Error('Unsupported browser inventory');
  const timer=setTimeout(()=>abort.abort(),600000);let violation=false;
  try{
    session=await openSession({bundleRoot,browserExecutable:inventory.browserExecutable,policy,roles:[...roles],signal:abort.signal,onBlocked:()=>{violation=true}});
    const authority=createAuthority({input,request,emit,session,abort});
    await emit('ready',{roles:[...roles]});
    // Browser windows are isolated test sessions. Login is a visible owner action.
    for(const [role,entry] of session.contexts){
      if(!policy.checkRequest(manifest.environment.preview_url,'GET'))throw new Error('Preview route missing');
      await entry.page.goto(manifest.environment.preview_url,{waitUntil:'domcontentloaded'});
      await emit('needs_login',{role});
    }
    await authority.waitForLogin();
    if(request.mode==='record'){
      // Selection observes the already-loaded surface. Disable every new request
      // so page event handlers cannot turn a selected locator into server writes.
      policy.suspend();
      for(const {page} of session.contexts.values())await installLocatorPicker(page,locator=>emit('locator',{locator}));
      await emit('recording');if(!abort.signal.aborted)await new Promise(resolve=>abort.signal.addEventListener('abort',resolve,{once:true}));
      throw new Error('Selection ended');
    }
    const readMarker=()=>readRevisionMarker(session,manifest,policy);
    let before=null;try{before=await readMarker()}catch{/* UI observations remain explicitly unbound. */}
    for(const scenario of manifest.scenarios)await executeScenario(scenario,{session,fixture:request.fixture,previewUrl:manifest.environment.preview_url,fixtureLocator,policy,signal:abort.signal,emit,blocked:()=>violation,authorize:authority.authorize});
    let after=null;try{after=await readMarker()}catch{/* Unknown is never a current-build pass. */}
    if(abort.signal.aborted || violation || (before && before!==after))throw new Error('Interrupted, blocked or revision changed');
    await authority.finish();
    await emit('complete',{identity:before&&before===after?'environment_reported':'unknown',revision:before&&before===after?before:null});
  }finally{clearTimeout(timer)}
}catch{if(emit)await emit('interrupted',{reason:'Execution ended without complete authorized evidence.'}).catch(()=>{});process.exitCode=1}
finally{await session?.close().catch(()=>{});process.stdin.destroy()}
