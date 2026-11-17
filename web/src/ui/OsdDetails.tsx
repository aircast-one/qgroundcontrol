import type { Fact } from '@/bridge/Qgc';
import { useQgcDouble, useQgcFacts, useQgcPath } from '@/bridge/QgcState';
import type { WhepStats } from '@/core/whep';
import { mutableStateMapOf, useValue } from '@/kotlin/State';
import type { DrawableName } from '@/res/drawables';
import { aircast, md } from '@/theme/Colors';
import type { FlyState } from './FlyState';
import { DetailRow } from './InstrumentDetail';
import { VIBRATION_VIEW, type VibrationReading, vibrationReading, worstSeverity } from './VibrationScreen';
import { withUnit } from './Quantity';

export const mainVideoStats = mutableStateMapOf<symbol, WhepStats>();

export const playingStats = (surfaces: ReadonlyMap<symbol, WhepStats>): WhepStats | null => [...surfaces.values()].at(0) ?? null;

type Tone = 'error' | 'warning' | null;
export type OsdDetail = { label: string; value: string; tone: Tone; hint?: string };
export type OsdLine = { id: string; icon: DrawableName; details: readonly OsdDetail[] };

export type OsdDetailsInput = {
  hdop: number;
  ekfRatios: readonly number[];
  vibration: VibrationReading | null;
  video: WhepStats | null;
};

const EKF_RATIOS: ReadonlySet<string> = new Set(['velRatio', 'horizPosRatio', 'vertPosRatio', 'magRatio', 'haglRatio', 'tasRatio']);
const ESTIMATOR_STATUS = 'vehicle.estimatorStatus';
const EKF_WARNING = 0.5;
const EKF_DANGER = 0.8;
const LOSS_WARNING = 2;
const LOSS_DANGER = 10;
const SKIP_WARNING = 1;
const SKIP_DANGER = 5;

const detail = (label: string, value: string, tone: Tone = null): OsdDetail[] => (value.trim() === '' ? [] : [{ label, value, tone }]);

export const ekfRatiosOf = (facts: readonly Fact[]): readonly number[] => facts.filter((it) => EKF_RATIOS.has(it.name)).map((it) => (typeof it.value === 'number' ? it.value : NaN));

export const ekfWorst = (ratios: readonly number[]): number | null => {
  const seen = ratios.filter((it) => Number.isFinite(it));
  return seen.length === 0 ? null : Math.max(...seen);
};

const ekfTone = (worst: number): Tone => (worst >= EKF_DANGER ? 'error' : worst >= EKF_WARNING ? 'warning' : null);

const severityTone = (severity: string | null): Tone => (severity === 'danger' ? 'error' : severity === 'warning' ? 'warning' : null);

const rateTone = (value: number, warning: number, danger: number): Tone => (value >= danger ? 'error' : value >= warning ? 'warning' : null);

export const percentText = (value: number): string => `${value < 10 ? value.toFixed(1) : Math.round(value)}%`;

export const bitrateText = (kbps: number): string => (kbps >= 1000 ? withUnit((kbps / 1000).toFixed(1), 'Mb/s') : withUnit(String(kbps), 'kb/s'));

const hdopDetails = (hdop: number): OsdDetail[] => (Number.isFinite(hdop) && hdop > 0 && hdop < 100 ? detail('HDOP', hdop.toFixed(1)) : []);

const ekfDetails = (ratios: readonly number[]): OsdDetail[] => {
  const worst = ekfWorst(ratios);
  return worst === null ? [] : detail('EKF variance', worst.toFixed(2), ekfTone(worst));
};

const withUnits = (value: number, units: string | undefined): string => [value.toFixed(0), units ?? ''].join(' ').trim();

export const vibrationHint = (tone: Tone, vibration: VibrationReading): string => {
  const units = vibration.units as string | undefined;
  if (tone === 'error') return `At or over the ${withUnits(vibration.dangerLevel, units)} danger level. Balance or replace the props and soft-mount the flight controller before flying.`;
  if (tone === 'warning') return `Over the ${withUnits(vibration.warningLevel, units)} warning level. Check the props and motor mounts.`;
  return `Under the ${withUnits(vibration.warningLevel, units)} warning level.`;
};

const vibrationDetails = (vibration: VibrationReading | null): OsdDetail[] => {
  const values = (vibration?.axes ?? []).flatMap((it) => (it.value === null ? [] : [it.value]));
  if (vibration === null || values.length === 0) return [];
  const tone = severityTone(worstSeverity(vibration));
  const units = (vibration.units as string | undefined) ?? '';
  return detail('Vibration', `${Math.max(...values).toFixed(1)}${units.trim()}`, tone).map((it) => ({ ...it, hint: vibrationHint(tone, vibration) }));
};

const videoDetails = (video: WhepStats | null): OsdDetail[] =>
  video === null
    ? []
    : [
        ...detail('Bitrate', bitrateText(video.kbps)),
        ...detail('Frame rate', withUnit(String(video.fps), 'fps')),
        ...detail('Jitter', withUnit(String(video.jitterMs), 'ms')),
        ...(video.lossPercent === null
          ? detail('Packets lost', `${Math.round(video.lostPerSecond)}/s`)
          : detail('Packet loss', percentText(video.lossPercent), rateTone(video.lossPercent, LOSS_WARNING, LOSS_DANGER))),
        ...detail('Skipped frames', `${Math.round(video.skippedPerSecond)}/s`, rateTone(video.skippedPerSecond, SKIP_WARNING, SKIP_DANGER)),
      ];

export const osdLines = (input: OsdDetailsInput): readonly OsdLine[] =>
  (
    [
      { id: 'health', icon: 'ic_explore', details: [...ekfDetails(input.ekfRatios), ...vibrationDetails(input.vibration), ...hdopDetails(input.hdop)] },
      { id: 'video', icon: 'ic_videocam', details: videoDetails(input.video) },
    ] satisfies readonly OsdLine[]
  ).filter((line) => line.details.length > 0);

const toneRank = (tone: Tone): number => (tone === 'error' ? 2 : tone === 'warning' ? 1 : 0);

export const osdCellText = (line: OsdLine): string =>
  line.details
    .toSorted((a, b) => toneRank(b.tone) - toneRank(a.tone))
    .map((it) => `${it.label} ${it.value}`)
    .join(' · ');

export const osdCellColour = (line: OsdLine): string | null => {
  const worst = Math.max(...line.details.map((it) => toneRank(it.tone)));
  return worst >= 2 ? md('error') : worst === 1 ? aircast('warning') : null;
};

export const osdRows = (line: OsdLine): readonly DetailRow[] => line.details.map((it) => DetailRow({ label: it.label, value: it.value, severity: toneRank(it.tone), hint: it.hint }));

export const RC_NO_SIGNAL = 'RC has no signal';

export const redIndicators = (lines: readonly OsdLine[], state: FlyState | null): readonly string[] => [
  ...lines
    .filter((line) => line.id === 'health')
    .flatMap((line) => line.details)
    .filter((it) => it.tone === 'error')
    .map((it) => `${it.label} ${it.value}`),
  ...(state !== null && state.connected && state.rcSupported && state.rcRequired && state.rcSignal === 0 ? [RC_NO_SIGNAL] : []),
];

export const NOT_FULLY_READY = 'Not fully ready';

export const withRedIndicators = (state: FlyState | null, red: readonly string[]): FlyState | null =>
  state === null || red.length === 0 || !state.connected || state.armed || state.contactLost === true || !state.readyToFly || !state.nominal
    ? state
    : { ...state, nominal: false, stateText: NOT_FULLY_READY, summaryDetail: `${red.join('. ')}. Tap a red reading to see how to fix it.` };

export const useRedIndicators = (state: FlyState | null): readonly string[] => redIndicators(useOsdLines(), state);

export const useOsdLines = (): readonly OsdLine[] =>
  osdLines({
    hdop: useQgcDouble('vehicle.gps.hdop'),
    ekfRatios: ekfRatiosOf(useQgcFacts(ESTIMATOR_STATUS)),
    vibration: vibrationReading(useQgcPath(VIBRATION_VIEW)),
    video: playingStats(useValue(mainVideoStats)),
  });
