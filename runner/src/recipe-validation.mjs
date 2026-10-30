const uuid=value=>typeof value==='string' && /^[0-9a-f-]{36}$/i.test(value);
const text=value=>typeof value==='string' && value.length>0 && value.length<=1000;
function keys(value,allowed){if(!value || typeof value!=='object' || Array.isArray(value) || Object.keys(value).some(key=>!allowed.includes(key)))throw new Error('Unsupported recipe field')}
export function validateLocator(locator) {
  const fields={role:['kind','role','name'],label:['kind','text'],test_id:['kind','value']};
  keys(locator,fields[locator?.kind]??[]);
  if(!fields[locator.kind] || Object.entries(locator).some(([key,value])=>key!=='kind'&&!text(value)))throw new Error('Unsupported locator');
  for(const field of fields[locator.kind])if(!text(locator[field]))throw new Error('Incomplete locator');
}
export function validateRecipe(recipe) {
  keys(recipe,['schema_version','role_refs','fixture_refs','steps','adapters']);
  if(recipe.schema_version!==1 || !Array.isArray(recipe.role_refs) || recipe.role_refs.length<1 || recipe.role_refs.length>2
    || recipe.role_refs.some(role=>!text(role)) || new Set(recipe.role_refs).size!==recipe.role_refs.length
    || !Array.isArray(recipe.fixture_refs) || recipe.fixture_refs.length>50 || recipe.fixture_refs.some(field=>!text(field))
    || !Array.isArray(recipe.steps) || !recipe.steps.length || recipe.steps.length>50 || !Array.isArray(recipe.adapters) || recipe.adapters.length>20)throw new Error('Recipe limits');
  const ids=new Set();const adapters=new Set();let assertions=0;
  for(const adapter of recipe.adapters){
    keys(adapter,['id','origin','path_template','role_ref','allowed_fields','effect']);
    if(!uuid(adapter.id) || adapters.has(adapter.id) || !recipe.role_refs.includes(adapter.role_ref) || !['read_only','read_denied'].includes(adapter.effect)
      || !/^\/[A-Za-z0-9_/-]*\{fixture_id\}[A-Za-z0-9_/-]*$/.test(adapter.path_template)
      || !Array.isArray(adapter.allowed_fields) || adapter.allowed_fields.length>20 || adapter.allowed_fields.some(field=>!/^\w{1,80}$/.test(field)))throw new Error('Unsupported adapter');
    adapters.add(adapter.id);
  }
  const actions={navigate:['kind','path'],fill:['kind','locator','fixture_field'],click:['kind','locator'],cleanup:['kind','locator'],read_back:['kind','adapter_id'],deny_other_role:['kind','adapter_id']};
  const checks={visible:['kind','locator'],text_equals:['kind','locator','expected'],field_equals:['kind','field','fixture_field'],denied:['kind','allowed_statuses','absent_fields']};
  for(const step of recipe.steps){
    keys(step,['id','action','assertions']);
    if(!uuid(step.id)||ids.has(step.id))throw new Error('Step identity');ids.add(step.id);
    const action=step.action;keys(action,actions[action?.kind]??[]);
    if(!actions[action.kind])throw new Error('Unsupported action');
    if(['fill','click','cleanup'].includes(action.kind))validateLocator(action.locator);
    if(action.kind==='navigate' && !/^\/[A-Za-z0-9_/-]*$/.test(action.path))throw new Error('Navigation path');
    if(action.kind==='fill'&&!recipe.fixture_refs.includes(action.fixture_field))throw new Error('Fixture field');
    if(action.adapter_id&&!adapters.has(action.adapter_id))throw new Error('Missing adapter');
    if(!Array.isArray(step.assertions) || step.assertions.length>20)throw new Error('Assertion limits');
    for(const named of step.assertions){
      keys(named,['id','expected_behavior','assertion']);
      if(!uuid(named.id)||ids.has(named.id)||!text(named.expected_behavior))throw new Error('Assertion identity');ids.add(named.id);
      const check=named.assertion;keys(check,checks[check?.kind]??[]);
      if(!checks[check.kind])throw new Error('Unsupported assertion');
      if(['visible','text_equals'].includes(check.kind))validateLocator(check.locator);
      if(check.kind==='text_equals'&&!text(check.expected))throw new Error('Expected text');
      if(check.kind==='field_equals'&&(!/^\w{1,80}$/.test(check.field)||!recipe.fixture_refs.includes(check.fixture_field)))throw new Error('Field assertion');
      const adapter=recipe.adapters.find(adapter=>adapter.id===action.adapter_id);
      if(action.kind==='read_back'&&adapter?.effect!=='read_only' || action.kind==='deny_other_role'&&adapter?.effect!=='read_denied')throw new Error('Adapter action purpose');
      if(check.kind==='field_equals'&&(action.kind!=='read_back'||!adapter?.allowed_fields.includes(check.field)))throw new Error('Read-back assertion purpose');
      if(check.kind==='denied'&&(!Array.isArray(check.allowed_statuses)||!check.allowed_statuses.length||check.allowed_statuses.length>2||check.allowed_statuses.some(status=>![403,404].includes(status))||!Array.isArray(check.absent_fields)||check.absent_fields.length>50||check.absent_fields.some(field=>!/^\w{1,80}$/.test(field)||!adapter?.allowed_fields.includes(field))||action.kind!=='deny_other_role'))throw new Error('Denied assertion');
      assertions++;
    }
  }
  if(!assertions || assertions>1000)throw new Error('No expected assertions');
  return recipe;
}
