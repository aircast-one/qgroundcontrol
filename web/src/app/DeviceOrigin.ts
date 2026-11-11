import { isJson, optText } from '@/bridge/Json';

const DEVICE_TLS = '/api/tls';
const APP_MANIFEST = 'manifest.webmanifest';
const PROBE_MS = 2000;

export const secureOrigins = (answer: unknown): string[] => {
  const json = isJson(answer) ? answer : null;
  return [optText(json, 'origin'), optText(json, 'fallback')].filter((origin) => origin.startsWith('https://'));
};

export const firstAnswered = (origins: readonly string[], answered: readonly boolean[]): string | null => origins.find((_, at) => answered[at]) ?? null;

export const httpsOrigin = (text: string): string | null => {
  const url = URL.parse(text);
  return url !== null && url.protocol === 'https:' && url.origin === text ? text : null;
};

const deviceAnswers = (origin: string): Promise<boolean> =>
  fetch(new URL(DEVICE_TLS, origin), { mode: 'no-cors', cache: 'no-store', signal: AbortSignal.timeout(PROBE_MS) }).then(
    () => true,
    () => false,
  );

export const trustedOrigin = async (): Promise<string | null> => {
  const origins = secureOrigins(
    await fetch(DEVICE_TLS, { cache: 'no-store', signal: AbortSignal.timeout(PROBE_MS) })
      .then((response) => (response.ok ? response.json() : null))
      .catch(() => null),
  );
  return firstAnswered(origins, await Promise.all(origins.map(deviceAnswers)));
};

export const appAnswers = (app: string): Promise<boolean> =>
  fetch(new URL(APP_MANIFEST, `${app}/`), { cache: 'no-store', signal: AbortSignal.timeout(PROBE_MS) }).then(
    (response) => response.ok,
    () => false,
  );
