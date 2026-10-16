import { useMemo, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { aircast, md } from '@/theme/Colors';

export const GPS_RESILIENCE_PATH = 'view.gpsResilience';

export type ResilienceMark = { shown: boolean; colour: string };

export type ResilienceSection = { title: string; rows: readonly (readonly [string, string])[] };

export type GpsResilience = { interference: ResilienceMark; authentication: ResilienceMark; sections: readonly ResilienceSection[] };

const mark = (json: Json, key: string): ResilienceMark => {
  const it = optJSONObject(json, key);
  return it === null ? { shown: false, colour: '' } : { shown: optBoolean(it, 'shown'), colour: optText(it, 'colour') };
};

export const gpsResilience = (view: Json | null): GpsResilience | null =>
  view !== null && optBoolean(view, 'shown')
    ? {
        interference: mark(view, 'interference'),
        authentication: mark(view, 'authentication'),
        sections: jsonObjects(optJSONArray(view, 'sections')).map((section) => ({
          title: optText(section, 'title'),
          rows: jsonObjects(optJSONArray(section, 'rows')).map((row) => [optText(row, 'label'), optText(row, 'text')] as const),
        })),
      }
    : null;

const markColour = (colour: string): string => {
  switch (colour) {
    case 'good':
      return aircast('success');
    case 'warning':
      return aircast('warning');
    case 'alert':
      return aircast('alert');
    case 'error':
      return md('error');
    default:
      return md('onSurfaceVariant');
  }
};

export const GpsResilienceCell = () => {
  const view = useQgcPath(GPS_RESILIENCE_PATH);
  const resilience = useMemo(() => gpsResilience(view), [view]);
  const [open, setOpen] = useState(false);
  if (resilience === null) return null;
  return (
    <>
      <button type="button" className="flex gap-1" onClick={() => setOpen(true)}>
        {resilience.interference.shown ? (
          <Text typography="labelMedium" color={markColour(resilience.interference.colour)}>
            RF
          </Text>
        ) : null}
        {resilience.authentication.shown ? (
          <Text typography="labelMedium" color={markColour(resilience.authentication.colour)}>
            AUTH
          </Text>
        ) : null}
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-1.5 px-5 pb-6">
            {resilience.sections.map((section, index) => (
              <div key={`${section.title}-${index}`} className="contents">
                <Text typography="titleSmall">{section.title}</Text>
                {section.rows.map(([label, text], row) => (
                  <Text key={`${label}-${row}`} typography="bodySmall">{`${label}  ${text}`}</Text>
                ))}
              </div>
            ))}
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};
