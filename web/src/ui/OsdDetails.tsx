import { useState } from 'react';
import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import type { WhepStats } from '@/core/whep';
import { mutableStateOf, useValue } from '@/kotlin/State';
import { Text } from '@/m3';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast, md, withAlpha } from '@/theme/Colors';
import { FLY_STATE, flyState } from './FlyState';
import { USB_RADIO_VIEW, type UsbRadioBadge, usbRadioBadge, usbRadioTone } from './UsbRadioIndicator';
import { OSD_LABEL_ALPHA } from './VehicleUi';
import { VIBRATION_VIEW, type VibrationReading, vibrationReading, worstSeverity } from './VibrationScreen';

export const mainVideoStats = mutableStateOf<WhepStats | null>(null);

type Tone = 'error' | 'warning' | null;
export type OsdDetail = { label: string; value: string; tone: Tone };
export type OsdLine = { id: string; icon: DrawableName; details: readonly OsdDetail[] };

export type OsdDetailsInput = {
  hdop: number;
  ekfRatios: readonly number[];
  vibration: VibrationReading | null;
  contactLost: boolean;
  lossPercent: number;
  video: WhepStats | null;
  radio: UsbRadioBadge | null;
};

const EKF_RATIOS = ['velRatio', 'horizPosRatio', 'vertPosRatio', 'magRatio', 'haglRatio', 'tasRatio'] as const;
const EKF_WARNING = 0.5;
const EKF_DANGER = 0.8;
const ICON_SIZE = 20;
const LOSS_WARNING = 2;
const LOSS_DANGER = 10;
const SKIP_WARNING = 1;
const SKIP_DANGER = 5;

const detail = (label: string, value: string, tone: Tone = null): OsdDetail[] => (value.trim() === '' ? [] : [{ label, value, tone }]);

export const ekfWorst = (ratios: readonly number[]): number | null => {
  const seen = ratios.filter((it) => Number.isFinite(it));
  return seen.length === 0 ? null : Math.max(...seen);
};

const ekfTone = (worst: number): Tone => (worst >= EKF_DANGER ? 'error' : worst >= EKF_WARNING ? 'warning' : null);

const severityTone = (severity: string | null): Tone => (severity === 'danger' ? 'error' : severity === 'warning' ? 'warning' : null);

const rateTone = (value: number, warning: number, danger: number): Tone => (value >= danger ? 'error' : value >= warning ? 'warning' : null);

export const percentText = (value: number): string => `${value < 10 ? value.toFixed(1) : Math.round(value)}%`;

export const bitrateText = (kbps: number): string => (kbps >= 1000 ? `${(kbps / 1000).toFixed(1)}Mb/s` : `${kbps}kb/s`);

const hdopDetails = (hdop: number): OsdDetail[] => (Number.isFinite(hdop) && hdop > 0 && hdop < 100 ? detail('HDOP', hdop.toFixed(1)) : []);

const ekfDetails = (ratios: readonly number[]): OsdDetail[] => {
  const worst = ekfWorst(ratios);
  return worst === null ? [] : detail('EKF', worst.toFixed(2), ekfTone(worst));
};

const vibrationDetails = (vibration: VibrationReading | null): OsdDetail[] => {
  const values = (vibration?.axes ?? []).flatMap((it) => (it.value === null ? [] : [it.value]));
  return vibration === null || values.length === 0 ? [] : detail('VIB', Math.max(...values).toFixed(1), severityTone(worstSeverity(vibration)));
};

const videoLine = (video: WhepStats | null): OsdLine =>
  video === null
    ? { id: 'video', icon: 'ic_videocam_off', details: detail('VIDEO', 'OFF', 'warning') }
    : {
        id: 'video',
        icon: 'ic_videocam',
        details: [
          ...detail('VIDEO', `${video.fps}fps`),
          ...detail('BR', bitrateText(video.kbps)),
          ...detail('JIT', `${video.jitterMs}ms`),
          ...detail('PKT LOSS', percentText(video.lossPercent), rateTone(video.lossPercent, LOSS_WARNING, LOSS_DANGER)),
          ...detail('SKIP', `${Math.round(video.skippedPerSecond)}/s`, rateTone(video.skippedPerSecond, SKIP_WARNING, SKIP_DANGER)),
        ],
      };

export const osdLines = (input: OsdDetailsInput): readonly OsdLine[] =>
  [
    { id: 'health', icon: 'ic_explore', details: [...ekfDetails(input.ekfRatios), ...vibrationDetails(input.vibration), ...hdopDetails(input.hdop)] },
    {
      id: 'link',
      icon: 'ic_sensors',
      details: [...detail('TLM', input.contactLost ? 'LOST' : 'LIVE', input.contactLost ? 'error' : null), ...(Number.isFinite(input.lossPercent) ? detail('TLM LOSS', percentText(input.lossPercent)) : [])],
    },
    videoLine(input.video),
    { id: 'radio', icon: 'ic_gamepad', details: input.radio === null ? [] : detail('RADIO', input.radio.state.toUpperCase(), usbRadioTone(input.radio.state)) },
  ].filter((line): line is OsdLine => line.details.length > 0);

const worstTone = (details: readonly OsdDetail[]): Tone => (details.some((it) => it.tone === 'error') ? 'error' : details.some((it) => it.tone === 'warning') ? 'warning' : null);

export const shownDetails = (line: OsdLine, opened: boolean): readonly OsdDetail[] => (opened || worstTone(line.details) !== null ? line.details : line.details.slice(0, 1));

const toneColour = (tone: Tone): string => (tone === 'error' ? md('error') : tone === 'warning' ? aircast('warning') : aircast('outdoorForeground'));

const toggled = (set: ReadonlySet<string>, id: string): ReadonlySet<string> => new Set(set.has(id) ? [...set].filter((it) => it !== id) : [...set, id]);

export const OsdDetails = () => {
  const [opened, setOpened] = useState<ReadonlySet<string>>(new Set());
  const ekfRatios = EKF_RATIOS.map((name) => useQgcDouble(`vehicle.estimatorStatus.${name}`));
  const lines = osdLines({
    hdop: useQgcDouble('vehicle.gps.hdop'),
    ekfRatios,
    vibration: vibrationReading(useQgcPath(VIBRATION_VIEW)),
    contactLost: flyState(useQgcPath(FLY_STATE))?.contactLost === true,
    lossPercent: useQgcDouble('vehicle.mavlinkLossPercent'),
    video: useValue(mainVideoStats),
    radio: usbRadioBadge(useQgcPath(USB_RADIO_VIEW)),
  });
  return (
    <div data-osd-details className="osd-shadow flex min-w-0 flex-1 flex-wrap items-center justify-center gap-x-6 gap-y-1">
      {lines.map((line) => (
        <button
          key={line.id}
          type="button"
          aria-label={`${line.id} readings`}
          aria-expanded={opened.has(line.id)}
          onClick={() => setOpened((was) => toggled(was, line.id))}
          className="flex cursor-pointer items-center gap-3 whitespace-nowrap"
        >
          <Drawable name={line.icon} size={ICON_SIZE} style={{ color: toneColour(worstTone(line.details)) }} />
          {shownDetails(line, opened.has(line.id)).map((it) => (
            <span key={it.label} className="flex items-baseline gap-1">
              <Text typography="labelLarge" color={withAlpha(aircast('outdoorForeground'), OSD_LABEL_ALPHA)} maxLines={1}>
                {it.label}
              </Text>
              <Text typography="titleLarge" className="tabular-nums" color={toneColour(it.tone)} maxLines={1}>
                {it.value}
              </Text>
            </span>
          ))}
        </button>
      ))}
    </div>
  );
};
