import { useMemo } from 'react';
import { type Json, opt, optBoolean, optJSONObject, optText, parseJson } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { call } from '@/core/client';
import { stageFile, unstage } from '@/core/files';

export const DEVICE_CONNECT = 'device.connect';
export const DEVICE_DISCONNECT = 'device.disconnect';
const DEVICE_CALL = 'device.call';
const DEVICE_READ = 'device.read';
const DEVICE_UPLOAD = 'device.upload';
const DEVICE_QR = 'device.qr';
const DEVICE_HOME = 'aircraft';
const UPLOAD_STAGE = '/cache/device-upload';

export const deviceViewPath = (page: string): string => `view.device(${page})`;

export type DeviceReading = { readonly origin: string; readonly host: string; readonly answered: boolean; readonly view: Json | null };

export const deviceReading = (json: Json | null): DeviceReading => ({
  origin: optText(json, 'origin'),
  host: optText(json, 'host'),
  answered: optBoolean(json, 'answered'),
  view: optJSONObject(json, 'view'),
});

export const useDeviceReading = (page: string = DEVICE_HOME): DeviceReading => {
  const json = useQgcPath(deviceViewPath(page));
  return useMemo(() => deviceReading(json), [json]);
};

export const useDeviceView = <T,>(page: string, parse: (json: unknown) => T | null): T | null => {
  const { view } = useDeviceReading(page);
  return useMemo(() => (view === null ? null : parse(view)), [view]);
};

export const resolveDeviceUrl = (origin: string, path: string): string => (origin === '' ? path : new URL(path, origin).href);

export const deviceUrl = (path: string): string => resolveDeviceUrl(deviceReading(Qgc.path(deviceViewPath(DEVICE_HOME))).origin, path);

export type DeviceRefusal = { readonly message: string; readonly field: string };

export type DeviceMethod = 'POST' | 'PUT' | 'DELETE';

export const NO_ANSWER: DeviceRefusal = { message: 'The device did not answer.', field: '' };

export const deviceRefusal = (answer: Json | null): DeviceRefusal | null => {
  if (answer === null) return NO_ANSWER;
  if (optBoolean(answer, 'ok')) return null;
  const message = optText(answer, 'reason') || NO_ANSWER.message;
  const field = optText(answer, 'field');
  return message === NO_ANSWER.message && field === '' ? NO_ANSWER : { message, field };
};

export type DeviceAnswer = { readonly refusal: DeviceRefusal | null; readonly result: unknown };

export const deviceAnswer = (answer: Json | null): DeviceAnswer => ({ refusal: deviceRefusal(answer), result: opt(answer, 'result') });

export const deviceInvoke = async (path: string, ...args: unknown[]): Promise<Json | null> => {
  const reply = await call<unknown>('deviceInvoke', path, JSON.stringify(args));
  return typeof reply === 'string' ? parseJson(reply) : null;
};

export const deviceSend = async (method: DeviceMethod, path: string, body?: unknown): Promise<DeviceAnswer> => deviceAnswer(await deviceInvoke(DEVICE_CALL, method, path, body ?? null));

export const deviceCall = async (method: DeviceMethod, path: string, body?: unknown): Promise<DeviceRefusal | null> => (await deviceSend(method, path, body)).refusal;

export const deviceRead = async <T,>(path: string, parse: (json: unknown) => T | null): Promise<T | null> => {
  const answer = await deviceInvoke(DEVICE_READ, path);
  return optBoolean(answer, 'ok') ? parse(opt(answer, 'result')) : null;
};

export type DevicePart = { readonly name: string; readonly file?: File; readonly value?: string };

const stagedAt = (part: DevicePart, file: File): string => `${UPLOAD_STAGE}/${part.name}/${file.name}`;

export const deviceUpload = async (method: DeviceMethod, path: string, parts: readonly DevicePart[]): Promise<DeviceAnswer> => {
  const files = parts.flatMap((part) => (part.file === undefined ? [] : [[stagedAt(part, part.file), part.file] as const]));
  await Promise.all(files.map(([at, file]) => stageFile(at, file).catch(() => false)));
  const answer = await deviceInvoke(DEVICE_UPLOAD, method, path, parts.map((part) => (part.file === undefined ? { name: part.name, value: part.value ?? '' } : { name: part.name, file: stagedAt(part, part.file) })));
  await Promise.all(files.map(([at]) => unstage(at).catch(() => false)));
  return deviceAnswer(answer);
};

export const deviceQr = async (text: string): Promise<string | null> => {
  const answer = await deviceInvoke(DEVICE_QR, text);
  const png = optText(answer, 'result');
  return optBoolean(answer, 'ok') && png !== '' ? `data:image/png;base64,${png}` : null;
};
