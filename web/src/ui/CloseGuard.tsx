import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optJSONArray } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, TextButton } from '@/m3';

export const CLOSE_CHECKS = 'view.closeChecks';

export const leavingLosesWork = (view: Json | null): boolean => jsonObjects(optJSONArray(view, 'prompts')).some((it) => optBoolean(it, 'losesWork'));

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
  const [asked, setAsked] = useState<readonly string[] | null>(null);
  useEffect(() => {
    const stop = listen<string[]>('close-blocked', ({ payload }) => setAsked(payload));
    return () => void stop.then((unlisten) => unlisten());
  }, []);
  useEffect(() => {
    if (asked !== null) void invoke('close_prompt_shown');
  }, [asked]);
  const keepWorking = () => {
    setAsked(null);
    void invoke('close_prompt_dismissed');
  };
  if (asked === null) return null;
  return (
    <AlertDialog
      onDismissRequest={keepWorking}
      title="Close Aircast?"
      text={asked.join(' ')}
      confirmButton={<TextButton onClick={() => void invoke('close_anyway')}>Close anyway</TextButton>}
      dismissButton={<TextButton onClick={keepWorking}>Keep working</TextButton>}
    />
  );
};

export const CloseGuard = __DESKTOP__ ? DesktopCloseGuard : BrowserCloseGuard;
