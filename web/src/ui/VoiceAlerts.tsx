import { useEffect, useMemo, useRef } from 'react';
import { type Json, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { SpeechOut } from '@/app/SpeechOut';
import { FLY_STATE, flyState } from '@/ui/FlyState';
import { BATTERY_VIEW, RETURN_NOW_CLIP, RETURN_NOW_SPOKEN, batteryHeadline } from '@/ui/InstrumentDetail';

export const SPEECH_VIEW = 'view.speech';
const LOST_BUZZ: readonly number[] = [0, 400, 150, 400, 150, 400];
const REGAINED_BUZZ: readonly number[] = [0, 120];

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

export type ReturnAlert = { speak: boolean; alerted: boolean };

export const returnAlert = (alerted: boolean, returnNow: boolean, flying: boolean): ReturnAlert => ({ speak: returnNow && !alerted, alerted: flying && (alerted || returnNow) });

const buzz = (pattern: readonly number[]) => {
  if ('vibrate' in navigator) navigator.vibrate([0, ...pattern]);
};

export const VoiceAlerts = () => (
  <>
    <LinkLossBuzz />
    <ReturnHomeAlert />
  </>
);

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
  const flyJson = useQgcPath(FLY_STATE);
  const lost = flyState(flyJson)?.contactLost === true;
  const heard = useRef<boolean | null>(null);
  useEffect(() => {
    if (heard.current === lost) return;
    if (heard.current !== null || lost) buzz(linkBuzz(lost));
    heard.current = lost;
  }, [lost]);
  return null;
};
