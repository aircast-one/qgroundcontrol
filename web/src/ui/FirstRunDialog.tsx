import { useMemo } from 'react';
import { Dialog } from '@base-ui/react';
import { AppCommands } from '@/bridge/BridgeCommands';
import { type Json, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import { Button, FilterChip, Text } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow, FactRuns } from '@/ui/SettingsScreen';
import { SectionHeader } from '@/ui/SetupRow';
import { unitFacts } from '@/ui/UnitsSection';

export const FIRST_RUN_PATH = 'view.firstRun';

export type FirstRun = {
  title: string;
  vehicleHeading: string;
  vehicleDescription: string;
  preferences: readonly Fact[];
  unitsHeading: string;
  unitsDescription: string;
};

export const firstRun = (view: Json | null): FirstRun | null =>
  view !== null && optBoolean(view, 'show')
    ? {
        title: optText(view, 'title'),
        vehicleHeading: optText(view, 'vehicleHeading'),
        vehicleDescription: optText(view, 'vehicleDescription'),
        preferences: jsonObjects(optJSONArray(view, 'vehiclePreferences')).flatMap((control) => factFromControl(control) ?? []),
        unitsHeading: optText(view, 'unitsHeading'),
        unitsDescription: optText(view, 'unitsDescription'),
      }
    : null;

const FIRST_RUN_WIDTH = 560;
const APP_NAME = 'Aircast';

export const FIRST_RUN_SYSTEMS: readonly string[] = ['Metric System', 'Imperial System'];

const METERS = 1.0;

const FIRST_RUN_UNIT_VALUES: ReadonlyMap<string, readonly [number, number]> = new Map([
  ['horizontalDistanceUnits', [1, 0]],
  ['verticalDistanceUnits', [1, 0]],
  ['areaUnits', [1, 0]],
  ['speedUnits', [1, 0]],
  ['temperatureUnits', [0, 1]],
]);

export const firstRunUnitRows = (facts: readonly Fact[]): Fact[] => facts.filter((it) => FIRST_RUN_UNIT_VALUES.has(it.name));

export const firstRunSystemIndex = (horizontalDistanceUnits: number): number => (horizontalDistanceUnits === METERS ? 0 : 1);

export const firstRunSystemWrites = (metric: boolean, rows: readonly Fact[]): [string, number][] =>
  rows.flatMap((fact): [string, number][] => {
    const values = FIRST_RUN_UNIT_VALUES.get(fact.name);
    return values === undefined ? [] : [[fact.path, metric ? values[0] : values[1]]];
  });

const FirstRunUnits = () => {
  const page = useQgcPath('view.settings(General)');
  const rows = useMemo(() => firstRunUnitRows(unitFacts(page)), [page]);
  const horizontal = useQgcDouble(settingControl('settings.unitsSettings.horizontalDistanceUnits'));
  const chosen = firstRunSystemIndex(horizontal);
  return (
    <div className="flex w-full flex-col">
      <div className="flex w-full flex-col gap-1 px-4 py-2">
        <Text typography="bodyLarge">System of units</Text>
        <div className="flex gap-2">
          {FIRST_RUN_SYSTEMS.map((label, index) => (
            <FilterChip
              key={label}
              selected={index === chosen}
              onClick={() => void Promise.all(firstRunSystemWrites(index === 0, rows).map(([path, value]) => Qgc.set(path, value)))}
              label={label}
            />
          ))}
        </div>
      </div>
      <FactRuns facts={rows} />
    </div>
  );
};

export const WELCOME_NOTE = 'Two questions so numbers and controls match your drone.';

export const FirstRunDialog = () => {
  const view = useQgcPath(FIRST_RUN_PATH);
  const prompt = useMemo(() => firstRun(view), [view]);
  if (prompt === null) return null;
  const close = () => void AppCommands.markFirstRunShown();

  return (
    <Dialog.Root open onOpenChange={(open) => !open && close()}>
      <Dialog.Portal>
        <Dialog.Popup aria-label={prompt.title} className="fixed inset-0 z-50 overflow-auto bg-surface text-on-surface outline-none">
          <div className="mx-auto flex w-full flex-col gap-4 px-2 py-12" style={{ maxWidth: FIRST_RUN_WIDTH }}>
            <div className="flex w-full flex-col items-center gap-4">
              <div className="flex size-24 items-center justify-center rounded-full bg-primary-container text-on-primary-container">
                <Drawable name="ic_flight" size={48} />
              </div>
              <Dialog.Title className="m-0 text-center text-headline-small">{`Welcome to ${APP_NAME}`}</Dialog.Title>
              <Text typography="bodyMedium" color={md('onSurfaceVariant')} textAlign="center">
                {WELCOME_NOTE}
              </Text>
            </div>
            <SectionHeader text={sentenceCase(prompt.unitsHeading)} />
            <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="px-4">
              {prompt.unitsDescription}
            </Text>
            <FirstRunUnits />
            {prompt.preferences.length > 0 ? (
              <>
                <SectionHeader text={sentenceCase(prompt.vehicleHeading)} />
                <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="px-4">
                  {prompt.vehicleDescription}
                </Text>
                {prompt.preferences.map((fact) => (
                  <FactRow key={fact.path} fact={fact} />
                ))}
              </>
            ) : null}
            <div className="flex w-full justify-end px-4">
              <Button onClick={close}>Continue</Button>
            </div>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
};
