import { useState } from 'react';
import { isJson, optText } from '@/bridge/Json';
import { requestSerialPort, serialAvailable } from '@/core/client';
import { Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const USB_RADIO_BLOCK = 'Radio on USB (EdgeTX script)';
export const USB_RADIO_PORT = 'settings.autoConnectSettings.usbRadioPort';
export const CHROME_INSECURE_ORIGIN_FLAG = 'chrome://flags/#unsafely-treat-insecure-origin-as-secure';

const DEVICE_TLS = '/api/tls';
const PROBE_MS = 2000;

export type UsbBlocked = { secureUrl: string | null; note: string };

export const securePage = (href: string, origin: string | null): string => {
  const page = new URL(href);
  return `${origin === null ? `https://${page.host}` : new URL(origin).origin}${page.pathname}${page.search}${page.hash}`;
};

export const usbBlocked = (canReachUsb: boolean, secure: boolean, href: string): UsbBlocked | null => {
  if (canReachUsb) return null;
  if (secure) return { secureUrl: null, note: 'This browser cannot reach USB devices. Use Chrome or Edge, or connect over a WebSocket.' };
  const page = new URL(href);
  return {
    secureUrl: securePage(href, null),
    note: `Chrome lets only https and localhost pages use USB. Open this page over https. If Chrome warns about the device's certificate, accept it once. Or in Chrome open ${CHROME_INSECURE_ORIGIN_FLAG}, add ${page.origin}, set it to Enabled and relaunch Chrome.`,
  };
};

export const secureOrigins = (answer: unknown): string[] => {
  const json = isJson(answer) ? answer : null;
  return [optText(json, 'origin'), optText(json, 'fallback')].filter((origin) => origin.startsWith('https://'));
};

export const firstAnswered = (origins: readonly string[], answered: readonly boolean[]): string | null => origins.find((_, at) => answered[at]) ?? null;

const answers = (origin: string): Promise<boolean> =>
  fetch(new URL(DEVICE_TLS, origin), { mode: 'no-cors', cache: 'no-store', signal: AbortSignal.timeout(PROBE_MS) }).then(
    () => true,
    () => false,
  );

const trustedOrigin = async (): Promise<string | null> => {
  const origins = secureOrigins(
    await fetch(DEVICE_TLS, { cache: 'no-store', signal: AbortSignal.timeout(PROBE_MS) })
      .then((response) => (response.ok ? response.json() : null))
      .catch(() => null),
  );
  return firstAnswered(origins, await Promise.all(origins.map(answers)));
};

const OpenOverHttps = () => {
  const [opening, setOpening] = useState(false);
  const open = async () => {
    setOpening(true);
    window.location.assign(securePage(window.location.href, await trustedOrigin()));
  };
  return (
    <div>
      <TextButton enabled={!opening} onClick={() => void open()}>
        Open over https
      </TextButton>
    </div>
  );
};

export const UsbAccess = () => {
  const blocked = usbBlocked(serialAvailable(), window.isSecureContext, window.location.href);
  if (blocked === null) {
    return (
      <div>
        <TextButton onClick={() => void requestSerialPort().catch(() => undefined)}>Allow a USB port</TextButton>
      </div>
    );
  }
  return (
    <>
      {blocked.secureUrl === null ? null : <OpenOverHttps />}
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block select-text">
        {blocked.note}
      </Text>
    </>
  );
};
