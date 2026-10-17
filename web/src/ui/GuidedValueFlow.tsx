import { useContext } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { type MutableState, mutableStateOf, useValue } from '@/kotlin/State';
import { Slider, stepsFor } from '@/m3';
import { type ClosedRange } from '@/map/SettingStepper';
import type { GuidedOffer } from './GuidedActions';
import { GUIDED_ALTITUDE, type GuidedAltitude, altitudeDelta, guidedAltitude, guidedAltitudePath, rangeLabel } from './GuidedAltitude';
import { GUIDED_SPEED, type GuidedSpeed, guidedSpeed, guidedSpeedPath } from './GuidedSpeed';
import { GUIDED_TAKEOFF, type GuidedTakeoff, guidedTakeoff, guidedTakeoffPath } from './GuidedTakeoff';
import { GuidedPresets, GuidedQuickPicks, GuidedStepper, GuidedValuePanel, LocalGuidedCompact, guidedDecimals, guidedRounded, guidedValueText } from './GuidedValuePanel';
import { PAUSE } from './MoreActions';
import { RangeHint } from './VehicleUi';

export type GuidedReading = {
  readonly label: string;
  readonly unit: string;
  readonly range: ClosedRange | null;
  readonly initial: number | null;
  readonly sentence: string;
  readonly sendable: boolean;
};

export type GuidedValueKind = {
  readonly offerId: string;
  readonly title: string;
  readonly prompt: string | null;
  readonly commitLabel: string;
  readonly missingRange: string;
  readonly quickPicks: boolean;
  readonly presets: boolean;
  readonly explains: boolean;
  readonly read: (target: number | null) => Promise<GuidedReading | null>;
  readonly commit: (target: number) => Promise<void>;
};

export const GuidedValueKind = (fields: Omit<GuidedValueKind, 'presets' | 'explains'> & { presets?: boolean; explains?: boolean }): GuidedValueKind => ({
  presets: false,
  explains: true,
  ...fields,
});

export class OpenGuidedValue {
  readonly target: MutableState<number>;
  readonly settled: MutableState<number>;

  constructor(
    readonly kind: GuidedValueKind,
    readonly reading: GuidedReading,
    readonly range: ClosedRange,
    initial: number,
  ) {
    this.target = mutableStateOf(initial);
    this.settled = mutableStateOf(initial);
  }
}

export const openedGuidedValue = (kind: GuidedValueKind, reading: GuidedReading | null): OpenGuidedValue | null => {
  const range = reading?.range ?? null;
  if (reading === null || range === null) return null;
  const initial = reading.initial;
  if (initial === null) return null;
  return new OpenGuidedValue(kind, reading, range, initial);
};

const usableRange = (minimum: number | null, maximum: number | null): ClosedRange | null =>
  minimum !== null && maximum !== null && maximum > minimum ? { start: minimum, endInclusive: maximum } : null;

export const takeoffReading = (takeoff: GuidedTakeoff | null): GuidedReading | null =>
  takeoff === null ? null : { label: takeoff.label, unit: takeoff.unit, range: usableRange(takeoff.minimum, takeoff.maximum), initial: takeoff.initial, sentence: takeoff.sentence, sendable: true };

export const speedReading = (speed: GuidedSpeed | null): GuidedReading | null =>
  speed === null ? null : { label: speed.label, unit: speed.unit, range: speed.command !== null ? usableRange(speed.minimum, speed.maximum) : null, initial: speed.initial, sentence: speed.sentence, sendable: true };

export const altitudeReading = (altitude: GuidedAltitude | null): GuidedReading | null =>
  altitude === null ? null : { label: altitude.label, unit: altitude.unit, range: usableRange(altitude.minimum, altitude.maximum), initial: altitude.current, sentence: altitude.sentence, sendable: altitude.sends };

const titleOr = (offer: GuidedOffer | null, fallback: string): string => {
  const title = offer?.title ?? '';
  return title.trim() === '' ? fallback : title;
};

export const takeoffValue = (offer: GuidedOffer | null): GuidedValueKind =>
  GuidedValueKind({
    offerId: 'takeoff',
    title: titleOr(offer, 'Takeoff'),
    prompt: offer?.prompt ?? null,
    commitLabel: 'Take off',
    missingRange: 'This vehicle did not report a takeoff height range.',
    quickPicks: false,
    presets: true,
    explains: false,
    read: async (target) => takeoffReading(guidedTakeoff(await Qgc.get(target === null ? GUIDED_TAKEOFF : guidedTakeoffPath(target)))),
    commit: async (target) => {
      const fresh = guidedTakeoff(await Qgc.get(guidedTakeoffPath(target)));
      if (fresh !== null) await VehicleCommands.takeoff(fresh.targetMeters);
    },
  });

export const speedValue = (offer: GuidedOffer | null): GuidedValueKind =>
  GuidedValueKind({
    offerId: 'changeSpeed',
    title: titleOr(offer, 'Change Max Ground Speed'),
    prompt: offer?.prompt ?? null,
    commitLabel: 'Set',
    missingRange: 'This vehicle did not report a speed range.',
    quickPicks: false,
    read: async (target) => speedReading(guidedSpeed(await Qgc.get(target === null ? GUIDED_SPEED : guidedSpeedPath(target)))),
    commit: async (target) => {
      const fresh = guidedSpeed(await Qgc.get(guidedSpeedPath(target)));
      if (fresh?.command) await VehicleCommands.changeSpeed(fresh.command, fresh.targetMetersSecond);
    },
  });

export const altitudeValue = (pauses: boolean): GuidedValueKind =>
  GuidedValueKind({
    offerId: pauses ? PAUSE : 'changeAltitude',
    title: pauses ? 'Pause' : 'Change altitude',
    prompt: null,
    commitLabel: pauses ? 'Pause' : 'Change',
    missingRange: 'This vehicle did not report an altitude range.',
    quickPicks: true,
    read: async (target) => altitudeReading(guidedAltitude(await Qgc.get(target === null ? GUIDED_ALTITUDE : guidedAltitudePath(target, pauses)))),
    commit: async (target) => {
      const fresh = guidedAltitude(await Qgc.get(guidedAltitudePath(target, pauses)));
      const delta = fresh === null ? null : altitudeDelta(fresh);
      if (delta !== null) await VehicleCommands.changeAltitude(delta, pauses);
    },
  });

export const openGuidedValue = async (kind: GuidedValueKind): Promise<OpenGuidedValue | null> => openedGuidedValue(kind, await kind.read(null));

const GuidedValueContent = ({ open }: { open: OpenGuidedValue }) => {
  const kind = open.kind;
  const unit = open.reading.unit;
  const target = useValue(open.target);
  const compact = useContext(LocalGuidedCompact);
  const settle = (value: number) => {
    open.target.value = value;
    open.settled.value = value;
  };
  const { start, endInclusive } = open.range;
  const range = rangeLabel(start, endInclusive, unit);
  return (
    <>
      <GuidedStepper value={target} label={guidedLabel(open.reading.label, range, compact)} unit={unit} minimum={start} maximum={endInclusive} onValue={settle} />
      {kind.presets ? <GuidedPresets value={target} minimum={start} maximum={endInclusive} unit={unit} onValue={settle} /> : null}
      {!compact ? (
        <Slider
          value={target}
          onValueChange={(it) => {
            open.target.value = guidedRounded(it, unit);
          }}
          onValueChangeFinished={() => {
            open.settled.value = open.target.value;
          }}
          valueRange={[start, endInclusive]}
          steps={stepsFor(start, endInclusive, 10 ** -guidedDecimals(unit))}
          label={open.reading.label}
        />
      ) : null}
      {kind.quickPicks ? <GuidedQuickPicks value={target} minimum={start} maximum={endInclusive} unit={unit} onValue={settle} /> : null}
      {!compact && range !== null ? <RangeHint text={range} /> : null}
    </>
  );
};

export const GuidedValueFlow = ({ open, onClose }: { open: OpenGuidedValue; onClose: () => void }) => {
  const kind = open.kind;
  const unit = open.reading.unit;
  const target = useValue(open.target);
  const settled = useValue(open.settled);
  const [probe, setProbe] = useKeyedState<GuidedReading | null>([open], () => null);
  useLaunchedEffect([open, settled], async (scope) => {
    const reading = await kind.read(settled);
    if (scope.isActive()) setProbe(reading);
  });
  return (
    <GuidedValuePanel
      title={kind.title}
      sentence={kind.explains ? [kind.prompt, probe?.sentence ?? null].filter((it): it is string => it !== null && it.trim() !== '').join('\n') : ''}
      commitLabel={`${kind.commitLabel} · ${guidedValueText(target, unit)} ${unit}`.trim()}
      commitEnabled={probe?.sendable === true}
      onCommit={() => {
        const chosen = open.target.value;
        onClose();
        void kind.commit(chosen);
      }}
      onCancel={onClose}
    >
      <GuidedValueContent open={open} />
    </GuidedValuePanel>
  );
};

export const guidedLabel = (label: string, range: string | null, compact: boolean): string =>
  compact && range !== null ? [label, range].filter((it) => it.trim() !== '').join(' · ') : label;
