const HEARTBEAT = { id: 0, crcExtra: 50 };
const GLOBAL_POSITION_INT = { id: 33, crcExtra: 104 };
const QUADROTOR = 2;
const AUTOPILOT_PX4 = 12;
const CUSTOM_MODE_ENABLED = 1;
const STATE_ACTIVE = 4;
const MAVLINK_VERSION = 3;
const START_US = 1_760_000_000_000_000n;

const crcX25 = (bytes: readonly number[]): number =>
  bytes.reduce((crc, byte) => {
    const mixed = (byte ^ crc) & 0xff;
    const folded = (mixed ^ (mixed << 4)) & 0xff;
    return ((crc >> 8) ^ (folded << 8) ^ (folded << 3) ^ (folded >> 4)) & 0xffff;
  }, 0xffff);

const le = (value: number, size: number): number[] => Array.from({ length: size }, (_, at) => Number((BigInt.asUintN(size * 8, BigInt(Math.trunc(value))) >> BigInt(at * 8)) & 0xffn));

const frame = (seq: number, message: { id: number; crcExtra: number }, payload: readonly number[]): number[] => {
  const header = [payload.length, 0, 0, seq & 0xff, 1, 1, ...le(message.id, 3)];
  const crc = crcX25([...header, ...payload, message.crcExtra]);
  return [0xfd, ...header, ...payload, crc & 0xff, crc >> 8];
};

const stamped = (us: bigint, bytes: readonly number[]): number[] => [...Array.from({ length: 8 }, (_, at) => Number((us >> BigInt((7 - at) * 8)) & 0xffn)), ...bytes];

export type Home = { lat: number; lon: number };

export const tickFrames = (tick: number, home: Home): readonly number[][] => {
  const heartbeat = tick % 4 === 0 ? [frame(tick * 2, HEARTBEAT, [...le(0, 4), QUADROTOR, AUTOPILOT_PX4, CUSTOM_MODE_ENABLED, STATE_ACTIVE, MAVLINK_VERSION])] : [];
  const position = [...le(tick * 250, 4), ...le((home.lat + tick * 0.00001) * 1e7, 4), ...le(home.lon * 1e7, 4), ...le(500_000, 4), ...le(20_000, 4), ...le(100, 2), ...le(0, 2), ...le(0, 2), ...le(0, 2)];
  return [...heartbeat, frame(tick * 2 + 1, GLOBAL_POSITION_INT, position)];
};

export const flightLog = (seconds: number, home: Home): Buffer =>
  Buffer.from(
    Array.from({ length: seconds * 4 }, (_, tick) => tickFrames(tick, home).flatMap((bytes) => stamped(START_US + BigInt(tick) * 250_000n, bytes))).flat(),
  );
