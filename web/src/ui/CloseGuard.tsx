import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, TextButton } from '@/m3';

export const CLOSE_CHECKS = 'view.closeChecks';

const desktop: boolean = import.meta.env.DESKTOP === true;

const LOST_ON_LEAVING: ReadonlySet<string> = new Set(['unsavedMission', 'pendingWrites']);

export const leavingLosesWork = (view: Json | null): boolean => jsonObjects(optJSONArray(view, 'prompts')).some((it) => LOST_ON_LEAVING.has(optText(it, 'id')));

const BrowserCloseGuard = () => {
  const guarding = leavingLosesWork(useQgcPath(CLOSE_CHECKS));
  useEffect(() => {
    if (!guarding) return undefined;
    const warn = (event: BeforeUnloadEvent) => event.preventDefault();
    window.addEventListener('beforeunload', warn);
    return () => window.removeEventListener('beforeunload', warn);
  }, [guarding]);
  return null;
};

const DesktopCloseGuard = () => {
  const [lost, setLost] = useState<readonly string[] | null>(null);
  useEffect(() => {
    const stop = listen<string[]>('close-blocked', ({ payload }) => setLost(payload));
    return () => void stop.then((unlisten) => unlisten());
  }, []);
  if (lost === null) return null;
  return (
    <AlertDialog
      onDismissRequest={() => setLost(null)}
      title="Close Aircast?"
      text={lost.join(' ')}
      confirmButton={<TextButton onClick={() => void invoke('close_anyway')}>Close anyway</TextButton>}
      dismissButton={<TextButton onClick={() => setLost(null)}>Keep working</TextButton>}
    />
  );
};

export const CloseGuard = desktop ? DesktopCloseGuard : BrowserCloseGuard;
