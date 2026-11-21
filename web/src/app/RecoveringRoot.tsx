import type { ReactNode } from 'react';
import { createRoot } from 'react-dom/client';
import { mutableStateOf } from '@/kotlin/State';
import { Button, Text } from '@/m3';

const REMOUNT_GAP_MS = 10_000;

export const CRASHED_TITLE = 'This screen stopped working';

const remountDue = (lastCrash: number, now: number): boolean => now - lastCrash >= REMOUNT_GAP_MS;

const oneLine = (trace: string | undefined): string =>
  (trace ?? '')
    .split('\n')
    .map((it) => it.trim())
    .filter((it) => it !== '')
    .join(' < ');

const crashLine = (error: unknown, componentStack: string | undefined): string =>
  `event=ui_crash reason="${String(error)}" stack="${oneLine(error instanceof Error ? error.stack : undefined)}" components="${oneLine(componentStack)}"`;

const Crashed = ({ reason }: { reason: string }) => (
  <div className="grid h-full place-content-center justify-items-center gap-4 p-6 text-center">
    <Text typography="titleMedium">{CRASHED_TITLE}</Text>
    <Text typography="bodyMedium">{reason}</Text>
    <Button onClick={() => location.reload()}>Reload</Button>
  </div>
);

export const recoveringRoot = (container: HTMLElement, log: (line: string) => void): ((node: ReactNode) => void) => {
  const shown = mutableStateOf<ReactNode>(null);
  const crashedAt = mutableStateOf(Number.NEGATIVE_INFINITY);
  const root = createRoot(container, {
    onUncaughtError: (error, { componentStack }) => {
      reportError(error);
      log(crashLine(error, componentStack));
      const now = performance.now();
      const again = !remountDue(crashedAt.value, now);
      crashedAt.value = now;
      root.render(again ? <Crashed reason={String(error)} /> : shown.value);
    },
  });
  return (node) => {
    shown.value = node;
    root.render(node);
  };
};
