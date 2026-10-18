import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { AlertDialog, TextButton } from '@/m3';
import { REBOOT_VEHICLE } from '@/ui/CalibrationView';

export const hostNoticesPath = (acknowledgedThrough: number): string => `view.hostNotices(${acknowledgedThrough})`;

export type AppMessage = { title: string; text: string; action: string | null };

export const AppMessage = (fields: { title: string; text: string; action?: string | null }): AppMessage => ({ action: null, ...fields });

export type NoticeBatch = {
  through: number;
  destination: string | null;
  banners: readonly string[];
  unknownKinds: readonly string[];
  errorBanners: readonly string[];
  dialogs: readonly AppMessage[];
};

export const NoticeBatch = (fields: Omit<NoticeBatch, 'dialogs'> & { dialogs?: readonly AppMessage[] }): NoticeBatch => ({ dialogs: [], ...fields });

export const REBOOT_VEHICLE_ACTION = 'rebootVehicle';
export const OPEN_SETUP_ACTION = 'openSetup';
export const USE_RADIO_ACTION = 'useRadio';
export const USE_RADIO = 'usbRadio.use';

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

const distinct = <T,>(items: readonly T[]): T[] => items.filter((item, index) => items.indexOf(item) === index);

export const noticeBatch = (view: Json | null): NoticeBatch | null => {
  const unseen = optJSONArray(view, 'unseen');
  if (unseen === null) return null;
  const notices = jsonObjects(unseen).filter((it) => optInt(it, 'id', -1) >= 0);
  if (notices.length === 0) return null;
  return NoticeBatch({
    through: Math.max(...notices.map((it) => optInt(it, 'id'))),
    destination: blankToNull(optText(view, 'destination')),
    banners: jsonStrings(optJSONArray(view, 'banners')).filter((it) => it.trim() !== ''),
    unknownKinds: notices.filter((it) => !optBoolean(it, 'known', true)).map((it) => optText(it, 'kind')),
    errorBanners: distinct(notices.filter((it) => optText(it, 'kind') === VEHICLE_ERROR_KIND).map((it) => optText(it, 'banner')).filter((it) => it.trim() !== '')),
    dialogs: jsonObjects(optJSONArray(view, 'dialogs')).map((it) => AppMessage({ title: optText(it, 'title'), text: optText(it, 'text'), action: blankToNull(optText(it, 'action')) })),
  });
};

export const REPEAT_QUIET_MS = 30_000;
export const VEHICLE_ERROR_KIND = 'vehicleError';
export const RESET_ERROR_LEVEL_MESSAGES = 'vehicle.resetErrorLevelMessages';

export const ADDITIONAL_ERRORS = 'Additional errors received';

export const criticalBanner = (errors: readonly string[]): string | null => {
  const first = errors[0];
  if (first === undefined) return null;
  return errors.length > 1 ? `${first} · ${ADDITIONAL_ERRORS}` : first;
};

export const AppMessageDialog = ({ message, onOpenSetup = () => undefined, onDismiss }: { message: AppMessage; onOpenSetup?: () => void; onDismiss: () => void }) => (
  <AlertDialog
    onDismissRequest={onDismiss}
    title={message.title}
    text={message.text}
    confirmButton={
      <TextButton
        onClick={() => {
          if (message.action === REBOOT_VEHICLE_ACTION) void Qgc.invoke(REBOOT_VEHICLE);
          if (message.action === OPEN_SETUP_ACTION) onOpenSetup();
          if (message.action === USE_RADIO_ACTION) void Qgc.invoke(USE_RADIO);
          onDismiss();
        }}
      >
        {message.action === OPEN_SETUP_ACTION ? 'Open Setup' : message.action === USE_RADIO_ACTION ? 'Use' : 'OK'}
      </TextButton>
    }
    dismissButton={
      message.action === REBOOT_VEHICLE_ACTION ? (
        <TextButton onClick={onDismiss}>
          Cancel
        </TextButton>
      ) : message.action === OPEN_SETUP_ACTION ? (
        <TextButton onClick={onDismiss}>
          Later
        </TextButton>
      ) : message.action === USE_RADIO_ACTION ? (
        <TextButton onClick={onDismiss}>
          Not now
        </TextButton>
      ) : undefined
    }
  />
);

export const quietBanners = (banners: readonly string[], shownAt: ReadonlyMap<string, number>, now: number): string[] =>
  distinct(banners).filter((banner) => {
    const at = shownAt.get(banner);
    return !(at !== undefined && now - at < REPEAT_QUIET_MS);
  });
