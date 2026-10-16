import { useMemo } from 'react';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';
import { FilterChip, Text } from '@/m3';
import { factFromControl } from '@/ui/ParameterForm';
import { FootNote } from '@/ui/SetupRow';
import { FactRuns } from '@/ui/SettingsScreen';

const UNITS_PATH = 'settings.unitsSettings';

export const UNIT_SYSTEM_CUSTOM = 2;

export const UNIT_SYSTEM_LABELS: readonly string[] = ['Metric', 'Imperial', 'Custom'];

export const PRESET_UNIT_FACTS: readonly string[] = ['horizontalDistanceUnits', 'verticalDistanceUnits', 'areaUnits', 'speedUnits', 'temperatureUnits'];

const GENERAL_SETTINGS = 'view.settings(General)';

export const unitFacts = (page: Json | null): readonly Fact[] => {
  const units = jsonObjects(optJSONArray(page, 'sections')).find((section) => optText(section, 'group') === 'unitsSettings');
  if (units === undefined) return [];
  return jsonObjects(optJSONArray(units, 'subsections')).flatMap((subsection) =>
    jsonObjects(optJSONArray(subsection, 'controls')).flatMap((control) => {
      const fact = factFromControl(control);
      return fact === null ? [] : [fact];
    }),
  );
};

export const unitRowsFor = (system: number, facts: readonly Fact[]): readonly Fact[] => {
  const custom = unitSystemLabel(system) === UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM];
  return facts.filter((fact) => fact.name !== 'customUnits' && fact.name !== 'weightUnits').filter((fact) => custom || !PRESET_UNIT_FACTS.includes(fact.name));
};

export const unitSystemLabel = (system: number): string => UNIT_SYSTEM_LABELS[system] ?? UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM] ?? '';

export const unitSystemNote = (system: number): string =>
  unitSystemLabel(system) === UNIT_SYSTEM_LABELS[UNIT_SYSTEM_CUSTOM]
    ? 'Each measurement is set on its own below.'
    : `Distance, area, speed and temperature all follow ${unitSystemLabel(system)}. ` + 'Choose Custom to set them one at a time.';

const UnitSystemRow = ({ system, onPick }: { system: number; onPick: (index: number) => void }) => {
  const chosen = unitSystemLabel(system);
  return (
    <div className="flex w-full flex-col gap-1 px-4 py-2">
      <Text typography="bodyLarge">Measurement system</Text>
      <div className="flex gap-2">
        {UNIT_SYSTEM_LABELS.map((label, index) => (
          <FilterChip key={label} selected={label === chosen} onClick={() => (label !== chosen ? onPick(index) : undefined)} label={label} />
        ))}
      </div>
    </div>
  );
};

export const UnitsSection = ({ className }: { className?: string }) => {
  const page = useQgcPath(GENERAL_SETTINGS);
  const facts = useMemo(() => unitFacts(page), [page]);
  const system = useQgcDouble(`${UNITS_PATH}.unitSystem`);
  if (Number.isNaN(system)) return null;
  const chosen = Math.trunc(system);
  return (
    <div className={`flex flex-col ${className ?? ''}`}>
      <UnitSystemRow system={chosen} onPick={(picked) => void Qgc.invoke(`${UNITS_PATH}.setUnitSystem`, picked)} />
      <FootNote text={unitSystemNote(chosen)} />
      <FactRuns facts={unitRowsFor(chosen, facts)} />
    </div>
  );
};
