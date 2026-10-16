import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';

export const summaryRow = (view: Json | null, label: string): string | null => {
  const rows = optJSONArray(view, 'rows');
  if (rows === null) return null;
  const row = jsonObjects(rows).find((it) => optText(it, 'label') === label);
  const value = row === undefined ? null : optText(row, 'value');
  return value === null || value.trim() === '' ? null : value;
};
