import type { OriginDto } from '../../lib/tauri/bindings';

export const originKey = (origin: OriginDto): string => `${origin.scheme}://${origin.host}:${origin.port}`;
export function uniqueOrigins(origins: OriginDto[]): OriginDto[] {
  return [...new Map(origins.map((origin) => [originKey(origin), origin])).values()];
}
export function previewUrl(value: string): URL {
  let url: URL;
  try { url = new URL(value); } catch { throw new Error('Use an HTTP or HTTPS test URL without credentials, query parameters or a fragment.'); }
  if (!['http:', 'https:'].includes(url.protocol) || url.username || url.password || url.search || url.hash) throw new Error('Use an HTTP or HTTPS test URL without credentials, query parameters or a fragment.');
  return url;
}
export function urlOrigin(url: URL): OriginDto { return { scheme: url.protocol.slice(0, -1), host: url.hostname, port: Number(url.port || (url.protocol === 'https:' ? 443 : 80)) }; }
export function environmentOrigins(preview: URL, additional: string): OriginDto[] {
  const origins = [urlOrigin(preview)];
  for (const [index, entry] of additional.split(/\r?\n/).entries()) {
    if (!entry.trim()) continue;
    let url: URL;
    try { url = previewUrl(entry.trim()); } catch { throw new Error(`Additional origin line ${index + 1}: enter an exact HTTP or HTTPS origin without credentials, query or fragment.`); }
    if (url.pathname !== '/') throw new Error(`Additional origin line ${index + 1}: enter only scheme, host and port, without a route path.`);
    origins.push(urlOrigin(url));
  }
  const unique = uniqueOrigins(origins);
  if (unique.length > 16) throw new Error('A preview supports at most 16 distinct exact origins, including the preview URL origin.');
  return unique;
}
