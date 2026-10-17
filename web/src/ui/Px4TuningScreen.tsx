import { useEffect, useRef, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { coerceAtLeast, coerceIn, roundToInt } from '@/kotlin/Std';
import { FilterChip, OutlinedButton, RadioButton, ScrollableTabRow, Slider, Tab, Text } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { sentenceCase } from '@/map/SentenceCase';
import { md } from '@/theme/Colors';
import { AutotuneSection } from '@/ui/AutotuneSection';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';
import { TuningChart } from '@/ui/TuningChart';

export const PX4_TUNING_VIEW = 'view.px4Tuning';
export const SET_TUNING_TELEMETRY = 'vehicle.setPIDTuningTelemetryMode';
const DEFAULT_CHART_SECONDS = 8.0;

export type TuningParam = { title: string; description: string; min: number; max: number; step: number; fact: Fact };
export type TuningPlot = { name: string; path: string };
export type TuningAxis = { name: string; chartTitle: string; plot: readonly TuningPlot[]; params: readonly TuningParam[] };
export type TuningTab = {
  name: string;
  title: string;
  unit: string;
  extras: readonly Fact[];
  axes: readonly TuningAxis[];
  tuningMode: number;
  autoModeChange: boolean;
  autoTuning: boolean;
  chartSeconds: number;
};

const mapObjects = <T,>(list: unknown[] | null, read: (json: Json) => T | null): T[] =>
  jsonObjects(list).flatMap((it) => {
    const value = read(it);
    return value === null ? [] : [value];
  });

const tuningParam = (param: Json): TuningParam | null => {
  const control = optJSONObject(param, 'fact');
  const fact = control === null ? null : factFromControl(control);
  return fact === null
    ? null
    : {
        title: sentenceCase(optText(param, 'title')),
        description: optText(param, 'description'),
        min: Math.fround(optDouble(param, 'min')),
        max: Math.fround(optDouble(param, 'max')),
        step: Math.fround(optDouble(param, 'step')),
        fact,
      };
};

export const tuningTabs = (view: Json | null): readonly TuningTab[] =>
  view === null || !optBoolean(view, 'available')
    ? []
    : mapObjects(optJSONArray(view, 'tabs'), (tab) => ({
        name: sentenceCase(optText(tab, 'name')),
        title: optText(tab, 'title'),
        unit: optText(tab, 'unit'),
        extras: mapObjects(optJSONArray(tab, 'extras'), factFromControl),
        tuningMode: optInt(tab, 'tuningMode'),
        autoModeChange: optBoolean(tab, 'autoModeChange'),
        autoTuning: optBoolean(tab, 'autoTuning'),
        chartSeconds: optDouble(tab, 'chartSeconds', DEFAULT_CHART_SECONDS),
        axes: mapObjects(optJSONArray(tab, 'axes'), (axis) => ({
          name: sentenceCase(optText(axis, 'name')),
          chartTitle: sentenceCase(optText(axis, 'chartTitle')),
          plot: mapObjects(optJSONArray(axis, 'plot'), (it) => ({ name: optText(it, 'name'), path: optText(it, 'path') })),
          params: mapObjects(optJSONArray(axis, 'params'), tuningParam),
        })),
      }));

export type TuningModes = { stabilized: string; pause: string };

export const tuningModes = (view: Json | null): TuningModes => ({ stabilized: optText(view, 'stabilizedFlightMode'), pause: optText(view, 'pauseFlightMode') });

export const sliderSteps = (min: number, max: number, step: number): number => (step <= 0 || max <= min ? 0 : coerceAtLeast(roundToInt((max - min) / step) - 1, 0));

export const factNumber = (fact: Fact): number | null => (typeof fact.value === 'number' ? fact.value : toDoubleOrNull(fact.valueString));

type Clipboard = readonly (readonly [Fact, string])[];

const clipboardOf = (axis: TuningAxis): Clipboard => axis.params.map((it) => [it.fact, it.fact.valueString] as const);

export const Px4TuningScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [tabs, setTabs] = useState<readonly TuningTab[]>([]);
  const [modes, setModes] = useState<TuningModes>({ stabilized: '', pause: '' });
  const [useAutoTuning, setUseAutoTuning] = useState(false);
  const [loaded, setLoaded] = useState(false);
  const [tabIndex, setTabIndex] = useState(0);
  const [axisIndex, setAxisIndex] = useState(0);
  const [clipboard, setClipboard] = useState<Clipboard>([]);
  const [refusal, setRefusal] = useState<string | null>(null);
  const bump = () => setRevision((it) => it + 1);

  useLaunchedEffect([revision], async (scope) => {
    const read = await Qgc.get(PX4_TUNING_VIEW);
    if (!scope.isActive()) return;
    setTabs(tuningTabs(read));
    setModes(tuningModes(read));
    setLoaded(true);
  });

  const tab = loaded ? (tabs[tabIndex] ?? null) : null;
  const axis = tab === null ? null : (tab.axes[axisIndex] ?? tab.axes[0] ?? null);
  const showing = tab !== null;

  useEffect(() => {
    if (showing && axis !== null) setClipboard(clipboardOf(axis));
  }, [showing, tabIndex, axisIndex]);

  const tuningMode = tab?.tuningMode ?? null;
  useEffect(() => {
    if (tuningMode === null) return undefined;
    void Qgc.invoke(SET_TUNING_TELEMETRY, tuningMode);
    return () => void Qgc.invoke(SET_TUNING_TELEMETRY, 0);
  }, [tuningMode]);

  if (!loaded) {
    return <Text className={`block p-4 ${className ?? ''}`}>Reading parameters from the vehicle.</Text>;
  }
  if (tab === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>This vehicle has no PX4 tuning page.</Text>;
  }

  const writing = async (path: string, value: unknown): Promise<void> => {
    setRefusal(await Qgc.writeRefusal(path, value));
    bump();
  };
  const write = (path: string, value: unknown) => void writing(path, value);

  return (
    <div className={`flex h-full flex-col ${className ?? ''}`}>
      <ScrollableTabRow selectedTabIndex={tabIndex}>
        {tabs.map((each, index) => (
          <Tab
            key={index}
            selected={index === tabIndex}
            onClick={() => {
              setTabIndex(index);
              setAxisIndex(0);
            }}
            text={each.name}
          />
        ))}
      </ScrollableTabRow>
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-4">
        {tab.extras.map((fact) => (
          <FactRow key={fact.path} fact={fact} onWrite={bump} />
        ))}
        <div className="flex gap-2 overflow-x-auto">
          {tab.axes.map((each, index) => (
            <FilterChip key={index} selected={each === axis} onClick={() => setAxisIndex(index)} label={each.name} />
          ))}
        </div>
        {axis !== null ? <TuningChart axis={axis} unit={tab.unit} windowSeconds={tab.chartSeconds} modes={tab.autoModeChange ? modes : null} /> : null}
        {tab.autoTuning ? (
          <div className="flex items-center gap-2">
            <RadioButton selected={useAutoTuning} onClick={() => setUseAutoTuning(true)} label="Use auto-tuning" />
            <Text>Use auto-tuning</Text>
            <RadioButton selected={!useAutoTuning} onClick={() => setUseAutoTuning(false)} label="Use manual tuning" />
            <Text>Use manual tuning</Text>
          </div>
        ) : null}
        {tab.autoTuning && useAutoTuning ? (
          <AutotuneSection />
        ) : (
          <>
            {axis?.params.map((param) => (
              <TuningSlider key={param.fact.path} param={param} onWrite={(value) => write(param.fact.path, value)} />
            ))}
            {axis !== null ? (
              <div className="flex gap-2">
                <OutlinedButton onClick={() => setClipboard(clipboardOf(axis))}>Save to clipboard</OutlinedButton>
                <OutlinedButton
                  onClick={() =>
                    void Promise.all(
                      clipboard.flatMap(([fact, value]) => {
                        const number = toDoubleOrNull(value);
                        return number === null ? [] : [writing(fact.path, number)];
                      }),
                    )
                  }
                  enabled={clipboard.length > 0}
                >
                  Restore from clipboard
                </OutlinedButton>
              </div>
            ) : null}
          </>
        )}
        {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
        {clipboard.length > 0 ? (
          <>
            <Text typography="labelLarge">Clipboard values:</Text>
            {clipboard.map(([fact, value], index) => (
              <Text key={index} typography="bodySmall" className="whitespace-pre">{`${fact.name}  ${value}`}</Text>
            ))}
          </>
        ) : null}
      </div>
    </div>
  );
};

const TuningSlider = ({ param, onWrite }: { param: TuningParam; onWrite: (value: number) => void }) => {
  const current = factNumber(param.fact);
  const [dragging, setDraggingState] = useKeyedState([param.fact.path, current], () => current ?? param.min);
  const draggingRef = useRef(dragging);
  draggingRef.current = dragging;
  const setDragging = (next: number) => {
    draggingRef.current = next;
    setDraggingState(next);
  };
  return (
    <div className="flex w-full flex-col">
      <Text typography="bodyMedium">{param.title}</Text>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {param.description}
      </Text>
      <div className="flex items-center">
        <Slider
          value={coerceIn(dragging, param.min, param.max)}
          onValueChange={setDragging}
          onValueChangeFinished={() => onWrite(draggingRef.current)}
          valueRange={[param.min, param.max]}
          steps={sliderSteps(param.min, param.max, param.step)}
          label={param.title}
          className="flex-1"
        />
        <Text className="pl-2">{param.fact.valueString}</Text>
      </div>
    </div>
  );
};
