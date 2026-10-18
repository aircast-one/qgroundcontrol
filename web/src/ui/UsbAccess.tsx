import { requestSerialPort, serialAvailable } from '@/core/client';
import { Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const USB_RADIO_BLOCK = 'Radio on USB (EdgeTX script)';
export const CHROME_INSECURE_ORIGIN_FLAG = 'chrome://flags/#unsafely-treat-insecure-origin-as-secure';

export const usbBlockedNote = (canReachUsb: boolean, secure: boolean, origin: string): string | null =>
  canReachUsb
    ? null
    : secure
      ? 'This browser cannot reach USB devices. Use Chrome or Edge, or connect over a WebSocket.'
      : `Chrome lets only https and localhost pages use USB, and this page is ${origin}. In Chrome open ${CHROME_INSECURE_ORIGIN_FLAG}, add ${origin}, set it to Enabled and relaunch Chrome.`;

export const UsbAccess = () => {
  const note = usbBlockedNote(serialAvailable(), window.isSecureContext, window.location.origin);
  return note === null ? (
    <div>
      <TextButton onClick={() => void requestSerialPort().catch(() => undefined)}>Allow a USB port</TextButton>
    </div>
  ) : (
    <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block select-text">
      {note}
    </Text>
  );
};
