import { createServer } from 'node:http';
import { WebSocketServer } from 'ws';
import { tickFrames } from '../tlog.ts';

const PORT = 18760;
const TICK_MS = 250;
const HOME = { lat: 47.3977, lon: 8.5456 };
const VEHICLE_SYSTEM = 1;
const HEARTBEAT_ID = 0;

const heard = { gcsHeartbeats: 0 };

const gcsHeartbeats = (bytes: Buffer): number => {
  const count = (at: number, found: number): number => {
    if (at + 10 > bytes.length) return found;
    if (bytes[at] !== 0xfd) return count(at + 1, found);
    const length = bytes[at + 1] ?? 0;
    const system = bytes[at + 5];
    const message = (bytes[at + 7] ?? 0) | ((bytes[at + 8] ?? 0) << 8) | ((bytes[at + 9] ?? 0) << 16);
    return count(at + 12 + length, found + (message === HEARTBEAT_ID && system !== VEHICLE_SYSTEM ? 1 : 0));
  };
  return count(0, 0);
};

const http = createServer((request, response) => {
  response.writeHead(200, { 'Content-Type': 'application/json', 'Access-Control-Allow-Origin': '*' });
  response.end(JSON.stringify(request.url === '/stats' ? heard : { ok: true }));
});

new WebSocketServer({ server: http }).on('connection', (socket) => {
  const state = { tick: 0 };
  const timer = setInterval(() => {
    tickFrames(state.tick, HOME).map((frame) => socket.send(Buffer.from(frame)));
    state.tick = state.tick + 1;
  }, TICK_MS);
  socket.on('message', (data: Buffer) => {
    heard.gcsHeartbeats = heard.gcsHeartbeats + gcsHeartbeats(data);
  });
  socket.on('close', () => clearInterval(timer));
});

http.listen(PORT);
