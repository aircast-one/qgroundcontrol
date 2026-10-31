import { createSocket, type RemoteInfo } from 'node:dgram';
import { mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync, openSync, writeSync, closeSync, existsSync } from 'node:fs';
import { execFileSync } from 'node:child_process';
import { createServer, request as forward, type IncomingMessage, type ServerResponse } from 'node:http';
import { createServer as createSecureServer } from 'node:https';
import { connect, createServer as createTcpServer } from 'node:net';
import type { Duplex } from 'node:stream';
import { tmpdir } from 'node:os';
import { dirname, join, normalize } from 'node:path';
import { WebSocketServer, type WebSocket } from 'ws';
import { tickFrames } from '../tlog.ts';

export const DEVICE_PORT = 18780;
export const SBS1_PORT = 30003;
export const UPLOAD_PORT = 18782;
export const UDP_VEHICLE_PORT = 18783;
export const TCP_VEHICLE_PORT = 18784;
export const DEVICE_TLS_PORT = 18785;
export const UPGRADING_PORT = 18786;
export const OFFLINE_PORT = 18787;
export const SILENT_PORT = 18788;
const SECURE_ORIGIN = `https://localhost:${DEVICE_TLS_PORT}`;
const VITE = Number(process.env.E2E_PORT ?? 8800);
const HOME = { lat: 47.3977, lon: 8.5456 };
const TICK_MS = 250;

const store = mkdtempSync(join(tmpdir(), 'aircastd-store-'));
const seen = { uploads: [] as string[], firmware: [] as number[], relays: [] as string[], gcsHeartbeats: 0, tcpHeartbeats: 0, tracks: [] as string[], secure: [] as string[] };
const tracker = { on: false };
const scenes = new Map<string, { x: number; y: number }>([['e2e', { x: 0.4, y: 0.4 }], ['lens', { x: 0.4, y: 0.4 }]]);

const body = (request: IncomingMessage): Promise<Buffer> =>
  new Promise((resolve) => {
    const chunks: Buffer[] = [];
    request.on('data', (chunk: Buffer) => chunks.push(chunk));
    request.on('end', () => resolve(Buffer.concat(chunks)));
  });

const json = (response: ServerResponse, status: number, value: unknown) => {
  response.writeHead(status, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' });
  response.end(JSON.stringify(value));
};

const stored = (path: string): string | null => {
  const name = normalize(decodeURIComponent(path.replace(/^\/fly\/files\/?/, ''))).replace(/^(\.\.(\/|$))+/, '');
  return name.split('/').some((part) => part.startsWith('.')) ? null : join(store, name);
};

const files = async (request: IncomingMessage, response: ServerResponse, url: URL) => {
  const path = stored(url.pathname);
  if (path === null) return json(response, 400, { error: 'file names cannot start with a dot' });
  if (request.method === 'GET') {
    if (!existsSync(path)) return json(response, 404, { error: 'no such file' });
    if (statSync(path).isDirectory()) {
      return json(
        response,
        200,
        readdirSync(path)
          .filter((name) => !name.startsWith('.'))
          .map((name) => {
            const info = statSync(join(path, name));
            return { name, size: info.size, dir: info.isDirectory(), modified: info.mtimeMs };
          }),
      );
    }
    response.writeHead(200, { 'Content-Type': 'application/octet-stream' });
    return response.end(readFileSync(path));
  }
  if (request.method === 'DELETE') {
    rmSync(path, { recursive: true, force: true });
    return response.writeHead(204).end();
  }
  const bytes = await body(request);
  mkdirSync(dirname(path), { recursive: true });
  if (request.method === 'PUT') writeFileSync(path, bytes);
  else {
    const offset = Number(url.searchParams.get('offset'));
    const handle = openSync(path, existsSync(path) ? 'r+' : 'w');
    writeSync(handle, bytes, 0, bytes.length, offset);
    closeSync(handle);
  }
  return response.writeHead(204).end();
};

const proxy = async (request: IncomingMessage, response: ServerResponse) => {
  const target = request.headers['aircast-target'];
  if (typeof target !== 'string' || !/^https?:\/\//.test(target)) return json(response, 400, { error: 'Aircast-Target must be an http or https address' });
  const answer = await fetch(target, {
    method: request.method,
    headers: { 'Content-Type': request.headers['content-type'] ?? 'application/octet-stream' },
    body: request.method === 'POST' ? new Uint8Array(await body(request)) : undefined,
  }).catch(() => null);
  if (answer === null) return json(response, 502, { error: `could not reach ${new URL(target).host}` });
  response.writeHead(answer.status, { 'Content-Type': answer.headers.get('content-type') ?? 'application/octet-stream' });
  response.end(Buffer.from(await answer.arrayBuffer()));
};

const firmware = async (request: IncomingMessage, response: ServerResponse) => {
  const image = await body(request);
  seen.firmware = [...seen.firmware, image.length];
  return image.toString().includes('"board_id"') ? json(response, 200, { status: 'flashed' }) : json(response, 400, { error: 'not an apj firmware image' });
};

const detections = (response: ServerResponse, camera: string) => {
  response.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', 'Access-Control-Allow-Origin': '*' });
  const timer = setInterval(() => {
    const scene = scenes.get(camera) ?? { x: 0.4, y: 0.4 };
    response.write(`data: ${JSON.stringify({ boxes: [{ label: 'car', conf: 0.9, x: scene.x, y: scene.y, w: 0.2, h: 0.2, target: tracker.on }], ageMs: 0 })}\n\n`);
  }, 100);
  response.on('close', () => clearInterval(timer));
};

const track = async (request: IncomingMessage, response: ServerResponse, url: URL) => {
  const sent = await body(request);
  seen.tracks = [...seen.tracks, `${request.method} ${url.pathname} ${sent.toString()}`];
  tracker.on = request.method === 'PUT';
  return json(response, 200, { status: tracker.on ? 'tracking' : 'stopped' });
};

const AIRCRAFT = {
  name: 'e2e-device',
  level: 2,
  headline: 'No camera yet',
  detail: 'Add the camera once and this device carries its picture from then on.',
  action: { label: 'Add a camera', href: '/streaming' },
  checks: [
    { label: 'Camera', value: 'not set up', level: 2, href: '/streaming' },
    { label: 'Telemetry', value: 'connected', level: 0 },
  ],
  pages: [
    { title: 'Ground station links', href: '/connect', icon: 'qr-code' },
    { title: 'Connection', href: '/network', icon: 'network', summary: 'Wi-Fi' },
  ],
  health: { memory: '120 MB of 415 MB', charts: [{ label: 'CPU', values: [3, 5, 4], max: 100, value: '4%', level: 3 }] },
};

const TELEMETRY = {
  status: { level: 1, text: 'no aircraft' },
  notices: [{ level: 3, text: 'No flight controller is attached. Plug the controller in and power it on.' }],
  endpoints: [{ proto: 'UDP', label: 'MAVLink', url: 'e2e-device:14550', hint: 'QGroundControl: Application Settings → Comm Links → Add → Type UDP.' }],
  settings: { endpoints: 'udps:0.0.0.0:14550', baud: '115200' },
  schemes: [],
};

const FLIGHTS = {
  flights: [
    {
      id: '20261029-205600',
      title: 'Thu 29 Oct, 20:56',
      summary: "Lasted 23h58m56s · didn't fly · 18:42 Uplink lost",
      badge: { level: 1, text: '2 events' },
      open: true,
      sections: [{ title: '', span: '', videos: [], lines: [{ index: 0, clock: '21:28:43', text: 'siyi stopped sending video', level: 1, alert: true, repeats: 'repeated ×31 until 21:33:43' }] }],
      csv: '/api/flights/20261029-205600.csv',
      json: '/api/flights/20261029-205600.json',
    },
    { id: '20261029-205000', title: 'Thu 29 Oct, 20:50', summary: "Lasted 6m6s · didn't fly", badge: { level: 3, text: 'nothing to report' }, sections: [], csv: '/api/flights/20261029-205000.csv', json: '/api/flights/20261029-205000.json' },
  ],
};

const VIEWS = { aircraft: AIRCRAFT, telemetry: TELEMETRY, flights: FLIGHTS };

const app = (request: IncomingMessage, response: ServerResponse) => {
  const path = (request.url ?? '/').replace(/^\/fly(?=\/)/, '');
  const upstream = forward({ host: 'localhost', port: VITE, path, method: request.method, headers: { ...request.headers, host: `localhost:${VITE}` } }, (answer) => {
    response.writeHead(answer.statusCode ?? 502, answer.headers);
    answer.pipe(response);
  });
  upstream.on('error', () => response.writeHead(502).end());
  request.pipe(upstream);
};

const device = (named: { origin: string }, secure: boolean) => (request: IncomingMessage, response: ServerResponse) => {
  const url = new URL(request.url ?? '/', `http://localhost:${DEVICE_PORT}`);
  if (secure) seen.secure = [...seen.secure, url.pathname];
  if (url.pathname === '/api/tls') return json(response, 200, named);
  if (url.pathname === '/fly/device.json') return json(response, 200, { name: 'e2e-device', cameras: [{ name: 'e2e', url: 'http://localhost:18889/e2e/whep' }] });
  const streamed = /^\/api\/streams\/(e2e|lens)\/detections\/stream$/.exec(url.pathname);
  if (streamed !== null) return detections(response, streamed[1]);
  if (url.pathname.startsWith('/api/streams/e2e/track')) return void track(request, response, url);
  const placed = /^\/scene\/(lens)$/.exec(url.pathname);
  if (placed !== null && request.method === 'PUT') {
    return void body(request).then((sent) => {
      scenes.set(placed[1], JSON.parse(sent.toString()) as { x: number; y: number });
      json(response, 200, scenes.get(placed[1]));
    });
  }
  const view = /^\/api\/views\/(aircraft|telemetry|flights)(\/events)?$/.exec(url.pathname);
  if (view !== null && view[2] === undefined) return json(response, 200, VIEWS[view[1] as keyof typeof VIEWS]);
  if (view !== null) {
    response.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', 'Access-Control-Allow-Origin': '*' });
    return void response.write(`data: ${JSON.stringify(VIEWS[view[1] as keyof typeof VIEWS])}\n\n`);
  }
  if (url.pathname === '/stats') return json(response, 200, { ...seen, store });
  if (url.pathname.startsWith('/fly/files/') || url.pathname === '/fly/files') return void files(request, response, url);
  if (url.pathname === '/fly/proxy') return void proxy(request, response);
  if (url.pathname === '/api/telemetry/firmware' && request.method === 'POST') return void firmware(request, response);
  if (url.pathname === '/') return void response.writeHead(302, { Location: '/fly/' }).end();
  return app(request, response);
};

const certificate = (): { key: Buffer; cert: Buffer } => {
  const dir = mkdtempSync(join(tmpdir(), 'aircastd-tls-'));
  execFileSync('openssl', ['req', '-x509', '-newkey', 'ec', '-pkeyopt', 'ec_paramgen_curve:prime256v1', '-nodes', '-days', '2', '-subj', '/CN=localhost', '-addext', 'subjectAltName=DNS:localhost', '-keyout', join(dir, 'key.pem'), '-out', join(dir, 'cert.pem')], { stdio: 'ignore' });
  return { key: readFileSync(join(dir, 'key.pem')), cert: readFileSync(join(dir, 'cert.pem')) };
};

const http = createServer(device({ origin: '' }, false));
const upgrading = createServer(device({ origin: SECURE_ORIGIN }, false));
const https = createSecureServer(certificate(), device({ origin: SECURE_ORIGIN }, true));
const offline = createServer(device({ origin: 'https://10.255.255.1' }, false));
const silent = createServer(device({ origin: '' }, false));

const pipeTcp = (socket: WebSocket, address: string) => {
  const [host, port] = [address.slice(0, address.lastIndexOf(':')).replace(/^\[|\]$/g, ''), Number(address.slice(address.lastIndexOf(':') + 1))];
  const tcp = connect({ host, port });
  tcp.on('data', (bytes) => socket.send(bytes));
  tcp.on('error', (error) => socket.close(4000, error.message.slice(0, 120)));
  tcp.on('end', () => socket.close(1000, 'the host closed the connection'));
  socket.on('message', (bytes: Buffer) => tcp.write(bytes));
  socket.on('close', () => tcp.destroy());
};

const pipeUdp = (socket: WebSocket, port: number, to: readonly string[]) => {
  const udp = createSocket('udp4');
  const peers = new Map(to.map((address) => [address, address]));
  udp.on('message', (bytes: Buffer, from: RemoteInfo) => {
    peers.set(`${from.address}:${from.port}`, `${from.address}:${from.port}`);
    socket.send(bytes);
  });
  udp.on('error', (error) => socket.close(4000, error.message.slice(0, 120)));
  udp.bind(port);
  socket.on('message', (bytes: Buffer) => [...peers.values()].forEach((peer) => udp.send(bytes, Number(peer.slice(peer.lastIndexOf(':') + 1)), peer.slice(0, peer.lastIndexOf(':')))));
  socket.on('close', () => udp.close());
};

const relay = new WebSocketServer({ noServer: true });
const mavlink = new WebSocketServer({ noServer: true });

const upgrade = (request: IncomingMessage, socket: Duplex, head: Buffer) => {
  const url = new URL(request.url ?? '/', `http://localhost:${DEVICE_PORT}`);
  if (url.pathname === '/fly/socket') {
    seen.relays = [...seen.relays, url.search];
    return relay.handleUpgrade(request, socket, head, (ws) => {
      const tcp = url.searchParams.get('tcp');
      const udp = url.searchParams.get('udp');
      if (tcp !== null) pipeTcp(ws, tcp);
      else if (udp !== null) pipeUdp(ws, Number(udp), url.searchParams.getAll('to'));
      else ws.close(4000, 'give one of tcp=host:port, tls=host:port or udp=port');
    });
  }
  if (url.pathname === '/fly/mavlink') {
    return mavlink.handleUpgrade(request, socket, head, (ws) => {
      if (request.headers.host?.endsWith(`:${SILENT_PORT}`)) return;
      const state = { tick: 0 };
      const timer = setInterval(() => {
        tickFrames(state.tick, HOME).map((frame) => ws.send(Buffer.from(frame)));
        state.tick = state.tick + 1;
      }, TICK_MS);
      ws.on('message', (data: Buffer) => {
        seen.gcsHeartbeats = seen.gcsHeartbeats + (data[0] === 0xfd && data[7] === 0 && data[8] === 0 && data[9] === 0 ? 1 : 0);
      });
      ws.on('close', () => clearInterval(timer));
    });
  }
  const upstream = connect({ host: 'localhost', port: VITE }, () => {
    upstream.write(`${request.method} ${request.url} HTTP/1.1\r\n${Object.entries(request.headers).map(([name, value]) => `${name}: ${String(value)}`).join('\r\n')}\r\n\r\n`);
    upstream.write(head);
    upstream.pipe(socket);
    socket.pipe(upstream);
  });
  upstream.on('error', () => socket.destroy());
  socket.on('error', () => upstream.destroy());
};
[http, upgrading, https, offline, silent].forEach((server) => server.on('upgrade', upgrade));

const sbs1 = (icao: string, lat: number, lon: number): string =>
  ['MSG', '3', '1', '1', icao, '1', '', '', '', '', '', '4000', '', '', String(lat), String(lon), '', '', '', '', '', '0'].join(',') + '\r\n';

createTcpServer((socket) => {
  const timer = setInterval(() => socket.write(sbs1('4CA2D6', HOME.lat + 0.01, HOME.lon + 0.01)), 500);
  socket.on('close', () => clearInterval(timer));
  socket.on('error', () => clearInterval(timer));
}).listen(SBS1_PORT);

createServer(async (request, response) => {
  const received = await body(request);
  seen.uploads = [...seen.uploads, received.toString('latin1')];
  response.writeHead(200, { 'Content-Type': 'text/plain' });
  response.end('uploaded');
}).listen(UPLOAD_PORT);

const vehicle = createSocket('udp4');
const listeners = new Map<string, RemoteInfo>();
vehicle.on('message', (_bytes: Buffer, from: RemoteInfo) => {
  listeners.set(`${from.address}:${from.port}`, from);
});
const udpTick = { tick: 0 };
setInterval(() => {
  tickFrames(udpTick.tick, HOME).forEach((frame) => [...listeners.values()].forEach((to) => vehicle.send(Buffer.from(frame), to.port, to.address)));
  udpTick.tick = udpTick.tick + 1;
}, TICK_MS);
vehicle.bind(UDP_VEHICLE_PORT);

const heartbeats = (bytes: Buffer): number =>
  bytes.reduce((count, byte, at) => count + (byte === 0xfd && bytes[at + 7] === 0 && bytes[at + 8] === 0 && bytes[at + 9] === 0 && bytes[at + 5] !== 1 ? 1 : 0), 0);

createTcpServer((socket) => {
  const state = { tick: 0 };
  const timer = setInterval(() => {
    tickFrames(state.tick, HOME).forEach((frame) => socket.write(Buffer.from(frame)));
    state.tick = state.tick + 1;
  }, TICK_MS);
  socket.on('data', (bytes: Buffer) => {
    seen.tcpHeartbeats = seen.tcpHeartbeats + heartbeats(bytes);
  });
  socket.on('close', () => clearInterval(timer));
  socket.on('error', () => clearInterval(timer));
}).listen(TCP_VEHICLE_PORT);

http.listen(DEVICE_PORT);
upgrading.listen(UPGRADING_PORT);
https.listen(DEVICE_TLS_PORT);
offline.listen(OFFLINE_PORT);
silent.listen(SILENT_PORT);
