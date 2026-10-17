import { useEffect } from 'react';
import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';

export const CLOSE_CHECKS = 'view.closeChecks';

const LOST_ON_LEAVING: ReadonlySet<string> = new Set(['unsavedMission', 'pendingWrites']);

export const leavingLosesWork = (view: Json | null): boolean => jsonObjects(optJSONArray(view, 'prompts')).some((it) => LOST_ON_LEAVING.has(optText(it, 'id')));

export const CloseGuard = () => {
  const guarding = leavingLosesWork(useQgcPath(CLOSE_CHECKS));
  useEffect(() => {
    if (!guarding) return undefined;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [guarding]);
  return null;
};
