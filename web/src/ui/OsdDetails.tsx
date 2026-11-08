import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import type { WhepStats } from '@/core/whep';
import { mutableStateOf, useValue } from '@/kotlin/State';
import { Text } from '@/m3';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { FLY_STATE, flyState } from './FlyState';
import { USB_RADIO_VIEW, type UsbRadioBadge, usbRadioBadge, usbRadioTone } from './UsbRadioIndicator';
import { OSD_LABEL_ALPHA } from './VehicleUi';
import { VIBRATION_VIEW, type VibrationReading, vibrationReading, worstSeverity } from './VibrationScreen';

export const mainVideoStats = mutableStateOf<WhepStats | null>(null);

type Tone = 'error' | 'warning' | null;
export type OsdDetail = { label: string; value: string; tone: Tone };
export type OsdLine = { icon: DrawableName; details: readonly OsdDetail[] };

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
const ICON_SIZE = 14;
const STRIPE_HEIGHT = 24;

const detail = (label: string, value: string, tone: Tone = null): OsdDetail[] => (value.trim() === '' ? [] : [{ label, value, tone }]);

export const ekfWorst = (ratios: readonly number[]): number | null => {
  const seen = ratios.filter((it) => Number.isFinite(it));
  return seen.length === 0 ? null : Math.max(...seen);
};

const ekfTone = (worst: number): Tone => (worst >= EKF_DANGER ? 'error' : worst >= EKF_WARNING ? 'warning' : null);

const severityTone = (severity: string | null): Tone => (severity === 'danger' ? 'error' : severity === 'warning' ? 'warning' : null);

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
    ? { icon: 'ic_videocam_off', details: detail('VIDEO', 'OFF', 'warning') }
    : {
        icon: 'ic_videocam',
        details: [
          ...detail('VIDEO', `${video.fps}fps`),
          ...detail('BR', bitrateText(video.kbps)),
          ...detail('JIT', `${video.jitterMs}ms`),
          ...detail('LOST', String(video.lost), video.lost > 0 ? 'warning' : null),
          ...detail('DROP', String(video.dropped), video.dropped > 0 ? 'warning' : null),
        ],
      };

export const osdLines = (input: OsdDetailsInput): readonly OsdLine[] =>
  [
    { icon: 'ic_explore', details: [...hdopDetails(input.hdop), ...ekfDetails(input.ekfRatios), ...vibrationDetails(input.vibration)] },
    {
      icon: 'ic_sensors',
      details: [...detail('TLM', input.contactLost ? 'LOST' : 'LIVE', input.contactLost ? 'error' : null), ...(Number.isFinite(input.lossPercent) ? detail('LOSS', `${Math.round(input.lossPercent)}%`) : [])],
    },
    videoLine(input.video),
    { icon: 'ic_gamepad', details: input.radio === null ? [] : detail('RADIO', input.radio.state.toUpperCase(), usbRadioTone(input.radio.state)) },
  ].filter((line): line is OsdLine => line.details.length > 0);

const worstTone = (details: readonly OsdDetail[]): Tone => (details.some((it) => it.tone === 'error') ? 'error' : details.some((it) => it.tone === 'warning') ? 'warning' : null);

const toneColour = (tone: Tone): string => (tone === 'error' ? md('error') : tone === 'warning' ? aircast('warning') : aircast('outdoorForeground'));

export const OsdDetails = () => {
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
    <div
      data-osd-details
      className="flex w-full shrink-0 items-center justify-center-safe overflow-x-auto whitespace-nowrap"
      style={{ height: STRIPE_HEIGHT, background: Color.Black, paddingInline: 'max(12px, env(safe-area-inset-left, 0px), env(safe-area-inset-right, 0px))' }}
    >
      {lines.map((line, at) => (
        <div key={line.icon} className="flex items-center gap-2.5" style={{ paddingInline: 12, borderInlineStart: at === 0 ? undefined : `1px solid ${withAlpha(Color.White, 0.18)}` }}>
          <Drawable name={line.icon} size={ICON_SIZE} style={{ color: toneColour(worstTone(line.details)) }} />
          {line.details.map((it) => (
            <div key={it.label} className="flex items-baseline gap-1">
              <Text typography="labelSmall" color={withAlpha(aircast('outdoorForeground'), OSD_LABEL_ALPHA)} maxLines={1}>
                {it.label}
              </Text>
              <Text typography="labelLarge" className="tabular-nums" color={toneColour(it.tone)} maxLines={1}>
                {it.value}
              </Text>
            </div>
          ))}
        </div>
      ))}
    </div>
  );
};
