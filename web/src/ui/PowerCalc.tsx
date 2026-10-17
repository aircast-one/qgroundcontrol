import { useState } from 'react';
import { type Json, optBoolean, optInt, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { AlertDialog, OutlinedTextField, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { md } from '@/theme/Colors';

const POWER_CALC_VIEW = 'view.powerCalc';
const POWER_CALCULATE = 'vehicleConfig.calculate';
const READING_POLL_MS = 1000;

export type PowerCalculator = {
  title: string;
  help: string;
  measure: string;
  measuredLabel: string;
  readingLabel: string;
  paramLabel: string;
  button: string;
  noReading: string;
  batteryIndex: number;
  param: string;
};

export const powerCalculator = (json: Json | null): PowerCalculator | null =>
  json === null || optText(json, 'param').trim() === ''
    ? null
    : {
        title: optText(json, 'title'),
        help: optText(json, 'help'),
        measure: optText(json, 'measure'),
        measuredLabel: optText(json, 'measuredLabel'),
        readingLabel: optText(json, 'readingLabel'),
        paramLabel: optText(json, 'paramLabel'),
        button: optText(json, 'button'),
        noReading: optText(json, 'noReading'),
        batteryIndex: optInt(json, 'batteryIndex'),
        param: optText(json, 'param'),
      };

export const powerCalcPath = (calculator: PowerCalculator): string => `${POWER_CALC_VIEW}(${calculator.measure},${calculator.batteryIndex},${calculator.param})`;

export const PowerCalcDialog = ({ calculator, onDone }: { calculator: PowerCalculator; onDone: () => void }) => {
  const [measured, setMeasured] = useState('');
  const [live, setLive] = useState<Json | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const path = powerCalcPath(calculator);

  useLaunchedEffect([path], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const next = await Qgc.get(path);
      if (!scope.isActive()) return;
      setLive(next);
      await delay(READING_POLL_MS);
      return loop();
    };
    await loop();
  });

  const available = optBoolean(live, 'readingAvailable');
  return (
    <AlertDialog
      onDismissRequest={onDone}
      title={sentenceCase(calculator.title)}
      text={
        <div className="flex flex-col gap-2">
          <Text typography="bodySmall">{calculator.help}</Text>
          {!available && calculator.noReading.trim() !== '' ? (
            <Text color={md('error')} typography="bodySmall">
              {calculator.noReading}
            </Text>
          ) : null}
          <OutlinedTextField value={measured} onValueChange={setMeasured} label={calculator.measuredLabel} singleLine keyboardType="decimal" />
          {available || calculator.noReading.trim() === '' ? <Text>{`${calculator.readingLabel} ${optText(live, 'readingText')}`}</Text> : null}
          <Text>{`${calculator.paramLabel} ${optText(live, 'paramText')}`}</Text>
          {refusal !== null ? (
            <Text color={md('error')} typography="bodySmall">
              {refusal}
            </Text>
          ) : null}
        </div>
      }
      confirmButton={
        <TextButton
          enabled={available}
          onClick={() =>
            void (async () => {
              setRefusal(await Qgc.refusalOf(POWER_CALCULATE, calculator.param, calculator.measure, calculator.batteryIndex, measured));
            })()
          }
        >
          {sentenceCase(calculator.button)}
        </TextButton>
      }
      dismissButton={<TextButton onClick={onDone}>Close</TextButton>}
    />
  );
};
