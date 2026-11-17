import { useMemo } from 'react';
import { Dialog } from '@base-ui/react';
import { AppCommands } from '@/bridge/BridgeCommands';
import { type Json, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import { Button, FilterChip, Text } from '@/m3';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';
import { SET_UNIT_SYSTEM, UNIT_SYSTEM_LABELS, UNIT_SYSTEM_PATH } from '@/ui/UnitsSection';

export const FIRST_RUN_PATH = 'view.firstRun';

export type FirstRun = { title: string; preferences: readonly Fact[] };

export const firstRun = (view: Json | null): FirstRun | null =>
  view !== null && optBoolean(view, 'show')
    ? {
        title: optText(view, 'title'),
        preferences: jsonObjects(optJSONArray(view, 'vehiclePreferences')).flatMap((control) => factFromControl(control) ?? []),
      }
    : null;

const FIRST_RUN_WIDTH = 560;
const APP_NAME = 'Aircast';

export const FIRST_RUN_SYSTEMS: readonly string[] = UNIT_SYSTEM_LABELS.slice(0, 2);

export const VEHICLE_PREFERENCE = 'preferredVehicleClass';

export const firstRunQuestions = (preferences: readonly Fact[]): readonly Fact[] => preferences.filter((it) => it.name === VEHICLE_PREFERENCE);

const FirstRunUnits = () => {
  const system = Math.trunc(useQgcDouble(UNIT_SYSTEM_PATH));
  return (
    <div className="flex w-full flex-col gap-1 px-4 py-2">
      <Text typography="bodyLarge">Units</Text>
      <div className="flex gap-2">
        {FIRST_RUN_SYSTEMS.map((label, index) => (
          <FilterChip key={label} selected={index === system} onClick={() => void Qgc.invoke(SET_UNIT_SYSTEM, index)} label={label} />
        ))}
      </div>
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
        <Dialog.Popup aria-label={prompt.title} className="fixed inset-0 z-50 flex flex-col bg-surface text-on-surface outline-none">
          <div className="min-h-0 w-full flex-1 overflow-auto">
            <div className="mx-auto flex w-full flex-col gap-4 px-2 pt-12 pb-4" style={{ maxWidth: FIRST_RUN_WIDTH }}>
              <div className="flex w-full flex-col items-center gap-4">
                <div className="flex size-24 items-center justify-center rounded-full bg-primary-container text-on-primary-container">
                  <Drawable name="ic_flight" size={48} />
                </div>
                <Dialog.Title className="m-0 text-center text-headline-small">{`Welcome to ${APP_NAME}`}</Dialog.Title>
                <Text typography="bodyMedium" color={md('onSurfaceVariant')} textAlign="center">
                  {WELCOME_NOTE}
                </Text>
              </div>
              <FirstRunUnits />
              {firstRunQuestions(prompt.preferences).map((fact) => (
                <FactRow key={fact.path} fact={fact} title="Vehicle type" subtitle="" />
              ))}
            </div>
          </div>
          <div className="w-full border-t border-outline-variant pb-[env(safe-area-inset-bottom)]">
            <div className="mx-auto flex w-full justify-end px-6 py-3" style={{ maxWidth: FIRST_RUN_WIDTH }}>
              <Button onClick={close}>Continue</Button>
            </div>
          </div>
        </Dialog.Popup>
      </Dialog.Portal>
    </Dialog.Root>
  );
};
