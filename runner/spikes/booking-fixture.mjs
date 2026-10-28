import { randomUUID } from 'node:crypto';
import { createServer } from 'node:http';
import { readFile, writeFile, rename } from 'node:fs/promises';

/** A real isolated round trip. This fixture has no payment, email or external service. */
export async function startBookingFixture(dataPath) {
  let records = [];
  try { records = JSON.parse(await readFile(dataPath, 'utf8')); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  const sessions = new Map();
  const server = createServer(async (req, res) => {
    res.setHeader('Cache-Control', 'no-store');
    res.setHeader('Content-Security-Policy', "default-src 'self'; script-src 'unsafe-inline'; connect-src 'self'; img-src 'none'; frame-src 'none'");
    const url = new URL(req.url, 'http://127.0.0.1');
    const role = sessions.get(req.headers.cookie?.split('session=')[1]?.split(';')[0]);
    const json = (status, value) => { res.writeHead(status, { 'Content-Type': 'application/json' }); res.end(JSON.stringify(value)); };
    try {
      if (req.method === 'GET' && url.pathname === '/.well-known/ownvero.json') {
        return json(200, { schemaVersion: 1, appId: 'booking-fixture', environmentId: 'loopback-test', revision: 'booking-fixture-v1', environmentType: 'test' });
      }
      if (req.method === 'POST' && url.pathname === '/session') {
        const chosen = url.searchParams.get('role');
        if (!['staff', 'customer', 'other-customer'].includes(chosen)) return json(400, { error: 'unknown test role' });
        const session = randomUUID();
        sessions.set(session, chosen);
        res.setHeader('Set-Cookie', `session=${session}; HttpOnly; SameSite=Strict; Path=/`);
        return json(200, { role: chosen });
      }
      if (req.method === 'POST' && url.pathname === '/bookings') {
        if (role !== 'customer') return json(403, { error: 'denied' });
        let body = '';
        for await (const chunk of req) { body += chunk; if (body.length > 2048) return json(413, { error: 'too large' }); }
        const input = JSON.parse(body);
        if (input.name !== 'Synthetic booking' || input.note !== 'Window seat') return json(400, { error: 'unexpected fixture' });
        const booking = { id: randomUUID(), name: input.name, note: input.note };
        records.push(booking);
        await writeFile(`${dataPath}.tmp`, JSON.stringify(records));
        await rename(`${dataPath}.tmp`, dataPath);
        return json(201, { id: booking.id });
      }
      if (req.method === 'GET' && /^\/bookings\/[0-9a-f-]{36}$/.test(url.pathname)) {
        if (role !== 'staff') return json(403, { error: 'denied' });
        // Read the committed file, not the same object used by the create request.
        const persisted = JSON.parse(await readFile(dataPath, 'utf8'));
        const record = persisted.find((item) => item.id === url.pathname.split('/').at(-1));
        return json(record ? 200 : 404, record || { error: 'not found' });
      }
      if (req.method === 'GET' && url.pathname === '/') {
        res.writeHead(200, { 'Content-Type': 'text/html; charset=utf-8' });
        return res.end(`<!doctype html><html lang="en"><title>Synthetic booking fixture</title><h1>Create a booking</h1>
          <form><label>Name<input name="name"></label><label>Note<input name="note"></label><button>Book</button></form>
          <p role="status"></p><script>document.querySelector('form').onsubmit=async(event)=>{event.preventDefault();
          const form=new FormData(event.target); const response=await fetch('/bookings',{method:'POST',headers:{'Content-Type':'application/json'},
          body:JSON.stringify(Object.fromEntries(form))}); const data=await response.json();
          document.querySelector('[role=status]').textContent=response.ok?'Booking saved':'Booking failed';};</script></html>`);
      }
      return json(404, { error: 'not found' });
    } catch { return json(500, { error: 'fixture operation failed' }); }
  });
  await new Promise((accept, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', accept); });
  return {
    origin: `http://127.0.0.1:${server.address().port}`,
    close: () => new Promise((accept, reject) => { server.closeAllConnections(); server.close((error) => error ? reject(error) : accept()); }),
  };
}
