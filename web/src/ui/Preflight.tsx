import { type Json, optBoolean } from '@/bridge/Json';

export const PREFLIGHT = 'view.preflight';

export const checklistOffered = (armed: boolean): string | null => (armed ? 'The checks are for before the flight' : null);

export const preflightOffered = (view: Json | null): boolean => (view === null ? true : optBoolean(view, 'offered', true));
