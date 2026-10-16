import { useMemo, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { aircast, md } from '@/theme/Colors';

export const ESC_PATH = 'view.escs';

export type EscMotor = { title: string; healthy: boolean; rows: readonly (readonly [string, string])[] };

export type EscSummary = {
  onlineCount: number;
  healthy: boolean;
  healthText: string;
  healthyMotorsText: string;
  totalErrors: number;
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
        motors: jsonObjects(optJSONArray(view, 'motors')).map((motor) => ({
          title: optText(motor, 'title'),
          healthy: optBoolean(motor, 'healthy'),
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

export const EscIndicatorCell = () => {
  const view = useQgcPath(ESC_PATH);
  const summary = useMemo(() => escSummary(view), [view]);
  const [open, setOpen] = useState(false);
  if (summary === null) return null;
  const good = aircast('success');
  const bad = md('error');
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        <Text typography="labelMedium" color={summary.healthy ? good : bad} maxLines={1}>
          {escCellText(summary)}
        </Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-2 px-5 pb-6">
            <Text typography="titleMedium">ESC Status Overview</Text>
            <Text typography="bodyMedium">{`Healthy Motors  ${summary.healthyMotorsText}`}</Text>
            <Text typography="bodyMedium">{`Total Errors  ${summary.totalErrors}`}</Text>
            <div className="max-h-[420px] overflow-auto">
              {summary.motors.map((motor) => (
                <div key={motor.title} className="flex flex-col py-1.5">
                  <Text typography="titleSmall" color={motor.healthy ? good : bad}>
                    {motor.title}
                  </Text>
                  {motor.rows
                    .filter(([, value]) => value.trim() !== '')
                    .map(([label, value]) => (
                      <Text key={label} typography="bodySmall">{`${label}  ${value}`}</Text>
                    ))}
                </div>
              ))}
            </div>
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};
