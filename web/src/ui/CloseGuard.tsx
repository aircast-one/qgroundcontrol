import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, jsonObjects, optJSONArray, optText } from '@/bridge/Json';
import { AlertDialog, TextButton } from '@/m3';
import { useBackHandler } from '@/kotlin/Compose';

export const CLOSE_CHECKS = 'view.closeChecks';

const APP_NAME = 'Aircast';

export const closePrompts = (view: Json | null): string[] =>
  jsonObjects(optJSONArray(view, 'prompts'))
    .map((it) => optText(it, 'message'))
    .filter((it) => it.trim() !== '');

export const CloseGuard = ({ enabled }: { enabled: boolean }) => {
  const [pending, setPending] = useState<readonly string[] | null>(null);
  const advance = (remaining: readonly string[]) => {
    if (remaining.length > 0) return setPending(remaining);
    setPending(null);
    window.close();
  };
  useBackHandler(enabled && pending === null, () => {
    void (async () => advance(closePrompts(await Qgc.get(CLOSE_CHECKS))))();
  });
  if (pending === null) return null;
  return (
    <AlertDialog
      onDismissRequest={() => setPending(null)}
      title={`Close ${APP_NAME}`}
      text={pending[0]}
      confirmButton={<TextButton onClick={() => advance(pending.slice(1))}>Yes</TextButton>}
      dismissButton={<TextButton onClick={() => setPending(null)}>No</TextButton>}
    />
  );
};
