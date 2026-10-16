import { useState } from 'react';
import { type Json, isNull, optBoolean, optDouble, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect, useTicker } from '@/kotlin/Compose';
import { format } from '@/kotlin/Std';
import { OutlinedButton, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { FactRow, settingsPagePath, settingsSections } from '@/ui/SettingsScreen';

export const GPS_RTK_VIEW = 'view.gpsRtk';
const RTK_POLL_MS = 1000;

export type RtkStatus = {
  active: boolean;
  valid: boolean;
  satellites: number | null;
  durationS: number | null;
  accuracyM: number | null;
  accuracyText: string | null;
  latitude: number | null;
  longitude: number | null;
  altitudeM: number | null;
};

export const RtkStatus = (fields: Omit<RtkStatus, 'accuracyText' | 'latitude' | 'longitude' | 'altitudeM'> & Partial<RtkStatus>): RtkStatus => ({
  accuracyText: null,
  latitude: null,
  longitude: null,
  altitudeM: null,
  ...fields,
});

export const RTK_SETTINGS_PAGE = 'RTK GPS';
const RTK_SETTINGS = 'settings.rtkSettings';
const RTK_AUTOCONNECT_GROUP = 'settings.autoConnectSettings';
const RTK_AUTOCONNECT = 'autoConnectRTKGPS';

export const basePositionWrites = (status: RtkStatus): readonly (readonly [string, number])[] | null => {
  if (!status.valid) return null;
  const named: readonly (readonly [string, number | null])[] = [
    ['fixedBasePositionLatitude', status.latitude],
    ['fixedBasePositionLongitude', status.longitude],
    ['fixedBasePositionAltitude', status.altitudeM],
    ['fixedBasePositionAccuracy', status.accuracyM],
  ];
  return named.every(([, value]) => value !== null) ? named.map(([name, value]) => [`${RTK_SETTINGS}.${name}`, value as number] as const) : null;
};

export const fixedBaseChosen = (facts: readonly Fact[]): boolean => {
  const value = facts.find((fact) => fact.name === 'useFixedBasePosition')?.value;
  return value === true || (typeof value === 'number' && Math.trunc(value) === 1) || value === '1';
};

const number = (json: Json, key: string): number | null => {
  if (isNull(json, key)) return null;
  const value = optDouble(json, key);
  return Number.isNaN(value) ? null : value;
};

export const rtkStatus = (view: Json | null): RtkStatus | null => {
  if (view === null || !optBoolean(view, 'connected')) return null;
  const satellites = number(view, 'numSatellites');
  const accuracyText = optText(view, 'currentAccuracyText');
  return RtkStatus({
    active: optBoolean(view, 'active'),
    valid: optBoolean(view, 'valid'),
    satellites: satellites === null ? null : Math.trunc(satellites),
    durationS: number(view, 'currentDuration'),
    accuracyM: number(view, 'currentAccuracy'),
    accuracyText: accuracyText === '' ? null : accuracyText,
    latitude: number(view, 'currentLatitude'),
    longitude: number(view, 'currentLongitude'),
    altitudeM: number(view, 'currentAltitude'),
  });
};

export const rtkRows = (status: RtkStatus): readonly (readonly [string, string])[] => [
  ['Satellites', status.satellites?.toString() ?? ''],
  ['Duration', `${format('%.0f', status.durationS ?? 0)} s`],
  ...(status.accuracyText !== null && (status.accuracyM ?? 0) > 0 ? [[status.valid ? 'Accuracy' : 'Current Accuracy', status.accuracyText] as const] : []),
];

export const rtkHeadline = (status: RtkStatus): string => (status.active ? 'Survey-in Active' : 'RTK Streaming');

export const useRememberRtkStatus = (): RtkStatus | null => {
  const [status, setStatus] = useState<RtkStatus | null>(null);
  useTicker(RTK_POLL_MS, [], async () => setStatus(rtkStatus(await Qgc.get(GPS_RTK_VIEW))));
  return status;
};

export const RtkSettingsSheetContent = ({ status }: { status: RtkStatus | null }) => (
  <div className="flex w-full flex-col gap-1.5 overflow-y-auto px-5 pb-6">
    <RtkSettingsSection status={status} />
  </div>
);

export const RtkIndicatorCell = ({ status }: { status: RtkStatus | null }) => {
  const [open, setOpen] = useState(false);
  if (status === null) return null;
  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        <Text typography="labelMedium">RTK</Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-1.5 overflow-y-auto px-5 pb-6">
            <Text typography="titleSmall">RTK GPS Status</Text>
            <Text typography="bodyMedium">{rtkHeadline(status)}</Text>
            {rtkRows(status).map(([label, value]) => (
              <Text key={label} typography="bodySmall">{`${label}  ${value}`}</Text>
            ))}
            <RtkSettingsSection status={status} />
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};

export const RtkSettingsSection = ({ status }: { status: RtkStatus | null }) => {
  const [reloads, setReloads] = useState(0);
  const [facts, setFacts] = useState<readonly Fact[]>([]);
  const [autoConnect, setAutoConnect] = useState<Fact | null>(null);
  const reload = () => setReloads((count) => count + 1);

  useLaunchedEffect([reloads], async (scope) => {
    const loaded = settingsSections(await Qgc.get(settingsPagePath(RTK_SETTINGS_PAGE))).flatMap((section) => section.blocks.flatMap((block) => block.facts));
    const auto = Qgc.fact(RTK_AUTOCONNECT_GROUP, { ...(await Qgc.get(`${RTK_AUTOCONNECT_GROUP}.${RTK_AUTOCONNECT}`)), name: RTK_AUTOCONNECT });
    if (!scope.isActive()) return;
    setFacts(loaded);
    setAutoConnect(auto);
  });

  const writes = status === null ? null : basePositionWrites(status);
  return (
    <>
      <Text typography="titleSmall">RTK GPS Settings</Text>
      {autoConnect !== null ? <FactRow fact={autoConnect} title="AutoConnect" onWrite={reload} /> : null}
      {facts.map((fact) => (
        <FactRow key={fact.path} fact={fact} onWrite={reload} />
      ))}
      {fixedBaseChosen(facts) ? (
        <div className="flex items-center">
          <Text className="flex-1">Current base position</Text>
          <OutlinedButton
            enabled={writes !== null}
            onClick={() =>
              void (async () => {
                await (writes ?? []).reduce<Promise<unknown>>((previous, [path, value]) => previous.then(() => Qgc.set(path, value)), Promise.resolve());
                reload();
              })()
            }
          >
            {writes !== null ? 'Save' : 'Not Yet Valid'}
          </OutlinedButton>
        </div>
      ) : null}
    </>
  );
};
