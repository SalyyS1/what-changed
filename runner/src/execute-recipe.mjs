import { locatorFor } from './browser-session.mjs';
import {setTimeout as pause} from 'node:timers/promises';

async function eventually(check){
  const deadline=performance.now()+30000;
  while(true){if(await check())return true;if(performance.now()>=deadline)return false;await pause(50)}
}

function originUrl(origin){return `${origin.scheme}://${origin.host.includes(':')?`[${origin.host}]`:origin.host}:${origin.port}`}

async function readAdapter(adapter,scenario,session,fixtureLocator,primary,policy) {
  const role=session.contexts.get(adapter.role_ref);
  if(!role)throw new Error('Required test role is missing');
  let fixtureId='';
  await eventually(async()=>{fixtureId=(await locatorFor(primary.page,fixtureLocator).innerText()).trim();return /^[A-Za-z0-9-]{1,128}$/.test(fixtureId)});
  if(!/^[A-Za-z0-9-]{1,128}$/.test(fixtureId))throw new Error('The created test record could not be identified');
  const url=new URL(adapter.path_template.replace('{fixture_id}',encodeURIComponent(fixtureId)),originUrl(adapter.origin));
  if(!policy.checkRequest(url.href,'GET'))throw new Error('Read-back route is outside permission');
  const response=await role.context.request.get(url.href,{maxRedirects:0,timeout:30000});
  try{
    if(response.status()>=300 && response.status()<400)throw new Error('Adapter redirect blocked');
    if(Number(response.headers()['content-length']??0)>65536)throw new Error('Adapter response limit');
    const bytes=await response.body();if(bytes.length>65536)throw new Error('Adapter response limit');
    let body;try{body=JSON.parse(bytes.toString('utf8'))}catch{throw new Error('Adapter requires bounded JSON')}
    if(!body || Array.isArray(body) || typeof body!=='object')throw new Error('Adapter response schema');
    const fields={};for(const field of adapter.allowed_fields)if(Object.hasOwn(body,field)&&['string','number','boolean'].includes(typeof body[field]))fields[field]=body[field];
    return {status:response.status(),fields,bodyKeys:Object.keys(body),fixtureId};
  }finally{await response.dispose()}
}

async function checkAssertion(assertion,{page,adapter,fixture}) {
  switch(assertion.kind){
    case 'visible':return eventually(()=>locatorFor(page,assertion.locator).isVisible());
    case 'text_equals':return eventually(async()=>(await locatorFor(page,assertion.locator).innerText()).trim()===assertion.expected);
    case 'field_equals':{
      if(adapter && adapter.status!==200)return false;
      if(!adapter || !Object.hasOwn(adapter.fields,assertion.field) || !Object.hasOwn(fixture,assertion.fixture_field))throw new Error('Read-back field is unavailable');
      return adapter.status===200 && adapter.fields[assertion.field]===fixture[assertion.fixture_field];
    }
    case 'denied':{
      if(!adapter)throw new Error('Separate-role adapter result is unavailable');
      return assertion.allowed_statuses.includes(adapter.status) && assertion.absent_fields.every(field=>!adapter.bodyKeys.includes(field));
    }
    default:throw new Error('Unsupported assertion');
  }
}

/** Only numeric/constant observations leave the browser, never page text or auth. */
export async function executeScenario(scenario,{session,fixture,previewUrl,fixtureLocator,policy,signal,emit,blocked,authorize}) {
  const primary=session.contexts.get(scenario.recipe.role_refs[0]);
  if(!primary)throw new Error('Required test role is missing');
  let stopped=false;
  for(const step of scenario.recipe.steps){
    if(signal.aborted)throw new Error('Run cancelled');
    await authorize(scenario,step);
    let adapter;let actionCompleted=false;
    if(!stopped){
      try{
        if(blocked())throw new Error('A request left the granted scope');
        const action=step.action;
        switch(action.kind){
          case 'navigate':{
            const url=new URL(action.path,previewUrl);if(!policy.checkRequest(url.href,'GET'))throw new Error('Navigation is not permitted');
            const response=await primary.page.goto(url.href,{waitUntil:'domcontentloaded'});
            if(response && response.status()>=400)throw new Error('Preview navigation failed');break;
          }
          case 'fill':{
            const locator=locatorFor(primary.page,action.locator);
            const type=(await locator.getAttribute('type')??'').toLowerCase();
            const attributes=await Promise.all(['autocomplete','name','id','aria-label'].map(name=>locator.getAttribute(name)));
            if(['password','file','hidden'].includes(type) || /password|credit-card|cc-|secret|token|credential/i.test(attributes.join(' ')))throw new Error('Sensitive fields are not recipe inputs');
            if(!Object.hasOwn(fixture,action.fixture_field))throw new Error('Test fixture value is missing');
            await locator.fill(fixture[action.fixture_field]);break;
          }
          case 'click':case 'cleanup':await locatorFor(primary.page,action.locator).click();break;
          case 'read_back':case 'deny_other_role':{
            const definition=scenario.recipe.adapters.find(adapter=>adapter.id===action.adapter_id);
            if(action.kind==='deny_other_role' && definition.role_ref===scenario.recipe.role_refs[0])throw new Error('Denied-access check requires another role');
            adapter=await readAdapter(definition,scenario,session,fixtureLocator,primary,policy);break;
          }
          default:throw new Error('Unsupported action');
        }
        if(blocked())throw new Error('A request left the granted scope');
        actionCompleted=true;
        await emit('step',{scenario_id:scenario.id,scenario_revision:scenario.revision,step_id:step.id,observed:'Approved action completed.'});
      }catch{stopped=true}
    }
    for(const expected of step.assertions){
      let status='blocked';let observed='Action or prerequisite unavailable; no pass inferred.';
      if(actionCompleted){
        try{const passed=await checkAssertion(expected.assertion,{page:primary.page,adapter,fixture});status=passed?'passed':'failed';observed=passed?'The named assertion matched.':'The named assertion did not match.'}
        catch{status='blocked';observed='Required locator, role, response or field was unavailable.'}
      }
      await emit('assertion',{scenario_id:scenario.id,scenario_revision:scenario.revision,step_id:step.id,assertion_id:expected.id,status,observed});
    }
  }
}
