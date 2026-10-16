import { useMemo } from 'react';
import { type Json, optBoolean, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, TextButton } from '@/m3';

export const DEEP_LINK_SETUP_PATH = 'view.deepLinkSetup';
const ACCEPT = 'deepLinkSetup.accept';
const DECLINE = 'deepLinkSetup.decline';

export type DeepLinkSetup = { host: string; title: string; text: string; accept: string; decline: string };

export const deepLinkSetup = (view: Json | null): DeepLinkSetup | null =>
  view !== null && optBoolean(view, 'show')
    ? { host: optText(view, 'host'), title: optText(view, 'title'), text: optText(view, 'text'), accept: optText(view, 'accept'), decline: optText(view, 'decline') }
    : null;

export const DeepLinkSetupPrompt = () => {
  const view = useQgcPath(DEEP_LINK_SETUP_PATH);
  const setup = useMemo(() => deepLinkSetup(view), [view]);
  if (setup === null) return null;
  const answer = (path: string) => void Qgc.invoke(path, setup.host);
  return (
    <AlertDialog
      onDismissRequest={() => answer(DECLINE)}
      title={setup.title}
      text={setup.text}
      confirmButton={<TextButton onClick={() => answer(ACCEPT)}>{setup.accept}</TextButton>}
      dismissButton={<TextButton onClick={() => answer(DECLINE)}>{setup.decline}</TextButton>}
    />
  );
};
