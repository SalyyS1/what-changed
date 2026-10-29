import {readFile,mkdtemp,rm} from 'node:fs/promises';
import {dirname,join} from 'node:path';
import {tmpdir} from 'node:os';
import {fileURLToPath} from 'node:url';
import {createInterface} from 'node:readline/promises';
import {makePolicy} from './policy.mjs';
import {validateRecipe} from './recipe-validation.mjs';
import {openSession} from './browser-session.mjs';
import {executeScenario} from './execute-recipe.mjs';
import {readRevisionMarker} from './revision-marker.mjs';

// This is an independent reviewed test artifact. It grants no Ownvero authority.
if(!process.argv.includes('--authorize-isolated-test'))throw new Error('Review README.md, then explicitly authorize this isolated test with --authorize-isolated-test');
const {manifest,routes}=JSON.parse(await readFile(new URL('../manifest.json',import.meta.url),'utf8'));
const fixture=JSON.parse(await readFile(new URL('../fixture.json',import.meta.url),'utf8'));
const origin=origin=>`${origin.scheme}://${origin.host.includes(':')?`[${origin.host}]`:origin.host}:${origin.port}`;
for(const scenario of manifest.scenarios)validateRecipe(scenario.recipe);
const policy=makePolicy(manifest.environment.origins.map(origin),routes);
const roles=[...new Set(manifest.scenarios.flatMap(scenario=>scenario.recipe.role_refs))];
const {chromium}=await import('playwright-core');
const bundleRoot=dirname(fileURLToPath(import.meta.resolve('playwright-core')));
const packagedRoot=dirname(bundleRoot);
const profile=await mkdtemp(join(tmpdir(),'ownvero-export-test-'));
const previousTemp={TEMP:process.env.TEMP,TMP:process.env.TMP};process.env.TEMP=profile;process.env.TMP=profile;
const abort=new AbortController();const timer=setTimeout(()=>abort.abort(),600000);
const stop=()=>abort.abort();process.on('SIGINT',stop);process.on('SIGTERM',stop);
let session;let blocked=false;const outcomes=[];
const terminal=createInterface({input:process.stdin,output:process.stdout});
try {
  session=await openSession({bundleRoot:packagedRoot,browserExecutable:chromium.executablePath(),policy,roles,signal:abort.signal,onBlocked:()=>{blocked=true}});
  for(const [role,{page}] of session.contexts){await page.goto(manifest.environment.preview_url);process.stdout.write(`Sign in as the ${role} TEST role in its isolated window.\n`)}
  await terminal.question('After all test roles are ready, press Enter to run the reviewed recipe. ',{signal:abort.signal});
  let before=null;try{before=await readRevisionMarker(session,manifest,policy)}catch{}
  for(const scenario of manifest.scenarios)await executeScenario(scenario,{session,fixture,previewUrl:manifest.environment.preview_url,
    fixtureLocator:{kind:'test_id',value:'ownvero-fixture-id'},policy,signal:abort.signal,blocked:()=>blocked,
    authorize:async()=>{if(abort.signal.aborted||blocked)throw new Error('Test permission ended')},
    emit:async(type,payload)=>{if(type==='assertion'){outcomes.push(payload.status);process.stdout.write(JSON.stringify({type,...payload})+'\n')}}});
  let after=null;try{after=await readRevisionMarker(session,manifest,policy)}catch{}
  const identity=before&&before===after?'environment_reported':'unknown';
  process.stdout.write(JSON.stringify({identity,revision:identity==='unknown'?null:before,assertions:outcomes.length})+'\n');
  if(blocked||abort.signal.aborted||identity==='unknown'||!outcomes.length||outcomes.some(status=>status!=='passed'))process.exitCode=1;
} catch {process.stderr.write('Test interrupted or unavailable; no pass inferred.\n');process.exitCode=1}
finally {
  clearTimeout(timer);terminal.close();await session?.close();
  for(const key of ['TEMP','TMP']){if(previousTemp[key]===undefined)delete process.env[key];else process.env[key]=previousTemp[key]}
  await rm(profile,{recursive:true,force:true});
}
