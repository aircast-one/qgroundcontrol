import { useMemo } from 'react';
import { Dialog } from '@base-ui/react';
import { AppCommands } from '@/bridge/BridgeCommands';
import { type Json, jsonObjects, optBoolean, optJSONArray, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import { Button, FilterChip, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { AircastMark } from '@/ui/AircastMark';
import { useDesktopLayout } from '@/ui/DesktopLayout';
import { factFromControl } from '@/ui/ParameterForm';
import { sentenceCase } from '@/ui/SettingsScreen';
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

const Question = ({ title, options, chosen, onPick }: { title: string; options: readonly string[]; chosen: number; onPick: (index: number) => void }) => (
  <div role="group" aria-label={title} className="flex w-full flex-col gap-2">
    <Text typography="titleSmall">{title}</Text>
    <div className="flex flex-wrap gap-2">
      {options.map((label, index) => (
        <FilterChip key={label} selected={index === chosen} onClick={() => onPick(index)} label={label} />
      ))}
    </div>
  </div>
);

const FirstRunUnits = () => {
  const system = Math.trunc(useQgcDouble(UNIT_SYSTEM_PATH));
  return <Question title="Units" options={FIRST_RUN_SYSTEMS} chosen={system} onPick={(index) => void Qgc.invoke(SET_UNIT_SYSTEM, index)} />;
};

const VehicleQuestion = ({ fact }: { fact: Fact }) => (
  <Question title="Vehicle type" options={fact.enumStrings.map(sentenceCase)} chosen={fact.enumIndex} onPick={(index) => void Qgc.set(`${fact.path}.enumIndex`, index)} />
);

export const WELCOME_NOTE = 'Two questions so numbers and controls match your drone.';

const continueButton = (): HTMLElement | null => document.querySelector<HTMLElement>('[data-first-run-continue] button');

export const FirstRunDialog = () => {
  const view = useQgcPath(FIRST_RUN_PATH);
  const prompt = useMemo(() => firstRun(view), [view]);
  const desktop = useDesktopLayout();
  if (prompt === null) return null;
  const close = () => void AppCommands.markFirstRunShown();
  const questions = (
    <>
      <FirstRunUnits />
      {firstRunQuestions(prompt.preferences).map((fact) => (
        <VehicleQuestion key={fact.path} fact={fact} />
      ))}
    </>
  );
  const heading = (
    <div className="flex w-full flex-col items-center gap-3 text-center">
      <AircastMark size={desktop ? 64 : 88} />
      <Dialog.Title className="m-0 text-headline-small">{`Welcome to ${APP_NAME}`}</Dialog.Title>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} textAlign="center">
        {WELCOME_NOTE}
      </Text>
    </div>
  );

  return (
    <Dialog.Root open onOpenChange={(open) => !open && close()}>
      <Dialog.Portal>
        {desktop ? (
          <>
            <Dialog.Backdrop className="fixed inset-0 z-50 bg-black/50" />
            <Dialog.Popup
              aria-label={prompt.title}
              initialFocus={() => continueButton()}
              className="fixed top-1/2 left-1/2 z-50 flex max-h-[calc(100vh-48px)] w-[min(620px,calc(100vw-32px))] -translate-x-1/2 -translate-y-1/2 flex-col gap-6 overflow-auto rounded-shape-xl bg-surface-container-high p-8 text-on-surface shadow-2xl outline-none"
            >
              {heading}
              <div className="flex w-full flex-col gap-5">{questions}</div>
              <div className="flex w-full justify-end" data-first-run-continue>
                <Button onClick={close}>Continue</Button>
              </div>
            </Dialog.Popup>
          </>
        ) : (
          <Dialog.Popup aria-label={prompt.title} className="fixed inset-0 z-50 flex flex-col bg-surface text-on-surface outline-none">
            <div className="min-h-0 w-full flex-1 overflow-auto">
              <div className="mx-auto flex w-full flex-col gap-6 px-6 pt-12 pb-4" style={{ maxWidth: FIRST_RUN_WIDTH }}>
                {heading}
                {questions}
              </div>
            </div>
            <div className="w-full border-t border-outline-variant pb-[env(safe-area-inset-bottom)]">
              <div className="mx-auto flex w-full justify-end px-6 py-3" style={{ maxWidth: FIRST_RUN_WIDTH }}>
                <Button onClick={close}>Continue</Button>
              </div>
            </div>
          </Dialog.Popup>
        )}
      </Dialog.Portal>
    </Dialog.Root>
  );
};
