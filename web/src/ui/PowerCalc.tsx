import { type Json, optInt, optText } from '@/bridge/Json';

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

export const PowerCalcDialog = (_props: { calculator: PowerCalculator; onDone: () => void }) => null;
