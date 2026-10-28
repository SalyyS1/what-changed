import { BlockList, isIP } from 'node:net';

// Separate families: Node's IPv6 mapped subnet rules also match IPv4 queries.
const denied4 = new BlockList();const denied6 = new BlockList();
for (const [base, bits] of [['0.0.0.0',8],['10.0.0.0',8],['100.64.0.0',10],['127.0.0.0',8],['169.254.0.0',16],['172.16.0.0',12],['192.0.0.0',24],['192.168.0.0',16],['198.18.0.0',15],['224.0.0.0',4],['240.0.0.0',4]]) denied4.addSubnet(base,bits,'ipv4');
for (const [base,bits] of [['::',128],['::1',128],['::ffff:0:0',96],['64:ff9b::',96],['100::',64],['fc00::',7],['fe80::',10],['ff00::',8]]) denied6.addSubnet(base,bits,'ipv6');

export function publicAddress(ip) {
  const version=isIP(ip);
  return version !== 0 && !(version===4?denied4:denied6).check(ip,version===4?'ipv4':'ipv6');
}

export function originOf(value) {
  const url=new URL(value);
  if (url.username || url.password || !['https:','http:'].includes(url.protocol)
    || url.hostname.endsWith('.') || url.hash || url.search || url.pathname!=='/') throw new Error('Unsupported origin');
  const loopback=['127.0.0.1','[::1]'].includes(url.hostname);
  if (url.protocol==='http:' && (!loopback || !url.port)) throw new Error('HTTP requires an explicit loopback port');
  if (!loopback && isIP(url.hostname.replace(/^\[|\]$/g,'')) && !publicAddress(url.hostname.replace(/^\[|\]$/g,''))) throw new Error('Private target');
  return url.origin;
}

export function makePolicy(origins, routes, { remoteEnabled=false }={}) {
  if (!Array.isArray(origins) || !origins.length || origins.length>8 || !Array.isArray(routes) || routes.length>150) throw new Error('Invalid policy');
  const allowed=new Set(origins.map(originOf));
  if (allowed.size!==origins.length) throw new Error('Duplicate origins');
  if (!remoteEnabled && [...allowed].some(origin=>new URL(origin).protocol!=='http:')) throw new Error('Remote preview verification is pending');
  const rules=routes.map(route=>{
    if (!route || Object.keys(route).some(key=>!['origin','method','path'].includes(key)) || !allowed.has(originOf(route.origin))
      || !['GET','POST','PUT','PATCH','DELETE'].includes(route.method) || typeof route.path!=='string'
      || route.path.length>300 || !/^\/[A-Za-z0-9_./{}-]*$/.test(route.path) || route.path.includes('..') || route.path.includes('//')
      || (/[{}]/.test(route.path) && !/^[^{}]*\{fixture_id\}[^{}]*$/.test(route.path))) throw new Error('Invalid route');
    return {...route,origin:originOf(route.origin)};
  });
  let suspended=false;
  return {
    origins:[...allowed],
    suspend(){suspended=true},
    checkOrigin(value) { let url; try {url=new URL(value)} catch {return false} return !suspended && !url.username && !url.password && allowed.has(url.origin); },
    checkRequest(value,method) {
      let url;try {url=new URL(value)}catch{return false}
      if (!this.checkOrigin(value) || url.hash || url.search || /%|\\/.test(url.pathname)) return false;
      return rules.some(rule=>rule.origin===url.origin && rule.method===method && (rule.path===url.pathname
        || (rule.path.includes('{fixture_id}') && new RegExp('^'+rule.path.split('{fixture_id}').map(part=>part.replace(/[.*+?^$()|[\]\\]/g,'\\$&')).join('[A-Za-z0-9-]{1,128}')+'$').test(url.pathname))));
    },
  };
}

/** Revalidate every DNS answer and pin the actual TCP connection to one checked IP. */
export async function checkedDestination(host,port,policy,lookup) {
  const authority=host.includes(':')?`[${host}]:${port}`:`${host}:${port}`;
  const loopback=['127.0.0.1','::1'].includes(host);
  if (![...policy.origins].some(origin=>{const url=new URL(origin);return url.hostname.replace(/^\[|\]$/g,'')===host && Number(url.port||(url.protocol==='https:'?443:80))===port})) throw new Error('Unapproved destination');
  const addresses=isIP(host)?[{address:host,family:isIP(host)}]:await lookup(host,{all:true,verbatim:true});
  if (!addresses.length || addresses.some(({address})=>!publicAddress(address) && !(loopback && address===host))) throw new Error('Blocked DNS destination');
  return addresses[0];
}
