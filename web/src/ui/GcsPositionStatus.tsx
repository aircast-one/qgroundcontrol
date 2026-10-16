import { useState } from 'react';
import { type Json, optBoolean, optDouble, optJSONObject } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useTicker } from '@/kotlin/Compose';
import { format } from '@/kotlin/Std';
import { Text } from '@/m3';
import { SectionHeader } from '@/ui/SetupRow';

export const REMOTE_ID_GROUP = 'remoteIDSettings';
export const GCS_LOCATION_BLOCK = 'Ground Station Location';
const GCS_POSITION = 'positionManager.gcsPosition';
const GCS_ACCURACY = 'positionManager.gcsPositionHorizontalAccuracy';
const GCS_POLL_MS = 1000;

export const gcsPositionRows = (position: Json | null, accuracy: Json | null): readonly (readonly [string, string])[] | null => {
  const coordinate = optJSONObject(position, 'value') ?? position;
  if (coordinate === null || !optBoolean(coordinate, 'valid')) return null;
  const hdop = accuracy === null ? NaN : optDouble(accuracy, 'value', NaN);
  return [
    ['Latitude', format('%.7f', optDouble(coordinate, 'latitude'))],
    ['Longitude', format('%.7f', optDouble(coordinate, 'longitude'))],
    ['HDOP', hdop > 0 ? format('%.1f m', hdop) : 'N/A'],
  ];
};

export const GcsPositionStatus = () => {
  const [rows, setRows] = useState<readonly (readonly [string, string])[] | null>(null);
  useTicker(GCS_POLL_MS, [], async () => setRows(gcsPositionRows(await Qgc.get(GCS_POSITION), await Qgc.get(GCS_ACCURACY))));
  if (rows === null) return null;
  return (
    <>
      <SectionHeader text="GCS position" />
      <div className="flex w-full flex-col gap-1.5 px-5 py-2">
        {rows.map(([label, value]) => (
          <div key={label} className="flex w-full">
            <Text typography="bodyMedium" className="flex-1">
              {label}
            </Text>
            <Text typography="bodyMedium">{value}</Text>
          </div>
        ))}
      </div>
    </>
  );
};
