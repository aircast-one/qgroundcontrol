import { useEffect, useState } from 'react';
import { isJson, optText } from '@/bridge/Json';
import { requestSerialPort, serialAvailable } from '@/core/client';
import { Button, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const USB_RADIO_BLOCK = 'Radio on USB (EdgeTX script)';
export const USB_RADIO_PORT = 'settings.autoConnectSettings.usbRadioPort';
export const OPEN_OVER_HTTPS = 'Open over https to use USB…';

const DEVICE_TLS = '/api/tls';
const PROBE_MS = 2000;

export type UsbBlocked = { secureUrl: string | null; note: string };

export const securePage = (href: string, origin: string | null): string => {
  const page = new URL(href);
  return `${origin === null ? `https://${page.host}` : new URL(origin).origin}${page.pathname}${page.search}${page.hash}`;
};

export const CERTIFICATE_WARNING = 'If the browser warns about the certificate, choose Advanced and continue.';

export const usbBlocked = (canReachUsb: boolean, secure: boolean, href: string): UsbBlocked | null => {
  if (canReachUsb) return null;
  if (secure) return { secureUrl: null, note: 'Use Chrome or Edge to connect a radio over USB.' };
  return { secureUrl: securePage(href, null), note: 'The radio needs a secure page.' };
};

export const secureOrigins = (answer: unknown): string[] => {
  const json = isJson(answer) ? answer : null;
  return [optText(json, 'origin'), optText(json, 'fallback')].filter((origin) => origin.startsWith('https://'));
};

export const firstAnswered = (origins: readonly string[], answered: readonly boolean[]): string | null => origins.find((_, at) => answered[at]) ?? null;

export const answers = (origin: string): Promise<boolean> =>
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
  return firstAnswered(origins, await Promise.all(origins.map(answers)));
};

export const openOverHttps = async (): Promise<void> => {
  window.location.assign(securePage(window.location.href, await trustedOrigin()));
};

const OpenOverHttps = ({ note }: { note: string }) => {
  const [opening, setOpening] = useState(false);
  const [trusted, setTrusted] = useState<boolean | null>(null);
  useEffect(() => {
    void trustedOrigin().then((origin) => setTrusted(origin !== null));
  }, []);
  const open = async () => {
    setOpening(true);
    await openOverHttps();
  };
  return (
    <div className="flex flex-wrap items-center gap-3 px-4 py-3">
      <Button enabled={!opening} onClick={() => void open()}>
        Open over https
      </Button>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="select-text">
        {trusted === false ? `${note} ${CERTIFICATE_WARNING}` : note}
      </Text>
    </div>
  );
};

export const UsbAccess = () => {
  const blocked = usbBlocked(serialAvailable(), window.isSecureContext, window.location.href);
  if (blocked === null) {
    return (
      <div className="px-1 py-1">
        <TextButton onClick={() => void requestSerialPort().catch(() => undefined)}>Allow a USB port</TextButton>
      </div>
    );
  }
  if (blocked.secureUrl !== null) {
    return <OpenOverHttps note={blocked.note} />;
  }
  return (
    <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block px-4 pt-1 pb-3 select-text">
      {blocked.note}
    </Text>
  );
};
