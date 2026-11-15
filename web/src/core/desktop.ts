import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';

type EventListener = (path: string, json: string) => void;

const tile = async (mapType: string, x: number, y: number, zoom: number): Promise<Uint8Array | undefined> => {
  const bytes = new Uint8Array(await invoke<ArrayBuffer>('map_tile', { mapType, x, y, zoom }));
  return bytes.length > 0 ? bytes : undefined;
};

export const desktopCall = <T,>(name: string, args: unknown[]): Promise<T> =>
  (name === 'mapTile' ? tile(...(args as [string, number, number, number])) : invoke('core', { name, args })) as Promise<T>;

export const desktopEvents = (deliver: EventListener): Promise<unknown> =>
  listen<[string, string][]>('core-events', ({ payload }) => payload.forEach(([path, json]) => deliver(path, json)));
