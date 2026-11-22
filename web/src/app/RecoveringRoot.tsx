import { type ReactNode, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { mutableStateOf } from '@/kotlin/State';
import { Button, OutlinedButton, Text } from '@/m3';

const REMOUNT_GAP_MS = 10_000;

export const CRASHED_TITLE = 'This screen stopped working';
export const RELOAD_KEEPS_LINK = 'Reload draws it again. Your vehicle stays connected.';
export const RELOAD_RESTARTS = 'Reload starts the app again.';

export type PageFacts = { readonly screen: () => string; readonly desktop: boolean; readonly version: string; readonly sha: string };

export type PageErrorKind = 'render' | 'error' | 'rejection';

const remountDue = (lastCrash: number, now: number): boolean => now - lastCrash >= REMOUNT_GAP_MS;

const oneLine = (trace: string | undefined): string =>
  (trace ?? '')
    .split('\n')
    .map((it) => it.trim())
    .filter((it) => it !== '')
    .join(' < ');

const quoted = (text: string): string => `"${text.replaceAll('"', "'")}"`;

export const pageErrorLine = (kind: PageErrorKind, error: unknown, componentStack: string | undefined, facts: PageFacts, uptimeMs: number): string =>
  [
    'event=page_error',
    `kind=${kind}`,
    `reason=${quoted(String(error))}`,
    `stack=${quoted(oneLine(error instanceof Error ? error.stack : undefined))}`,
    `components=${quoted(oneLine(componentStack))}`,
    `screen=${facts.screen()}`,
    `head=${facts.desktop ? 'desktop' : 'browser'}`,
    `version=${facts.version}`,
    `sha=${facts.sha}`,
    `uptime_s=${(uptimeMs / 1000).toFixed(1)}`,
  ].join(' ');

const Crashed = ({ reason, details, desktop }: { reason: string; details: string; desktop: boolean }) => {
  const [copied, setCopied] = useState(false);
  return (
    <div className="grid h-full place-content-center justify-items-center gap-4 p-6 text-center">
      <Text typography="titleMedium">{CRASHED_TITLE}</Text>
      <Text typography="bodyMedium">{desktop ? RELOAD_KEEPS_LINK : RELOAD_RESTARTS}</Text>
      <Text typography="bodySmall" className="max-w-xl break-words text-on-surface-variant">
        {reason}
      </Text>
      <div className="flex gap-2">
        <Button onClick={() => location.reload()}>Reload</Button>
        <OutlinedButton onClick={() => void navigator.clipboard?.writeText(details).then(() => setCopied(true))}>{copied ? 'Copied' : 'Copy details'}</OutlinedButton>
      </div>
    </div>
  );
};

export const recordPageErrors = (log: (line: string) => void, facts: PageFacts): void => {
  window.addEventListener('error', (event) => log(pageErrorLine('error', event.error ?? event.message, undefined, facts, performance.now())));
  window.addEventListener('unhandledrejection', (event) => log(pageErrorLine('rejection', event.reason, undefined, facts, performance.now())));
};

export const recoveringRoot = (container: HTMLElement, log: (line: string) => void, facts: PageFacts): ((node: ReactNode) => void) => {
  const shown = mutableStateOf<ReactNode>(null);
  const crashedAt = mutableStateOf(Number.NEGATIVE_INFINITY);
  const root = createRoot(container, {
    onUncaughtError: (error, { componentStack }) => {
      console.error(error);
      const now = performance.now();
      const line = pageErrorLine('render', error, componentStack, facts, now);
      log(line);
      const again = !remountDue(crashedAt.value, now);
      crashedAt.value = now;
      root.render(again ? <Crashed reason={String(error)} details={line} desktop={facts.desktop} /> : shown.value);
    },
  });
  return (node) => {
    shown.value = node;
    root.render(node);
  };
};
