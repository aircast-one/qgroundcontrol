import { useMemo, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { aircast, md } from '@/theme/Colors';
import { JOYSTICK_VIEW } from '@/ui/JoystickScreen';

export type JoystickDetail = { label: string; value: string; warn: boolean };

export type JoystickBadge = { heading: string; enabledText: string; warn: boolean; typeText: string; inputsText: string; details: readonly JoystickDetail[] };

export const JoystickBadge = (heading: string, enabledText: string, warn: boolean, typeText: string, inputsText: string, details: readonly JoystickDetail[] = []): JoystickBadge => ({
  heading,
  enabledText,
  warn,
  typeText,
  inputsText,
  details,
});

export const joystickBadge = (view: Json | null): JoystickBadge | null => {
  const it = optJSONObject(view, 'indicator');
  return it === null
    ? null
    : JoystickBadge(
        optText(it, 'heading'),
        optText(it, 'enabledText'),
        optBoolean(it, 'warn'),
        optText(it, 'typeText'),
        optText(it, 'inputsText'),
        jsonObjects(optJSONArray(it, 'details')).map((row) => ({ label: optText(row, 'label'), value: optText(row, 'value'), warn: optBoolean(row, 'warn') })),
      );
};

export const JoystickIndicatorCell = () => {
  const view = useQgcPath(JOYSTICK_VIEW);
  const badge = useMemo(() => joystickBadge(view), [view]);
  const [open, setOpen] = useState(false);
  if (badge === null) return null;
  const tint = badge.warn ? aircast('warning') : undefined;
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        <Text typography="labelMedium" color={tint} maxLines={1}>
          Joystick
        </Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-2 px-5 pb-6">
            <Text typography="titleMedium">{badge.heading}</Text>
            <Text typography="bodyMedium" color={tint}>{`Enabled:  ${badge.enabledText}`}</Text>
            <Text typography="bodyMedium">{`Type:  ${badge.typeText}`}</Text>
            <Text typography="bodyMedium">{`Inputs:  ${badge.inputsText}`}</Text>
            {badge.details.map((detail, index) => (
              <Text key={`${detail.label}-${index}`} typography="bodyMedium" color={detail.warn ? md('error') : undefined}>{`${detail.label}  ${detail.value}`}</Text>
            ))}
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};
