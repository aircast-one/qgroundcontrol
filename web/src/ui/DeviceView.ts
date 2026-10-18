import { useEffect, useState } from 'react';
import { isJson, optText, parseJson } from '@/bridge/Json';

export const deviceViewEvents = (page: string): string => `/api/views/${page}/events`;

export const followDeviceView = <T,>(page: string, parse: (json: unknown) => T | null, onView: (view: T) => void): (() => void) => {
  const source = new EventSource(deviceViewEvents(page));
  source.onmessage = (event: MessageEvent<string>) => {
    const view = parse(parseJson(event.data));
    if (view !== null) onView(view);
  };
  return () => source.close();
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
  const response = await fetch(path, {
    method,
    headers: body === undefined ? { Accept: 'application/json' } : { Accept: 'application/json', 'Content-Type': 'application/json' },
    body: body === undefined ? undefined : JSON.stringify(body),
  }).catch(() => null);
  if (response === null) return NO_ANSWER;
  return response.ok ? null : refusalOf(response.status, await response.json().catch(() => null));
};

export const deviceRead = async <T,>(path: string, parse: (json: unknown) => T | null): Promise<T | null> => {
  const response = await fetch(path, { headers: { Accept: 'application/json' }, cache: 'no-store' }).catch(() => null);
  return response?.ok ? parse(await response.json().catch(() => null)) : null;
};
