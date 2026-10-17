import { useMemo, useState } from 'react';
import { type Json, optBoolean, optJSONObject, optText } from '@/bridge/Json';
import { type Fact, makeFact } from '@/bridge/Qgc';
import { AlertDialog, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { SpeedSectionRow, speedSectionOf } from '@/ui/ItemEditor';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';

export const PLAN_DEFAULT_KEYS: readonly string[] = ['altitude', 'cruise', 'hover', 'ascent', 'descent'];

const SPEEDS_SET_BY_FLIGHT_SPEED: ReadonlySet<string> = new Set(['cruise', 'hover']);

export const planDefaults = (view: Json | null): readonly Fact[] => {
  const served = optJSONObject(view, 'defaults');
  if (served === null) return [];
  const flightSpeedSpecified = optBoolean(optJSONObject(served, 'flightSpeed'), 'specified');
  return PLAN_DEFAULT_KEYS.flatMap((key) => {
    const control = optJSONObject(served, key);
    const fact = control === null ? null : factFromControl(control);
    if (fact === null) return [];
    return [flightSpeedSpecified && SPEEDS_SET_BY_FLIGHT_SPEED.has(key) ? makeFact({ ...fact, enabled: false, disabledReason: 'The mission flight speed is used instead' }) : fact];
  });
};

export const planDefaultsNote = (view: Json | null, facts: readonly Fact[]): string =>
  facts.length === 0 ? "This core does not report the plan's defaults." : optText(optJSONObject(view, 'defaults'), 'speedNote');

export const PlanDefaultsDialog = ({ view, onDismiss }: { view: Json | null; onDismiss: () => void }) => {
  const facts = useMemo(() => planDefaults(view), [view]);
  const altitude = facts.filter((it) => it.path.endsWith('.defaultMissionItemAltitude'));
  const speeds = facts.filter((it) => !it.path.endsWith('.defaultMissionItemAltitude'));
  const flightSpeed = useMemo(() => speedSectionOf(optJSONObject(optJSONObject(view, 'defaults'), 'flightSpeed')), [view]);
  const [refusal, setRefusal] = useState<string | null>(null);
  const note = planDefaultsNote(view, facts);

  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Plan defaults"
      text={
        <div className="flex flex-col gap-1">
          {note.trim() !== '' ? <Text typography="bodySmall">{note}</Text> : null}
          {altitude.map((fact) => (
            <FactRow key={fact.path} fact={fact} />
          ))}
          {flightSpeed !== null ? (
            <SpeedSectionRow
              speed={flightSpeed}
              withSlider
              onWrite={(written) =>
                void (async () => {
                  setRefusal(await written());
                })()
              }
            />
          ) : null}
          {refusal !== null ? (
            <Text typography="bodySmall" color={md('error')}>
              {refusal}
            </Text>
          ) : null}
          {speeds.map((fact) => (
            <FactRow key={fact.path} fact={fact} />
          ))}
        </div>
      }
      confirmButton={<TextButton onClick={onDismiss}>Done</TextButton>}
    />
  );
};
