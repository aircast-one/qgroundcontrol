import { useState } from 'react';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { AlertDialog, DropdownMenu, DropdownMenuItem, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { md } from '@/theme/Colors';

export const SET_RC_TO_PARAM = 'parameters.setRcToParam';
const OPEN_RC_TO_PARAM = 'parameters.openRcToParam';
const RC_TO_PARAM_READY = 'parameters.rcToParamReady';
const READY_POLL_MS = 250;
const DEFAULT_RC_SCALE = '1.0';
const TUNING_IDS = [1, 2, 3];

export type RcToParam = { scale: number; center: number; tuningIndex: number; min: number; max: number };

export const rcToParam = (scale: string, center: string, tuningIndex: number, min: string, max: string): RcToParam | null => {
  const numbers = [scale, center, min, max].map((it) => toDoubleOrNull(it.trim()));
  if (numbers.some((it) => it === null)) return null;
  const [s = 0, c = 0, lo = 0, hi = 0] = numbers as number[];
  return { scale: s, center: c, tuningIndex, min: lo, max: hi };
};

export const RcToParamDialog = ({ fact, onDismiss }: { fact: Fact; onDismiss: () => void }) => {
  const [scale, setScale] = useState(DEFAULT_RC_SCALE);
  const [center, setCenter] = useState(() => (typeof fact.value === 'number' ? String(fact.value) : fact.valueString));
  const [min, setMin] = useState(fact.minString);
  const [max, setMax] = useState(fact.maxString);
  const [tuningIndex, setTuningIndex] = useState(0);
  const [choosing, setChoosing] = useState(false);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [ready, setReady] = useState(false);

  useLaunchedEffect([fact.name], async (scope) => {
    await Qgc.invoke(OPEN_RC_TO_PARAM, fact.name);
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const answer = await Qgc.invokeResult(RC_TO_PARAM_READY, fact.name);
      const now = typeof answer === 'boolean' ? answer : true;
      if (!scope.isActive()) return;
      setReady(now);
      if (now) return;
      await delay(READY_POLL_MS);
      return poll();
    };
    await poll();
  });

  const entered = rcToParam(scale, center, tuningIndex, min, max);
  const fields: readonly (readonly [string, string, (value: string) => void])[] = [
    ['Scale', scale, setScale],
    ['Center Value', center, setCenter],
    ['Min Value', min, setMin],
    ['Max Value', max, setMax],
  ];
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="RC To Param"
      text={
        <div className="flex flex-col gap-2">
          <Text typography="bodySmall">Bind an RC Channel to a parameter value. Tuning IDs can be mapped to an RC Channel from Radio Setup page.</Text>
          {!ready ? <Text>Waiting on parameter update from Vehicle.</Text> : null}
          <Text className="whitespace-pre">{`Parameter  ${fact.name}`}</Text>
          <div className="relative self-start">
            <OutlinedButton enabled={ready} onClick={() => setChoosing(true)}>{`Tuning ID ${TUNING_IDS[tuningIndex] ?? ''}`}</OutlinedButton>
            <DropdownMenu expanded={choosing} onDismissRequest={() => setChoosing(false)}>
              {TUNING_IDS.map((id, index) => (
                <DropdownMenuItem
                  key={id}
                  text={`${id}`}
                  onClick={() => {
                    setTuningIndex(index);
                    setChoosing(false);
                  }}
                />
              ))}
            </DropdownMenu>
          </div>
          {fields.map(([label, value, onChange]) => (
            <OutlinedTextField key={label} value={value} onValueChange={onChange} enabled={ready} label={label} keyboardType="decimal" />
          ))}
          <Text typography="bodySmall">Double check that all values are correct prior to confirming dialog.</Text>
          {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}
        </div>
      }
      confirmButton={
        <TextButton
          enabled={entered !== null}
          onClick={() => {
            if (entered === null) return;
            void (async () => {
              const refused = await Qgc.refusalOf(SET_RC_TO_PARAM, fact.name, entered.scale, entered.center, entered.tuningIndex, entered.min, entered.max);
              setRefusal(refused);
              if (refused === null) onDismiss();
            })();
          }}
        >
          OK
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};
