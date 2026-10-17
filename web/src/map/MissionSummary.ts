import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';

export const summaryRow = (view: Json | null, label: string): string | null => {
  const rows = optJSONArray(view, 'rows');
  if (rows === null) return null;
  const row = jsonObjects(rows).find((it) => optText(it, 'label') === label);
  const value = row === undefined ? null : optText(row, 'value');
  return value === null || value.trim() === '' ? null : value;
};

export const missionSummaryText = (view: Json | null): string =>
  [
    [summaryRow(view, 'Distance'), 'Distance'],
    [summaryRow(view, 'Time'), 'Time'],
    [summaryRow(view, 'Furthest from launch'), 'Max telem'],
  ]
    .flatMap(([value, label]) => (value === null ? [] : [`${label} ${value}`]))
    .join(' · ');
