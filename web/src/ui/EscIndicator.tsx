import { useState } from 'react';
import { createPortal } from 'react-dom';
import { type Json, isNull, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcDerived } from '@/bridge/QgcState';
import { IconButton, type SheetAnchor, Text, anchorPlacement } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';

export const ESC_PATH = 'view.escs';

export type Level = 'normal' | 'elevated' | 'critical';

export const level = (json: Json, key: string): Level => {
  const text = optText(json, key);
  return text === 'critical' || text === 'elevated' ? text : 'normal';
};

export type EscMotor = { title: string; motor: number | null; level: Level; reasons: readonly string[]; rows: readonly (readonly [string, string])[] };

export type EscSummary = {
  onlineCount: number;
  healthy: boolean;
  healthText: string;
  healthyMotorsText: string;
  totalErrors: number;
  hints: readonly string[];
  motors: readonly EscMotor[];
};

export const escSummary = (view: Json | null): EscSummary | null =>
  view !== null && optBoolean(view, 'shown')
    ? {
        onlineCount: optInt(view, 'onlineCount'),
        healthy: optBoolean(view, 'healthy'),
        healthText: optText(view, 'healthText'),
        healthyMotorsText: optText(view, 'healthyMotorsText'),
        totalErrors: optInt(view, 'totalErrors'),
        hints: jsonStrings(optJSONArray(view, 'hints')),
        motors: jsonObjects(optJSONArray(view, 'motors')).map((motor) => ({
          title: optText(motor, 'title'),
          motor: isNull(motor, 'motor') ? null : optInt(motor, 'motor'),
          level: level(motor, 'level'),
          reasons: jsonStrings(optJSONArray(motor, 'reasons')),
          rows: [
            ['RPM', optText(motor, 'rpm')],
            ['Temp', optText(motor, 'temperature')],
            ['Voltage', optText(motor, 'voltage')],
            ['Current', optText(motor, 'current')],
            ['Errors', optText(motor, 'errors')],
          ],
        })),
      }
    : null;

export const escCellText = (summary: EscSummary): string => `ESC ${summary.onlineCount} ${summary.healthText}`;

export const splitFocus = (motors: readonly EscMotor[], focus: number | null): { focused: EscMotor | null; rest: readonly EscMotor[] } => {
  const focused = focus === null ? null : (motors.find((it) => it.motor === focus) ?? null);
  return { focused, rest: motors.filter((it) => it !== focused) };
};

const levelColour = (motorLevel: Level): string => (motorLevel === 'critical' ? md('error') : motorLevel === 'elevated' ? aircast('warning') : aircast('success'));

const MotorCard = ({ motor, highlighted }: { motor: EscMotor; highlighted: boolean }) => (
  <div className={`flex flex-col rounded-shape-sm px-2 py-1.5 ${highlighted ? 'bg-surface-container-highest' : ''}`}>
    <Text typography="titleSmall" color={levelColour(motor.level)}>
      {motor.title}
    </Text>
    {motor.reasons.map((reason) => (
      <Text key={reason} typography="bodySmall" color={levelColour(motor.level)}>
        {reason}
      </Text>
    ))}
    {motor.rows
      .filter(([, value]) => value.trim() !== '')
      .map(([label, value]) => (
        <Text key={label} typography="bodySmall">{`${label}  ${value}`}</Text>
      ))}
  </div>
);

const EscStatusContent = ({ summary, focus }: { summary: EscSummary; focus: number | null }) => {
  const { focused, rest } = splitFocus(summary.motors, focus);
  return (
    <div className="flex w-full flex-col gap-2">
      {focused !== null ? <MotorCard motor={focused} highlighted /> : null}
      <Text typography="bodyMedium">{`Healthy Motors  ${summary.healthyMotorsText}`}</Text>
      <Text typography="bodyMedium">{`Total Errors  ${summary.totalErrors}`}</Text>
      {summary.hints.map((hint) => (
        <Text key={hint} typography="bodySmall" color={summary.healthy ? aircast('warning') : md('error')}>
          {hint}
        </Text>
      ))}
      <div className="flex flex-col">
        {rest.map((motor) => (
          <MotorCard key={motor.title} motor={motor} highlighted={false} />
        ))}
      </div>
    </div>
  );
};

export const EscStatusSheet = ({ onDismissRequest }: { onDismissRequest: () => void }) => {
  const summary = useQgcDerived(ESC_PATH, escSummary);
  if (summary === null) return null;
  return (
    <AircastSheet onDismissRequest={onDismissRequest} label="ESC status">
      <div className="flex w-full flex-col gap-2 px-sheet pb-6">
        <Text typography="titleMedium">ESC Status Overview</Text>
        <div className="max-h-[420px] overflow-auto">
          <EscStatusContent summary={summary} focus={null} />
        </div>
      </div>
    </AircastSheet>
  );
};

export const EscStatusPanel = ({ anchor, focus, onClose }: { anchor: SheetAnchor; focus: number | null; onClose: () => void }) => {
  const summary = useQgcDerived(ESC_PATH, escSummary);
  if (summary === null) return null;
  return createPortal(
    <div
      role="dialog"
      aria-label="ESC status"
      className="fixed left-[var(--anchor-left)] top-[var(--anchor-top,auto)] bottom-[var(--anchor-bottom,auto)] z-40 flex max-h-[60vh] w-[320px] max-w-[calc(100vw-32px)] flex-col rounded-shape-md bg-surface-container text-on-surface shadow-lg"
      style={anchorPlacement(anchor, { width: window.innerWidth, height: window.innerHeight })}
    >
      <div className="flex items-center justify-between pl-4 pr-1">
        <Text typography="titleMedium">ESC status</Text>
        <IconButton onClick={onClose} label="Close ESC status">
          <Drawable name="ic_close" />
        </IconButton>
      </div>
      <div className="overflow-auto px-2 pb-3">
        <EscStatusContent summary={summary} focus={focus} />
      </div>
    </div>,
    document.body,
  );
};

export const EscIndicatorCell = () => {
  const summary = useQgcDerived(ESC_PATH, escSummary);
  const [open, setOpen] = useState(false);
  if (summary === null) return null;
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        <Text typography="labelMedium" color={summary.healthy ? aircast('success') : md('error')} maxLines={1}>
          {escCellText(summary)}
        </Text>
      </button>
      {open ? <EscStatusSheet onDismissRequest={() => setOpen(false)} /> : null}
    </>
  );
};
