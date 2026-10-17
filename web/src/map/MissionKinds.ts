import { call } from '@/core/client';
import { type Json, optBoolean, optText, parseJson } from '@/bridge/Json';

export const AT_END = -1;
export const HOME_ITEM = 0;
export const BEFORE_THE_REST = 1;

export type InsertOutcome = { ok: boolean; reason: string; index: number | null };

export const InsertOutcome = (fields: { ok: boolean; reason: string; index?: number | null }): InsertOutcome => ({ index: null, ...fields });

const NO_ANSWER = 'The plan did not answer.';

export const insertOutcome = (view: Json | null): InsertOutcome => {
  if (view === null) return InsertOutcome({ ok: false, reason: NO_ANSWER });
  if (optBoolean(view, 'ok')) {
    const index = view.index;
    return InsertOutcome({ ok: true, reason: '', index: typeof index === 'number' ? Math.trunc(index) : null });
  }
  const reason = optText(view, 'reason');
  return InsertOutcome({ ok: false, reason: reason.trim() === '' ? NO_ANSWER : reason });
};

const invoked = async (path: string, args: readonly unknown[]): Promise<InsertOutcome> => {
  try {
    const text = await call<string>('invoke', path, JSON.stringify(args));
    return insertOutcome(parseJson(text));
  } catch {
    return InsertOutcome({ ok: false, reason: NO_ANSWER });
  }
};

export const insertMissionItem = (kind: string, latitude: number, longitude: number, index: number): Promise<InsertOutcome> =>
  invoked('mission.insert', [kind, latitude, longitude, index]);

export const removeMissionItem = (index: number): Promise<InsertOutcome> => invoked('mission.remove', [index]);
