import { execFileSync } from 'node:child_process';
import { connect, type Socket } from 'node:net';

const IMAGE = 'ghcr.io/aircast-one/aircast-sitl:v0.0.12-95148b0';
const NAME = 'qgc-web-e2e-sitl';
const WS_PORT = 18790;
const TCP_PORT = 18791;
const STX = 0xfd;
const SIGNED = 0x01;
const HEARTBEAT = 0;
const RC_CHANNELS = 65;
const RC_CHANNELS_LENGTH = 42;
const VEHICLE = 1;
const RECONNECT_MS = 500;

export const SITL_WS = `ws://localhost:${WS_PORT}/ws/mavlink`;

type Frame = { message: number; system: number; payload: Buffer };

const frames = (bytes: Buffer, found: readonly Frame[] = []): { rest: Buffer; found: readonly Frame[] } => {
  const start = bytes.indexOf(STX);
  if (start < 0) return { rest: Buffer.alloc(0), found };
  const at = bytes.subarray(start);
  if (at.length < 10) return { rest: at, found };
  const length = at[1] ?? 0;
  const size = 12 + length + ((at[2] ?? 0) & SIGNED ? 13 : 0);
  if (at.length < size) return { rest: at, found };
  const message = (at[7] ?? 0) | ((at[8] ?? 0) << 8) | ((at[9] ?? 0) << 16);
  return frames(at.subarray(size), [...found, { message, system: at[5] ?? 0, payload: at.subarray(10, 10 + length) }]);
};

const channels = (payload: Buffer): number[] => {
  const full = Buffer.concat([payload, Buffer.alloc(RC_CHANNELS_LENGTH)]);
  return Array.from({ length: 4 }, (_, index) => full.readUInt16LE(4 + index * 2));
};

export type Sitl = { heard: { heartbeat: boolean; rc: readonly number[] }; stop: () => void };

const docker = (...args: string[]) => execFileSync('docker', args, { stdio: 'pipe' });

const removeContainer = () => {
  try {
    docker('rm', '-f', NAME);
  } catch {
    return;
  }
};

export const startSitl = (): Sitl => {
  removeContainer();
  docker('run', '-d', '--rm', '--name', NAME, '-p', `${WS_PORT}:8889`, '-p', `${TCP_PORT}:5760`, IMAGE);
  const state: { heard: Sitl['heard']; pending: Buffer; socket: Socket | null; stopped: boolean } = { heard: { heartbeat: false, rc: [] }, pending: Buffer.alloc(0), socket: null, stopped: false };
  const open = () => {
    if (state.stopped) return;
    const socket = connect(TCP_PORT, 'localhost');
    state.socket = socket;
    socket.on('data', (data: Buffer) => {
      const { rest, found } = frames(Buffer.concat([state.pending, data]));
      state.pending = rest;
      const mine = found.filter((frame) => frame.system === VEHICLE);
      const rc = mine.filter((frame) => frame.message === RC_CHANNELS).at(-1);
      state.heard = {
        heartbeat: state.heard.heartbeat || mine.some((frame) => frame.message === HEARTBEAT),
        rc: rc ? channels(rc.payload) : state.heard.rc,
      };
    });
    socket.on('error', () => socket.destroy());
    socket.on('close', () => setTimeout(open, RECONNECT_MS));
  };
  open();
  return {
    get heard() {
      return state.heard;
    },
    stop: () => {
      state.stopped = true;
      state.socket?.destroy();
      removeContainer();
    },
  };
};
