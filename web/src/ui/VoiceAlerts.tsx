import { useEffect, useMemo, useRef } from 'react';
import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcDerived, useQgcPath } from '@/bridge/QgcState';
import { SpeechOut } from '@/app/SpeechOut';
import { useValue } from '@/kotlin/State';
import { lostVehicle } from '@/map/LostVehicle';
import { FLY_STATE, flyState } from '@/ui/FlyState';
import { ESC_PATH } from '@/ui/EscIndicator';
import { BATTERY_VIEW, RETURN_NOW_CLIP, RETURN_NOW_SPOKEN, SIGNAL_LOST, batteryHeadline } from '@/ui/InstrumentDetail';
import { motorLoads } from '@/ui/MotorLoadWidget';

export const SPEECH_VIEW = 'view.speech';
const LOST_BUZZ: readonly number[] = [0, 400, 150, 400, 150, 400];
const REGAINED_BUZZ: readonly number[] = [0, 120];
export const SIGNAL_LOST_CLIP = 'signal_lost';

export type SpokenLine = { sequence: number; text: string; clip: string; volume: number };

export type SpeechBatch = { last: number; muted: boolean; lines: readonly SpokenLine[] };

export const speechBatch = (view: Json | null): SpeechBatch | null =>
  view !== null && optText(view, 'class') === 'Speech'
    ? {
        last: optInt(view, 'last'),
        muted: optBoolean(view, 'muted'),
        lines: jsonObjects(optJSONArray(view, 'lines')).map((it) => ({ sequence: optInt(it, 'sequence'), text: optText(it, 'text'), clip: optText(it, 'clip'), volume: optDouble(it, 'volume', 1) })),
      }
    : null;

export const speechPath = (after: number): string => `${SPEECH_VIEW}(${after})`;

export const linkBuzz = (lost: boolean): readonly number[] => (lost ? LOST_BUZZ : REGAINED_BUZZ);

export type LinkAlert = { readonly heard: boolean | null; readonly buzz: readonly number[] | null; readonly speak: boolean };

export const linkAlert = (heard: boolean | null, connected: boolean, contactLost: boolean, dropped: boolean): LinkAlert => {
  if (!connected && !dropped) return { heard, buzz: null, speak: false };
  const lost = contactLost || dropped;
  if (heard === lost || (heard === null && !lost)) return { heard: lost, buzz: null, speak: false };
  return { heard: lost, buzz: linkBuzz(lost), speak: lost && !contactLost };
};

export type ReturnAlert = { speak: boolean; alerted: boolean };

export const returnAlert = (alerted: boolean, returnNow: boolean, flying: boolean): ReturnAlert => ({ speak: returnNow && !alerted, alerted: flying && (alerted || returnNow) });

export const lostMotors = (view: Json | null): string =>
  motorLoads(view)
    .filter((load) => load.state === 'lost')
    .map((load) => load.motor)
    .join(',');

export type MotorLossAlert = { heard: readonly number[]; spoken: string | null };

export const motorLossAlert = (heard: readonly number[], lost: readonly number[], flying: boolean): MotorLossAlert => {
  const fresh = lost.filter((motor) => !heard.includes(motor));
  return {
    heard: lost,
    spoken: !flying || fresh.length === 0 ? null : fresh.length === 1 ? `Motor ${fresh[0]} telemetry lost` : `Motors ${fresh.join(', ')} telemetry lost`,
  };
};

const buzz = (pattern: readonly number[]) => {
  if ('vibrate' in navigator) navigator.vibrate([0, ...pattern]);
};

export const VoiceAlerts = () => (
  <>
    <LinkLossBuzz />
    <ReturnHomeAlert />
    <MotorLossVoice />
  </>
);

const MotorLossVoice = () => {
  const lost = useQgcDerived(ESC_PATH, lostMotors);
  const flying = optBoolean(useQgcPath(FLY_STATE), 'flying');
  const heard = useRef<readonly number[]>([]);
  useEffect(() => {
    const alert = motorLossAlert(heard.current, lost === '' ? [] : lost.split(',').map(Number), flying);
    heard.current = alert.heard;
    if (alert.spoken === null) return;
    SpeechOut.say(alert.spoken, '');
    buzz(linkBuzz(true));
  }, [lost, flying]);
  return null;
};

const ReturnHomeAlert = () => {
  const batteryJson = useQgcPath(BATTERY_VIEW);
  const flyJson = useQgcPath(FLY_STATE);
  const returnNow = useMemo(() => batteryHeadline(batteryJson)?.returnNow === true, [batteryJson]);
  const flying = optBoolean(flyJson, 'flying');
  const alerted = useRef(false);
  const handled = useRef<string | null>(null);
  useEffect(() => {
    const keys = `${returnNow}:${flying}`;
    if (handled.current === keys) return;
    handled.current = keys;
    const next = returnAlert(alerted.current, returnNow, flying);
    alerted.current = next.alerted;
    if (!next.speak) return;
    SpeechOut.say(RETURN_NOW_SPOKEN, RETURN_NOW_CLIP);
    buzz(linkBuzz(true));
  }, [returnNow, flying]);
  return null;
};

const LinkLossBuzz = () => {
  const fly = flyState(useQgcPath(FLY_STATE));
  const connected = fly?.connected === true;
  const contactLost = fly?.contactLost === true;
  const dropped = useValue(lostVehicle) !== null;
  const heard = useRef<boolean | null>(null);
  useEffect(() => {
    const alert = linkAlert(heard.current, connected, contactLost, dropped);
    heard.current = alert.heard;
    if (alert.buzz !== null) buzz(alert.buzz);
    if (alert.speak) SpeechOut.say(SIGNAL_LOST, SIGNAL_LOST_CLIP);
  }, [connected, contactLost, dropped]);
  return null;
};
