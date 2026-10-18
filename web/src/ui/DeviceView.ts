import { useEffect, useState } from 'react';
import { isJson, optText, parseJson } from '@/bridge/Json';
import { mutableStateOf } from '@/kotlin/State';

export const deviceOrigin = mutableStateOf('');

export const resolveDeviceUrl = (origin: string, path: string): string => (origin === '' ? path : new URL(path, origin).href);

export const deviceUrl = (path: string): string => resolveDeviceUrl(deviceOrigin.value, path);

export const deviceViewEvents = (page: string): string => deviceUrl(`/api/views/${page}/events`);

export const eventData = (block: string): string | null => {
  const data = block
    .split('\n')
    .filter((line) => line.startsWith('data:'))
    .map((line) => line.slice(5).replace(/^ /, ''));
  return data.length === 0 || data.join('') === '' ? null : data.join('\n');
};

export const splitEvents = (buffer: string): { readonly events: readonly string[]; readonly rest: string } => {
  const blocks = buffer.replace(/\r\n/g, '\n').split('\n\n');
  return { events: blocks.slice(0, -1).flatMap((block) => eventData(block) ?? []), rest: blocks.at(-1) ?? '' };
};

const RECONNECT_MS = 2000;

const readEvents = async (url: string, signal: AbortSignal, onEvent: (data: string) => void): Promise<void> => {
  const response = await fetch(url, { headers: { Accept: 'text/event-stream' }, cache: 'no-store', signal });
  if (!response.ok || response.body === null) return;
  const reader = response.body.pipeThrough(new TextDecoderStream()).getReader();
  let buffer = '';
  while (!signal.aborted) {
    const { done, value } = await reader.read();
    if (done) return;
    const { events, rest } = splitEvents(buffer + value);
    buffer = rest;
    const latest = events.at(-1);
    if (latest !== undefined) onEvent(latest);
  }
};

const follow = (url: string, signal: AbortSignal, onEvent: (data: string) => void): void =>
  void readEvents(url, signal, onEvent)
    .catch(() => undefined)
    .then(() => {
      if (!signal.aborted) setTimeout(() => follow(url, signal, onEvent), RECONNECT_MS);
    });

export const followDeviceView = <T,>(page: string, parse: (json: unknown) => T | null, onView: (view: T) => void): (() => void) => {
  const stop = new AbortController();
  follow(deviceViewEvents(page), stop.signal, (data) => {
    const view = parse(parseJson(data));
    if (view !== null) onView(view);
  });
  return () => stop.abort();
};

export const useDeviceView = <T,>(page: string, parse: (json: unknown) => T | null): T | null => {
  const [view, setView] = useState<T | null>(null);
  useEffect(() => followDeviceView(page, parse, setView), [page]);
  return view;
};

export type DeviceRefusal = { readonly message: string; readonly field: string };

export type DeviceMethod = 'POST' | 'PUT' | 'PATCH' | 'DELETE';

export const refusalOf = (status: number, body: unknown): DeviceRefusal => {
  const json = isJson(body) ? body : null;
  return { message: optText(json, 'error') || `The device refused (${status}).`, field: optText(json, 'field') };
};

export const NO_ANSWER: DeviceRefusal = { message: 'The device did not answer.', field: '' };

export const deviceCall = async (method: DeviceMethod, path: string, body?: unknown): Promise<DeviceRefusal | null> => {
  const response = await fetch(deviceUrl(path), {
    method,
    headers: body === undefined ? { Accept: 'application/json' } : { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  }).catch(() => null);
  if (response === null) return NO_ANSWER;
  return response.ok ? null : refusalOf(response.status, await response.json().catch(() => null));
};

export const deviceRead = async <T,>(path: string, parse: (json: unknown) => T | null): Promise<T | null> => {
  const response = await fetch(deviceUrl(path), { headers: { Accept: 'application/json' }, cache: 'no-store' }).catch(() => null);
  return response?.ok ? parse(await response.json().catch(() => null)) : null;
};
