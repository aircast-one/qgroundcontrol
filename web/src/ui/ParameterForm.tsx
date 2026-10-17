import { type ReactNode, useRef, useState } from 'react';
import { type Fact, type FactSlider, Qgc, makeFact } from '@/bridge/Qgc';
import { type Json, isJson, isNull, jsonObjects, jsonStrings, opt, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { FilledTonalButton, OutlinedButton, Slider, Surface, Text } from '@/m3';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { coerceIn } from '@/kotlin/Std';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { sentenceCase } from '@/map/SentenceCase';
import { aircast, md, withAlpha } from '@/theme/Colors';
import { DIALOG_CONTROL, EscCalibrationDialog } from '@/ui/EscCalibration';
import { parameterPath } from '@/ui/ParametersScreen';
import { type PowerCalculator, PowerCalcDialog, powerCalculator } from '@/ui/PowerCalc';
import { FactRow } from '@/ui/SettingsScreen';
import { EmptyState, SectionHeader } from '@/ui/SetupRow';
import { setupPagePath } from '@/ui/SetupView';

export type ParameterRows = {
  title: string;
  facts: Fact[];
  note: string;
  calculators: ReadonlyMap<string, PowerCalculator>;
  image: string;
  keywords: string[];
};

export const ParameterRows = (fields: { title: string; facts: Fact[]; note: string; calculators?: ReadonlyMap<string, PowerCalculator>; image?: string; keywords?: string[] }): ParameterRows => ({
  calculators: new Map(),
  image: '',
  keywords: [],
  ...fields,
});

export const sectionMatches = (rows: ParameterRows, search: string): boolean => {
  const query = search.trim().toLowerCase();
  return query === '' || [rows.title, ...rows.keywords].some((it) => it.toLowerCase().includes(query));
};

export const factFromParameter = (name: string, json: Json, quantity: string | null = null): Fact | null =>
  optText(json, 'kind') === 'fact' && optText(json, 'name') !== '' ? makeFact({ ...Qgc.factAt(parameterPath(name, quantity), json), name }) : null;

export const readOnlyNote = (facts: readonly Fact[]): string | null => {
  const locked = facts.filter((it) => it.readOnly);
  if (locked.length === 0) return null;
  if (locked.length === facts.length) return 'This firmware reports all of these as read-only, so they are shown for reference.';
  return `This firmware reports ${locked.map((it) => it.name).join(', ')} as read-only, so they are shown but cannot be changed here.`;
};

const toLongOrNull = (text: string): number | null => (/^[+-]?\d+$/.test(text) ? Number(text) : null);

export const factFromControl = (control: Json): Fact | null => {
  const label = optText(control, 'label');
  const name = optText(control, 'name');
  if (label.trim() === '' && name.trim() === '') return null;
  const options = (optJSONArray(control, 'options') ?? []).map((it) => (isJson(it) ? it : null));
  const labels = options.map((it) => optText(it, 'label'));
  const bits = optText(control, 'control') === 'bitmask' ? jsonObjects(optJSONArray(control, 'bits')) : [];
  const bitEntries = bits.flatMap((bit) => {
    const raw = toLongOrNull(optText(bit, 'raw'));
    return raw === null ? [] : [[optText(bit, 'label'), raw] as const];
  });
  const slider = optJSONObject(control, 'slider');
  return makeFact({
    path: optText(control, 'path'),
    name,
    description: label,
    units: optText(control, 'units'),
    valueString: optText(control, 'valueString'),
    value: opt(control, 'value'),
    wholeNumbersOnly: optBoolean(control, 'wholeNumbersOnly'),
    enumStrings: labels,
    enumValues: options.map((it) => optText(it, 'raw')),
    enumIndex: labels.indexOf(optText(control, 'display')),
    bitmaskStrings: bitEntries.map(([bit]) => bit),
    bitmaskValues: bitEntries.map(([, raw]) => raw),
    controlKind: optText(control, 'control'),
    isBool: optText(control, 'control') === 'toggle',
    isString: optText(control, 'control') === 'text',
    readOnly: optBoolean(control, 'readOnly'),
    enabled: optBoolean(control, 'enabled', true),
    disabledReason: optText(control, 'disabledReason'),
    vehicleRebootRequired: optBoolean(control, 'vehicleRebootRequired'),
    minString: optText(control, 'minimumText'),
    maxString: optText(control, 'maximumText'),
    minIsDefaultForType: isNull(control, 'minimumText'),
    maxIsDefaultForType: isNull(control, 'maximumText'),
    defaultValueString: optText(control, 'defaultText'),
    qgcRebootRequired: optBoolean(control, 'applicationRestartRequired'),
    slider: slider === null ? null : factSlider(slider, optText(control, 'description')),
    warning: optBoolean(control, 'warning'),
    optional: optBoolean(control, 'optional'),
    firstEntryIsAll: optBoolean(control, 'firstEntryIsAll'),
    indent: optBoolean(control, 'indent'),
    smallFont: optBoolean(control, 'smallFont'),
    shortLabel: optText(control, 'shortLabel'),
    keywords: optText(control, 'keywords'),
    inverted: optBoolean(control, 'inverted'),
    rawChoice: optBoolean(control, 'rawChoice'),
    valueDetails: optText(control, 'valueDetails'),
    problem: optText(control, 'problem'),
    enumGroups: options.map((it) => optText(it, 'group')),
  });
};

export const readPage = async (page: string): Promise<ParameterRows[]> =>
  jsonObjects(optJSONArray(await Qgc.get(setupPagePath(page)), 'sections')).flatMap((section) => {
    const controls = jsonObjects(optJSONArray(section, 'controls'));
    const facts = controls.flatMap((control) => factFromControl(control) ?? []);
    if (facts.length === 0) return [];
    const note = [optText(section, 'note').trim() === '' ? null : optText(section, 'note'), readOnlyNote(facts)].filter((it): it is string => it !== null);
    const calculators = new Map(
      controls.flatMap((row) => {
        const calculator = powerCalculator(optJSONObject(row, 'calculator'));
        return calculator === null ? [] : [[optText(row, 'path'), calculator] as const];
      }),
    );
    const keywords = jsonStrings(optJSONArray(section, 'keywords'));
    return [ParameterRows({ title: optText(section, 'title'), facts, note: note.join(' '), calculators, image: optText(section, 'image'), keywords })];
  });

export const SETUP_PAGE_OPENED = 'setup.pageOpened';

export const EMPTY_PAGE_TEXTS: ReadonlyMap<string, string> = new Map([['Gimbal', 'Gimbal settings are not available for this firmware version.']]);

export const ParameterForm = ({ page, className, highlighted = new Set(), section = null }: { page: string; className?: string; highlighted?: ReadonlySet<string>; section?: string | null }) => {
  const [rows, setRows] = useState<ParameterRows[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [reloads, setReloads] = useState(0);
  const [calculating, setCalculating] = useState<PowerCalculator | null>(null);
  const [calibratingEscs, setCalibratingEscs] = useState(false);
  const reload = () => setReloads((it) => it + 1);

  useLaunchedEffect([page, section, reloads], async (scope) => {
    if (reloads === 0) await Qgc.invoke(SETUP_PAGE_OPENED, page);
    const read = (await readPage(page)).filter((it) => section === null || it.title === section);
    if (!scope.isActive()) return;
    setRows(read);
    setLoaded(true);
  });

  if (!loaded) return <Text className={`block p-4 ${className ?? ''}`}>Reading parameters from the vehicle.</Text>;

  if (rows.length === 0) return <EmptyState icon="ic_tune" title="Nothing to set here" text={EMPTY_PAGE_TEXTS.get(page) ?? 'This vehicle exposes none of these parameters.'} className={className} />;

  const factControl = (rowsSection: ParameterRows, fact: Fact): ReactNode => {
    if (fact.controlKind === DIALOG_CONTROL) {
      return (
        <OutlinedButton enabled={fact.enabled} onClick={() => setCalibratingEscs(true)} className="mx-5 my-2 self-start">
          {fact.title}
        </OutlinedButton>
      );
    }
    if (fact.controlKind === BUTTON_CONTROL) {
      return (
        <OutlinedButton
          enabled={fact.enabled}
          onClick={() => {
            void (async () => {
              await Qgc.set(fact.path, true);
              reload();
            })();
          }}
          className="mx-5 my-2 self-start"
        >
          {fact.title}
        </OutlinedButton>
      );
    }
    if (fact.controlKind === LABEL_CONTROL) {
      return (
        <Text
          typography={fact.smallFont ? 'bodySmall' : 'bodyMedium'}
          color={withAlpha(fact.warning ? md('error') : md('onSurfaceVariant'), fact.enabled ? 1 : DISABLED_LABEL_ALPHA)}
          className="block w-full px-5 py-2"
        >
          {fact.title}
        </Text>
      );
    }
    if (fact.slider !== null) return <FactSliderRow fact={fact} slider={fact.slider} onWrite={reload} />;
    const calculator = rowsSection.calculators.get(fact.path);
    return (
      <>
        <FactRow fact={fact} titleColor={highlighted.has(fact.name) ? aircast('warning') : undefined} onWrite={reload} />
        {calculator ? (
          <Surface className="mx-4 my-2" color={md('surfaceContainer')} shape="medium">
            <div className="flex items-center gap-3 p-4">
              <div className="flex min-w-0 flex-1 flex-col">
                <Text typography="bodyLarge">{sentenceCase(calculator.title)}</Text>
                <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                  {`Measure the ${calculator.measure} with a meter`}
                </Text>
              </div>
              <FilledTonalButton onClick={() => setCalculating(calculator)}>Calibrate</FilledTonalButton>
            </div>
          </Surface>
        ) : null}
      </>
    );
  };

  return (
    <>
      <div className={`h-full w-full overflow-auto ${className ?? ''}`}>
        {rows.flatMap((rowsSection) => [
          <div key={`section:${rowsSection.title}`}>
            <SectionHeader text={sentenceCase(rowsSection.title)} image={rowsSection.image} />
            {rowsSection.note.trim() !== '' ? (
              <Text typography="bodySmall" color={md('onSurfaceVariant')} className="block w-full px-5 pb-3">
                {rowsSection.note}
              </Text>
            ) : null}
          </div>,
          ...rowsSection.facts.map((fact) => (
            <div key={fact.path} className="flex flex-col" style={{ paddingLeft: fact.indent ? INDENT : 0 }}>
              {factControl(rowsSection, fact)}
            </div>
          )),
        ])}
      </div>
      {calibratingEscs ? <EscCalibrationDialog onClose={() => setCalibratingEscs(false)} /> : null}
      {calculating !== null ? (
        <PowerCalcDialog
          calculator={calculating}
          onDone={() => {
            setCalculating(null);
            reload();
          }}
        />
      ) : null}
    </>
  );
};

export const bitmaskRaw = (fact: Fact): number => {
  if (typeof fact.value === 'number') return Math.trunc(fact.value);
  const parsed = toDoubleOrNull(fact.valueString);
  return parsed === null ? 0 : Math.trunc(parsed);
};

const bitSet = (raw: number, bit: number): boolean => Number.isFinite(raw) && Number.isFinite(bit) && (BigInt(raw) & BigInt(Math.trunc(bit))) !== 0n;

export const bitmaskSummary = (fact: Fact): string => {
  const raw = bitmaskRaw(fact);
  const set = fact.bitmaskValues.flatMap((bit, index) => (bitSet(raw, bit) ? [index] : []));
  if (set.length === 0) return 'None';
  if (set.length === fact.bitmaskStrings.length) return 'All';
  return set.map((it) => fact.bitmaskStrings[it] ?? '').join(', ');
};

export const LABEL_CONTROL = 'label';
export const BUTTON_CONTROL = 'button';

const INDENT = 16;

const DISABLED_LABEL_ALPHA = 0.38;

export const KNOWN_CONTROL_KINDS: ReadonlySet<string> = new Set(['toggle', 'choice', 'bitmask', 'text', 'number', LABEL_CONTROL, DIALOG_CONTROL, BUTTON_CONTROL]);

export const controlIsUnderstood = (kind: string): boolean => kind.trim() === '' || KNOWN_CONTROL_KINDS.has(kind);

export const factSlider = (json: Json, hint: string): FactSlider | null => {
  const from = optDouble(json, 'from', NaN);
  if (Number.isNaN(from)) return null;
  const to = optDouble(json, 'to', NaN);
  if (Number.isNaN(to) || !(to > from)) return null;
  return { from, to, decimals: optInt(json, 'decimals', 2), hint };
};

const FactSliderRow = ({ fact, slider, onWrite }: { fact: Fact; slider: FactSlider; onWrite: () => void }) => {
  const held = typeof fact.value === 'number' ? fact.value : (toDoubleOrNull(fact.valueString) ?? slider.from);
  const [shown, setShownState] = useKeyedState([fact.path, held], () => coerceIn(held, slider.from, slider.to));
  const shownRef = useRef(shown);
  shownRef.current = shown;
  const setShown = (next: number) => {
    shownRef.current = next;
    setShownState(next);
  };
  return (
    <div className="flex w-full flex-col px-4 py-2">
      <div className="flex items-center">
        <Text typography="bodyLarge" className="min-w-0 flex-1">
          {sliderTitle(fact)}
        </Text>
        <Text typography="labelLarge" color={md('primary')}>
          {sliderValue(shown, slider.decimals, fact.units)}
        </Text>
      </div>
      <Slider
        value={shown}
        onValueChange={(it) => setShown(sliderSnapped(it, slider.decimals))}
        valueRange={[slider.from, slider.to]}
        enabled={fact.enabled && !fact.readOnly}
        label={sliderTitle(fact)}
        onValueChangeFinished={() => {
          void (async () => {
            await Qgc.writeRefusal(fact.path, shownRef.current);
            onWrite();
          })();
        }}
      />
      {slider.hint.trim() !== '' ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {slider.hint}
        </Text>
      ) : null}
    </div>
  );
};

export const sliderTitle = (fact: Fact): string => sentenceCase(fact.description.trim() === '' ? fact.heading : fact.description);

export const sliderSnapped = (value: number, decimals: number): number => Number(value.toFixed(coerceIn(decimals, 0, 6)));

export const sliderValue = (value: number, decimals: number, units: string): string =>
  [value.toFixed(coerceIn(decimals, 0, 6)), units].filter((it) => it.trim() !== '').join(' ');
