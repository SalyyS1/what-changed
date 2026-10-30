/** Target-reported identity only; unknown or stale identity cannot establish currency. */
export async function readRevisionMarker(session,manifest,policy) {
  const url=new URL('/.well-known/ownvero.json',manifest.environment.preview_url).href;
  if(!policy.checkRequest(url,'GET'))return null;
  const response=await session.contexts.values().next().value.context.request.get(url,{maxRedirects:0,headers:{'Cache-Control':'no-cache'},timeout:10000});
  try {
    if(response.status()!==200 || Number(response.headers()['content-length']??0)>8192)return null;
    const age=response.headers().age;
    if(age!==undefined && (!/^\d+$/.test(age)||Number(age)>0))return null;
    const bytes=await response.body();if(bytes.length>8192)return null;
    const marker=JSON.parse(bytes.toString('utf8'));
    if(!marker || Array.isArray(marker) || Object.keys(marker).sort().join(',')!=='appId,environmentId,environmentType,revision,schemaVersion')return null;
    return marker.schemaVersion===1 && marker.appId===manifest.change.app_id && marker.environmentId===manifest.environment.id
      && ['test','local'].includes(marker.environmentType) && typeof marker.revision==='string'
      && marker.revision.length<=1000 && marker.revision===manifest.candidate.identity ? marker.revision : null;
  } finally {await response.dispose()}
}
