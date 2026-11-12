import { useState } from 'react';
import { type Json, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { OutlinedButton, Text } from '@/m3';

export const SIX_POS_VIEW = 'view.usbRadioSixPos';
const LEARN = 'usbRadio.learnSixPos';
const CANCEL = 'usbRadio.cancelSixPos';
const FORGET = 'usbRadio.forgetSixPos';

export type SixPos = { learning: boolean; heard: number; learned: boolean; blocked: string | null; text: string };

export const sixPosOf = (view: Json | null): SixPos | null => {
  if (view === null) return null;
  const blocked = optText(view, 'blocked');
  return {
    learning: optBoolean(view, 'learning'),
    heard: optInt(view, 'heard'),
    learned: optJSONArray(view, 'learned') !== null,
    blocked: blocked === '' ? null : blocked,
    text: optText(view, 'text'),
  };
};

export const SixPosSection = () => {
  const sixPos = sixPosOf(useQgcPath(SIX_POS_VIEW) as Json | null);
  const [refusal, setRefusal] = useState<string | null>(null);
  if (sixPos === null) return null;
  const run = (path: string) => void Qgc.refusalOf(path).then(setRefusal);
  const note = refusal ?? sixPos.blocked;
  return (
    <div className="flex w-full flex-col gap-1.5 px-5 py-2">
      <Text typography="titleSmall">6POS switch</Text>
      <Text typography="bodySmall">{sixPos.text}</Text>
      {note === null ? null : <Text typography="bodySmall">{note}</Text>}
      <div className="flex gap-2">
        {sixPos.learning ? (
          <OutlinedButton onClick={() => run(CANCEL)}>Stop learning</OutlinedButton>
        ) : (
          <OutlinedButton enabled={sixPos.blocked === null} onClick={() => run(LEARN)}>
            {sixPos.learned ? 'Learn again' : 'Learn 6POS'}
          </OutlinedButton>
        )}
        {!sixPos.learning && sixPos.learned ? <OutlinedButton onClick={() => run(FORGET)}>Forget</OutlinedButton> : null}
      </div>
    </div>
  );
};
