import { call } from '@/core/client';

type RelayRequest = { readonly id: string; readonly call: 'get' | 'set' | 'invoke'; readonly path: string; readonly payload: string };

export const relayPort = (search: string): number | null => {
  const port = Number(new URLSearchParams(search).get('debug') ?? '');
  return Number.isInteger(port) && port > 0 && port < 65536 ? port : null;
};

const answer = (request: RelayRequest): Promise<string> =>
  (request.call === 'get' ? call<string>('get', request.path) : call<string>(request.call, request.path, request.payload))
    .catch((error: unknown) => JSON.stringify({ error: String(error) }));

const serve = async (relay: string): Promise<void> => {
  const polled = await fetch(`${relay}/poll`).catch(() => null);
  if (polled === null || !polled.ok) return;
  if (polled.status === 200) {
    const request = (await polled.json()) as RelayRequest;
    const body = await answer(request);
    await fetch(`${relay}/reply`, { method: 'POST', body: JSON.stringify({ id: request.id, body }) }).catch(() => null);
  }
  void serve(relay);
};

export const DebugRelay = {
  start(search: string): void {
    const port = relayPort(search);
    if (port !== null) void serve(`http://127.0.0.1:${port}`);
  },
};
