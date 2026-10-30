import test from 'node:test';import assert from 'node:assert/strict';import {randomUUID as id} from 'node:crypto';
import {validateRecipe} from '../src/recipe-validation.mjs';
import {executeScenario} from '../src/execute-recipe.mjs';
import {makePolicy} from '../src/policy.mjs';
const origin='http://127.0.0.1:5185';
function recipe(){return {schema_version:1,role_refs:['staff'],fixture_refs:['note'],adapters:[],steps:[
  {id:id(),action:{kind:'fill',locator:{kind:'label',text:'Booking note'},fixture_field:'note'},assertions:[{id:id(),expected_behavior:'The field is visible',assertion:{kind:'visible',locator:{kind:'label',text:'Booking note'}}}]},
  {id:id(),action:{kind:'click',locator:{kind:'role',role:'button',name:'Save booking'}},assertions:[{id:id(),expected_behavior:'The record ID is visible',assertion:{kind:'visible',locator:{kind:'test_id',value:'ownvero-fixture-id'}}}]},
]}}
test('recipe validation rejects executable fields, missing locators and incompatible backend assertions',()=>{
  assert.doesNotThrow(()=>validateRecipe(recipe()));
  for(const mutate of [r=>r.script='eval',r=>delete r.steps[0].action.locator,r=>r.steps[0].action.kind='evaluate',
    r=>r.steps[0].assertions[0].assertion={kind:'field_equals',field:'note',fixture_field:'note'},
    r=>r.steps[0].assertions[0].assertion={kind:'denied',allowed_statuses:[200],absent_fields:[]}]){
    const r=recipe();mutate(r);assert.throws(()=>validateRecipe(r));
  }
});
test('an unavailable action blocks later actions and still reports every assertion',async()=>{
  const r=recipe();const events=[];let clicks=0;let releases=0;
  const locator={getAttribute:async name=>name==='type'?'password':null,fill:async()=>assert.fail('password was filled'),click:async()=>clicks++,isVisible:async()=>true};
  const page={getByLabel:()=>locator,getByRole:()=>locator,getByTestId:()=>locator};
  await executeScenario({id:id(),revision:1,recipe:r},{session:{contexts:new Map([['staff',{page}]])},fixture:{note:'synthetic'},previewUrl:origin,
    policy:makePolicy([origin],[]),signal:new AbortController().signal,emit:async(type,payload)=>events.push({type,...payload}),blocked:()=>false,authorize:async()=>releases++});
  assert.equal(clicks,0);assert.equal(releases,2);assert.deepEqual(events.filter(event=>event.type==='assertion').map(event=>event.status),['blocked','blocked']);
});
test('a known failed UI assertion remains failed with sanitized observations',async()=>{
  const r=recipe();r.steps.splice(0,1);const events=[];
  const page={getByRole:()=>({click:async()=>{}}),getByTestId:()=>({isVisible:async()=>false})};
  await executeScenario({id:id(),revision:1,recipe:r},{session:{contexts:new Map([['staff',{page}]])},fixture:{note:'synthetic'},previewUrl:origin,
    policy:makePolicy([origin],[]),signal:new AbortController().signal,emit:async(type,payload)=>events.push({type,...payload}),blocked:()=>false,authorize:async()=>{}});
  assert.deepEqual(events.map(event=>event.type),['step','assertion']);assert.equal(events[1].status,'failed');
});
