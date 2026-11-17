import { listen } from '@tauri-apps/api/event';
import { useEffect, useState } from 'react';
import { type Json, jsonObjects, optBoolean, optJSONArray } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { sendCommand } from '@/core/desktopCommands';
import { AlertDialog, TextButton } from '@/m3';
import { useHasVehicle } from '@/ui/FlyState';

export const CLOSE_CHECKS = 'view.closeChecks';

export const leavingLosesWork = (view: Json | null): boolean => jsonObjects(optJSONArray(view, 'prompts')).some((it) => optBoolean(it, 'losesWork'));

export const leavingWarns = (view: Json | null, vehicle: boolean): boolean => vehicle || leavingLosesWork(view);

const BrowserCloseGuard = () => {
  const guarding = leavingWarns(useQgcPath(CLOSE_CHECKS), useHasVehicle());
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
    if (asked !== null) sendCommand('close_prompt_shown', {});
  }, [asked]);
  const keepWorking = () => {
    setAsked(null);
    sendCommand('close_prompt_dismissed', {});
  };
  if (asked === null) return null;
  return (
    <AlertDialog
      onDismissRequest={keepWorking}
      title="Close Aircast?"
      text={asked.join(' ')}
      confirmButton={<TextButton onClick={() => sendCommand('close_anyway', {})}>Close anyway</TextButton>}
      dismissButton={<TextButton onClick={keepWorking}>Keep working</TextButton>}
    />
  );
};

export const CloseGuard = __DESKTOP__ ? DesktopCloseGuard : BrowserCloseGuard;
