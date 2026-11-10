const DEVICE_SETTINGS = 'settings';
const KEEPALIVE_BYTES = 60_000;
export const DEVICE_RETRY_MS = 5_000;

type Send = (text: string) => Promise<boolean>;

const isSettings = (body: unknown): body is { text: string } => typeof body === 'object' && body !== null && typeof (body as { text?: unknown }).text === 'string';

export const readDeviceSettings = async (): Promise<string | null> => {
  const response = await fetch(DEVICE_SETTINGS, { cache: 'no-store' }).catch(() => null);
  if (!response?.ok || !response.headers.get('content-type')?.startsWith('application/json')) return null;
  const body: unknown = await response.json().catch(() => null);
  return isSettings(body) ? body.text : null;
};

export const sendDeviceSettings: Send = (text) => {
  const body = JSON.stringify({ text });
  return fetch(DEVICE_SETTINGS, { method: 'PUT', headers: { 'Content-Type': 'application/json' }, body, keepalive: body.length < KEEPALIVE_BYTES }).then(
    (response) => response.ok,
    () => false,
  );
};

export const latestWins = (send: Send, retryMs: number): ((text: string) => Promise<void>) => {
  const state: { next: string | null; busy: boolean } = { next: null, busy: false };
  const drain = async (): Promise<void> => {
    if (state.busy || state.next === null) return;
    const text = state.next;
    state.next = null;
    state.busy = true;
    const sent = await send(text);
    state.busy = false;
    if (!sent) state.next ??= text;
    if (sent || state.next !== text) return drain();
    setTimeout(() => void drain(), retryMs);
  };
  return (text) => {
    state.next = text;
    return drain();
  };
};
