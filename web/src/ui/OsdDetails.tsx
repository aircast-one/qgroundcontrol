import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray, optText } from '@/bridge/Json';
import { useQgcDouble, useQgcPath } from '@/bridge/QgcState';
import type { WhepStats } from '@/core/whep';
import { mutableStateOf, useValue } from '@/kotlin/State';
import { Text } from '@/m3';
import { aircast, md, withAlpha } from '@/theme/Colors';
import { FLY_STATE, flyState } from './FlyState';
import { BATTERY_VIEW } from './InstrumentDetail';
import { USB_RADIO_VIEW, type UsbRadioBadge, usbRadioBadge, usbRadioTone } from './UsbRadioIndicator';
import { OSD_LABEL_ALPHA } from './VehicleUi';
import { VIBRATION_VIEW, type VibrationReading, vibrationReading, worstSeverity } from './VibrationScreen';

export const mainVideoStats = mutableStateOf<WhepStats | null>(null);

type Tone = 'error' | 'warning' | null;
export type OsdDetail = { label: string; value: string; tone: Tone };

export type OsdDetailsInput = {
  battery: Json | null;
  gps: Json | null;
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
const GPS_VIEW = 'view.gps';

const detail = (label: string, value: string, tone: Tone = null): OsdDetail[] => (value.trim() === '' ? [] : [{ label, value, tone }]);

export const ekfWorst = (ratios: readonly number[]): number | null => {
  const seen = ratios.filter((it) => Number.isFinite(it));
  return seen.length === 0 ? null : Math.max(...seen);
};

const ekfTone = (worst: number): Tone => (worst >= EKF_DANGER ? 'error' : worst >= EKF_WARNING ? 'warning' : null);

const severityTone = (severity: string | null): Tone => (severity === 'danger' ? 'error' : severity === 'warning' ? 'warning' : null);

export const bitrateText = (kbps: number): string => (kbps >= 1000 ? `${(kbps / 1000).toFixed(1)}Mb/s` : `${kbps}kb/s`);

const batteryDetails = (battery: Json | null): OsdDetail[] => {
  const pack = jsonObjects(optJSONArray(battery, 'packs'))[0] ?? null;
  return pack === null ? [] : detail('BAT', `${optText(pack, 'voltageText')} ${optText(pack, 'currentText')}`);
};

const gpsDetails = (gps: Json | null, hdop: number): OsdDetail[] =>
  gps === null || !optBoolean(gps, 'available')
    ? []
    : [
        ...detail('GPS', optText(gps, 'lockText')),
        ...detail('SAT', String(optDouble(gps, 'satellites', 0))),
        ...(Number.isFinite(hdop) && hdop > 0 ? detail('HDOP', hdop.toFixed(1)) : []),
      ];

const ekfDetails = (ratios: readonly number[]): OsdDetail[] => {
  const worst = ekfWorst(ratios);
  return worst === null ? [] : detail('EKF', worst.toFixed(2), ekfTone(worst));
};

const vibrationDetails = (vibration: VibrationReading | null): OsdDetail[] => {
  const values = (vibration?.axes ?? []).flatMap((it) => (it.value === null ? [] : [it.value]));
  return vibration === null || values.length === 0 ? [] : detail('VIBE', Math.max(...values).toFixed(1), severityTone(worstSeverity(vibration)));
};

const videoDetails = (video: WhepStats | null): OsdDetail[] =>
  video === null
    ? detail('VIDEO', 'OFF', 'warning')
    : [
        ...detail('VIDEO', `${video.fps}fps ${bitrateText(video.kbps)}`),
        ...detail('JIT', `${video.jitterMs}ms`),
        ...detail('LOST', String(video.lost), video.lost > 0 ? 'warning' : null),
        ...detail('DROP', String(video.dropped), video.dropped > 0 ? 'warning' : null),
      ];

export const osdDetails = (input: OsdDetailsInput): readonly OsdDetail[] => [
  ...batteryDetails(input.battery),
  ...gpsDetails(input.gps, input.hdop),
  ...ekfDetails(input.ekfRatios),
  ...vibrationDetails(input.vibration),
  ...detail('TLM', input.contactLost ? 'LOST' : 'LIVE', input.contactLost ? 'error' : null),
  ...(Number.isFinite(input.lossPercent) ? detail('LOSS', `${Math.round(input.lossPercent)}%`) : []),
  ...videoDetails(input.video),
  ...(input.radio === null ? [] : detail('RADIO', input.radio.state.toUpperCase(), usbRadioTone(input.radio.state))),
];

const toneColour = (tone: Tone): string => (tone === 'error' ? md('error') : tone === 'warning' ? aircast('warning') : aircast('outdoorForeground'));

export const OsdDetails = () => {
  const ekfRatios = EKF_RATIOS.map((name) => useQgcDouble(`vehicle.estimatorStatus.${name}`));
  const details = osdDetails({
    battery: useQgcPath(BATTERY_VIEW),
    gps: useQgcPath(GPS_VIEW),
    hdop: useQgcDouble('vehicle.gps.hdop'),
    ekfRatios,
    vibration: vibrationReading(useQgcPath(VIBRATION_VIEW)),
    contactLost: flyState(useQgcPath(FLY_STATE))?.contactLost === true,
    lossPercent: useQgcDouble('vehicle.mavlinkLossPercent'),
    video: useValue(mainVideoStats),
    radio: usbRadioBadge(useQgcPath(USB_RADIO_VIEW)),
  });
  return (
    <div className="mb-1 flex flex-wrap gap-x-4 gap-y-0.5" data-osd-details>
      {details.map((it) => (
        <div key={it.label} className="flex items-baseline gap-1">
          <Text typography="labelMedium" color={withAlpha(aircast('outdoorForeground'), OSD_LABEL_ALPHA)} maxLines={1}>
            {it.label}
          </Text>
          <Text typography="titleMedium" className="tabular-nums" color={toneColour(it.tone)} maxLines={1}>
            {it.value}
          </Text>
        </div>
      ))}
    </div>
  );
};
