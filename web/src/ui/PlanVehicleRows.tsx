import { useState } from 'react';
import { type Json, optBoolean, optJSONObject, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { Text } from '@/m3';
import { useHasVehicle } from '@/ui/FlyState';
import { FactRow } from '@/ui/SettingsScreen';

const APP_SETTINGS = 'settings.appSettings';
const OFFLINE_CLASSES: readonly (readonly [string, string])[] = [
  ['offlineEditingFirmwareClass', 'Firmware'],
  ['offlineEditingVehicleClass', 'Vehicle'],
];

export const choosesPlanVehicle = (connected: boolean, plan: Json | null): boolean => !connected && !optBoolean(plan, 'hasMissionItems');

export const PlanVehicleRows = () => {
  const plan = useQgcPath('view.plan');
  const choosing = choosesPlanVehicle(useHasVehicle(), plan);
  const [reloads, setReloads] = useState(0);
  const [facts, setFacts] = useState<readonly (readonly [string, Fact])[]>([]);

  useLaunchedEffect([choosing, reloads], async (scope) => {
    const read = !choosing
      ? []
      : await Promise.all(OFFLINE_CLASSES.map(async ([name, label]) => [label, Qgc.fact(APP_SETTINGS, { ...(await Qgc.get(`${APP_SETTINGS}.${name}`)), name })] as const));
    if (scope.isActive()) setFacts(read);
  });

  if (choosing) {
    return (
      <>
        {facts.map(([label, fact]) => (
          <FactRow key={fact.path} fact={fact} title={label} onWrite={() => setReloads((it) => it + 1)} />
        ))}
      </>
    );
  }
  const vehicle = optJSONObject(plan, 'planningFor');
  return vehicle === null ? null : (
    <>
      {(
        [
          ['Firmware', optText(vehicle, 'firmware')],
          ['Vehicle', optText(vehicle, 'type')],
        ] as const
      )
        .filter(([, value]) => value.trim() !== '')
        .map(([label, value]) => (
          <div key={label} className="flex w-full px-5 py-1">
            <Text typography="bodyMedium" className="min-w-0 flex-1">
              {label}
            </Text>
            <Text typography="bodyMedium">{value}</Text>
          </div>
        ))}
    </>
  );
};
