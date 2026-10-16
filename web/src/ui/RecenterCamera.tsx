import { Qgc } from '@/bridge/Qgc';
import { Surface, Text } from '@/m3';
import { roundToInt } from '@/kotlin/Std';
import { Drawable } from '@/res/Icon';
import { Color, aircast, withAlpha } from '@/theme/Colors';
import { type GimbalIndicatorState, gimbalRefusal } from '@/ui/GimbalIndicator';
import { osdShadow } from '@/ui/Osd';
import { DEFAULT_TILT_DEG, SYNTHETIC_AIM } from '@/ui/SyntheticView';

const GIMBAL_CENTER = 'gimbal.center';
const OFF_CENTRE_DEG = 2.0;
const RECENTER_SCRIM_ALPHA = 0.55;
const TILT_READOUT_WIDTH = 64;
const TILT_READOUT_HEIGHT = 32;
const RECENTER_GLYPH = 14;
const TILT_READOUT_PADDING = 8;

export type RecenterTarget = { readonly name: 'Synthetic' | 'Gimbal'; readonly ordinal: number };

export const RecenterTarget = {
  Synthetic: { name: 'Synthetic', ordinal: 0 },
  Gimbal: { name: 'Gimbal', ordinal: 1 },
} as const satisfies Record<string, RecenterTarget>;

export const RecenterTargetEntries: readonly RecenterTarget[] = [RecenterTarget.Synthetic, RecenterTarget.Gimbal];

const offCentre = (...degrees: (number | null)[]): boolean => degrees.some((it) => it !== null && Math.abs(it) > OFF_CENTRE_DEG);

export const recenterTarget = (syntheticAims: boolean, syntheticTilt: number | null, syntheticPan: number | null, gimbal: GimbalIndicatorState | null): RecenterTarget | null =>
  syntheticAims
    ? offCentre(syntheticTilt === null ? null : syntheticTilt - DEFAULT_TILT_DEG, syntheticPan) ? RecenterTarget.Synthetic : null
    : gimbal !== null
      ? offCentre(gimbal.pitchDegrees, gimbal.yawDegrees) ? RecenterTarget.Gimbal : null
      : null;

export const recenter = (target: RecenterTarget, onRefused: (refusal: string | null) => void): void =>
  target === RecenterTarget.Synthetic
    ? void Qgc.invoke(SYNTHETIC_AIM, DEFAULT_TILT_DEG, 0.0)
    : void (async () => onRefused(gimbalRefusal(await Qgc.call(GIMBAL_CENTER))))();

export const tiltText = (degrees: number): string => `${roundToInt(degrees)}°`;

export const TiltReadout = ({ degrees, onRecenter, onTap = null, tapLabel = null }: { degrees: number; onRecenter: (() => void) | null; onTap?: (() => void) | null; tapLabel?: string | null }) => {
  const text = tiltText(degrees);
  const action = onRecenter ?? onTap;
  const description = onRecenter !== null ? `Tilt ${text}, off centre` : `Tilt ${text}`;
  const actionLabel = onRecenter !== null ? 'Recenter' : tapLabel;
  return (
    <Surface
      onClick={action ?? undefined}
      shape="full"
      color={onRecenter !== null ? withAlpha(Color.Black, RECENTER_SCRIM_ALPHA) : Color.Transparent}
      contentColor={aircast('outdoorForeground')}
      label={description}
      title={action !== null ? (actionLabel ?? undefined) : undefined}
      className="inline-flex items-center justify-center gap-1"
      style={{ minWidth: TILT_READOUT_WIDTH, height: TILT_READOUT_HEIGHT, paddingInline: TILT_READOUT_PADDING }}
    >
      {onRecenter !== null ? <Drawable name="ic_my_location" size={RECENTER_GLYPH} /> : null}
      <Text typography="labelMedium" className={onRecenter === null ? osdShadow() : undefined}>
        Tilt {text}
      </Text>
    </Surface>
  );
};
