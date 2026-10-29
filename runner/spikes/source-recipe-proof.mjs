// Opt-in source execution proof only. Run inside the owned-process launcher.
import {mkdtemp,rm,readFile,writeFile} from 'node:fs/promises';
import {join,resolve} from 'node:path';import {randomUUID as id} from 'node:crypto';
import {startBookingFixture} from '../fixtures/booking-app/server.mjs';
import {openSession} from '../src/browser-session.mjs';
import {makePolicy} from '../src/policy.mjs';
import {validateRecipe} from '../src/recipe-validation.mjs';
import {executeScenario} from '../src/execute-recipe.mjs';
import {readRevisionMarker} from '../src/revision-marker.mjs';
import {writeFileSync} from 'node:fs';
const bundleRoot=process.env.OWNVERO_TEST_BROWSER_BUNDLE;const runRoot=process.env.OWNVERO_TEST_RUN_ROOT;
if(!bundleRoot||!runRoot)throw new Error('Explicit owned source-proof environment required');
const inventory=JSON.parse(await readFile(join(bundleRoot,'bundle-inventory.json'),'utf8'));
process.on('uncaughtException',error=>{writeFileSync(join(runRoot,'source-recipe-proof-crash.json'),JSON.stringify({message:error.message,stack:error.stack}));process.exit(1)});
const reports=[];
for(const mode of ['successful','lost-persistence','other-role-exposure']){
  const data=await mkdtemp(join(runRoot,'synthetic-'));const appId=id(),environmentId=id();
  const fixtureServer=await startBookingFixture({root:data,appId,environmentId,losePersistence:mode==='lost-persistence',leakOtherRole:mode==='other-role-exposure'});
  let session;const abort=new AbortController();const timer=setTimeout(()=>abort.abort(),90000);let blocked=false;
  const origin=fixtureServer.origin;const grantOrigin={scheme:'http',host:'127.0.0.1',port:Number(new URL(origin).port)};
  const routes=[['GET','/'],['GET','/signin/staff'],['GET','/signin/customer'],['GET','/api/session'],['GET','/.well-known/ownvero.json'],['GET','/favicon.ico'],['GET','/api/bookings/{fixture_id}'],['POST','/api/bookings'],['POST','/api/bookings/{fixture_id}/cancel']].map(([method,path])=>({origin,method,path}));
  const policy=makePolicy([origin],routes);const read=id(),denied=id();
  const assertion=(expected_behavior,assertion)=>({id:id(),expected_behavior,assertion});
  const recipe={schema_version:1,role_refs:['staff','customer'],fixture_refs:['note','run_tag'],adapters:[
    {id:read,origin:grantOrigin,path_template:'/api/bookings/{fixture_id}',role_ref:'staff',allowed_fields:['note','run_tag'],effect:'read_only'},
    {id:denied,origin:grantOrigin,path_template:'/api/bookings/{fixture_id}',role_ref:'customer',allowed_fields:['note','run_tag'],effect:'read_denied'},
  ],steps:[
    {id:id(),action:{kind:'fill',locator:{kind:'label',text:'Booking note'},fixture_field:'note'},assertions:[]},
    {id:id(),action:{kind:'fill',locator:{kind:'label',text:'Test run tag'},fixture_field:'run_tag'},assertions:[]},
    {id:id(),action:{kind:'click',locator:{kind:'role',role:'button',name:'Save booking'}},assertions:[assertion('Created record is identified',{kind:'visible',locator:{kind:'test_id',value:'ownvero-fixture-id'}})]},
    {id:id(),action:{kind:'read_back',adapter_id:read},assertions:[assertion('Exact note persists',{kind:'field_equals',field:'note',fixture_field:'note'})]},
    {id:id(),action:{kind:'deny_other_role',adapter_id:denied},assertions:[assertion('Customer cannot read staff record',{kind:'denied',allowed_statuses:[403,404],absent_fields:['note','run_tag']})]},
  ]};
  validateRecipe(recipe);const observations=[];const synthetic={note:'Synthetic source execution proof',run_tag:'ownvero-'+id()};
  try{
    session=await openSession({bundleRoot,browserExecutable:inventory.browserExecutable,policy,roles:recipe.role_refs,signal:abort.signal,onBlocked:()=>{blocked=true}});
    // Synthetic fixture identities only; this harness does not support real logins.
    for(const [role,{page}] of session.contexts)await page.goto(origin+'/signin/'+role,{waitUntil:'networkidle'});
    const manifest={environment:{id:environmentId,preview_url:origin},change:{app_id:appId},candidate:{identity:'fixture-v1'}};
    const before=await readRevisionMarker(session,manifest,policy);
    await executeScenario({id:id(),revision:1,recipe},{session,fixture:synthetic,previewUrl:origin,fixtureLocator:{kind:'test_id',value:'ownvero-fixture-id'},policy,signal:abort.signal,blocked:()=>blocked,
      authorize:async()=>{if(abort.signal.aborted||blocked)throw new Error('Synthetic authority ended')},emit:async(type,payload)=>{if(type==='assertion')observations.push(payload.status)}});
    const after=await readRevisionMarker(session,manifest,policy);
    const expected=mode==='successful'?['passed','passed','passed']:mode==='lost-persistence'?['passed','failed','passed']:['passed','passed','failed'];
    const page=session.contexts.get('staff').page;
    const rawMarker=await session.contexts.get('staff').context.request.get(origin+'/.well-known/ownvero.json',{maxRedirects:0});
    const diagnostic={blocked,role:await page.getByTestId('session-role').innerText(),ui_status:await page.locator('#status').innerText(),markerBefore:before,markerAfter:after,adapterTransportStatus:rawMarker.status(),fixtureMarker:(await rawMarker.text()).slice(0,600)};await rawMarker.dispose();
    const matches=JSON.stringify(observations)===JSON.stringify(expected)&&before==='fixture-v1'&&after===before&&!blocked;
    reports.push({case:mode,status:matches?'matched_expected':'mismatch',expected,observed:observations,identity:before===after?before:null,diagnostic,scope:'Synthetic current-source interpreter proof; native IPC/installation/network confinement acceptance not established.'});
    if(!matches)process.exitCode=1;
  }catch(error){reports.push({case:mode,status:'unavailable',observed:observations,diagnostic:String(error.message).slice(0,500)});process.exitCode=1}
  finally{clearTimeout(timer);await session?.close();await fixtureServer.close();await rm(data,{recursive:true,force:true})}
}
await writeFile(join(runRoot,'source-recipe-proof.json'),JSON.stringify({schemaVersion:1,cases:reports},null,2));
process.stdout.write(JSON.stringify({cases:reports.map(row=>({case:row.case,status:row.status,observed:row.observed}))})+'\n');
