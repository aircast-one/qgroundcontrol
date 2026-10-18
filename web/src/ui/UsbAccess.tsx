import { requestSerialPort, serialAvailable } from '@/core/client';
import { Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const USB_RADIO_BLOCK = 'Radio on USB (EdgeTX script)';
export const USB_RADIO_PORT = 'settings.autoConnectSettings.usbRadioPort';
export const CHROME_INSECURE_ORIGIN_FLAG = 'chrome://flags/#unsafely-treat-insecure-origin-as-secure';

export type UsbBlocked = { secureUrl: string | null; note: string };

export const usbBlocked = (canReachUsb: boolean, secure: boolean, href: string): UsbBlocked | null => {
  if (canReachUsb) return null;
  if (secure) return { secureUrl: null, note: 'This browser cannot reach USB devices. Use Chrome or Edge, or connect over a WebSocket.' };
  const page = new URL(href);
  return {
    secureUrl: `https://${page.host}${page.pathname}${page.search}${page.hash}`,
    note: `Chrome lets only https and localhost pages use USB. Open this page over https and accept the device's certificate once. Or in Chrome open ${CHROME_INSECURE_ORIGIN_FLAG}, add ${page.origin}, set it to Enabled and relaunch Chrome.`,
  };
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
      {blocked.secureUrl === null ? null : (
        <div>
          <TextButton onClick={() => window.location.assign(blocked.secureUrl ?? '')}>Open over https</TextButton>
        </div>
      )}
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block select-text">
        {blocked.note}
      </Text>
    </>
  );
};
