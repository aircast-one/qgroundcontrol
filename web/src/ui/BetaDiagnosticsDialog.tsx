import { useMemo } from 'react';
import { AppCommands } from '@/bridge/BridgeCommands';
import { type Json, optBoolean, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Button, TextButton } from '@/m3';

export const BETA_DIAGNOSTICS_PATH = 'view.betaDiagnostics';

export type BetaDiagnostics = {
  title: string;
  text: string;
  accept: string;
  dismiss: string;
  endpoint: string;
};

export const betaDiagnostics = (view: Json | null): BetaDiagnostics | null =>
  view !== null && optBoolean(view, 'show')
    ? {
        title: optText(view, 'title'),
        text: optText(view, 'text'),
        accept: optText(view, 'accept'),
        dismiss: optText(view, 'dismiss'),
        endpoint: optText(view, 'endpoint'),
      }
    : null;

export const BetaDiagnosticsDialog = () => {
  const view = useQgcPath(BETA_DIAGNOSTICS_PATH);
  const prompt = useMemo(() => betaDiagnostics(view), [view]);
  if (prompt === null) return null;
  const accept = () => void AppCommands.acceptBetaDiagnostics();
  const dismiss = () => void AppCommands.dismissBetaDiagnostics();

  return (
    <AlertDialog
      onDismissRequest={dismiss}
      title={prompt.title}
      text={prompt.text}
      confirmButton={<Button onClick={accept}>{prompt.accept}</Button>}
      dismissButton={<TextButton onClick={dismiss}>{prompt.dismiss}</TextButton>}
    />
  );
};
