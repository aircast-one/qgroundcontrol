import { type ReactNode, useMemo, useRef, useState } from 'react';
import { Tooltip } from '@base-ui/react';

export type SnackbarResult = 'Dismissed' | 'ActionPerformed';
export type SnackbarDuration = 'Short' | 'Long' | 'Indefinite';
type Shown = { message: string; actionLabel: string | null; settle: (result: SnackbarResult) => void; id: number };

export type SnackbarHostState = {
  current: Shown | null;
  showSnackbar: (message: string, actionLabel?: string | null, duration?: SnackbarDuration, signal?: AbortSignal) => Promise<SnackbarResult>;
};

const DURATION_MS: Record<SnackbarDuration, number | null> = { Short: 4000, Long: 10000, Indefinite: null };

export const useSnackbarHostState = (): SnackbarHostState => {
  const [current, setCurrent] = useState<Shown | null>(null);
  const queue = useRef<Promise<unknown>>(Promise.resolve());
  return useMemo(() => {
    const display = (message: string, actionLabel: string | null, duration: SnackbarDuration, signal: AbortSignal | undefined) => new Promise<SnackbarResult>((resolve) => {
      if (signal?.aborted === true) return resolve('Dismissed');
      const id = Date.now() + Math.random();
      const settled = { done: false };
      const settle = (result: SnackbarResult) => {
        if (settled.done) return;
        settled.done = true;
        setCurrent((shown) => (shown?.id === id ? null : shown));
        resolve(result);
      };
      setCurrent({ message, actionLabel, settle, id });
      const timeout = DURATION_MS[duration];
      if (timeout !== null) setTimeout(() => settle('Dismissed'), timeout);
      signal?.addEventListener('abort', () => settle('Dismissed'), { once: true });
    });
    return {
      current,
      showSnackbar: (message, actionLabel = null, duration = actionLabel ? 'Indefinite' : 'Short', signal) => {
        const shown = queue.current.then(() => display(message, actionLabel, duration, signal));
        queue.current = shown.catch(() => undefined);
        return shown;
      },
    };
  }, [current]);
};

export const SnackbarHost = ({ hostState, className }: { hostState: SnackbarHostState; className?: string }) => {
  const shown = hostState.current;
  return shown ? (
    <div role="status" aria-live="polite" className={`pointer-events-auto mx-auto flex min-h-12 w-[min(560px,calc(100vw-32px))] items-center gap-2 rounded-shape-xs bg-inverse-surface px-4 py-2 text-body-medium text-inverse-on-surface shadow-lg ${className ?? ''}`}>
      <span className="flex-1">{shown.message}</span>
      {shown.actionLabel ? (
        <button type="button" className="h-9 rounded-full px-3 text-label-large text-[var(--md-inverse-primary,var(--md-primary-container))]" onClick={() => shown.settle('ActionPerformed')}>
          {shown.actionLabel}
        </button>
      ) : null}
    </div>
  ) : null;
};

export const TooltipBox = ({ tooltip, children }: { tooltip: ReactNode; children: ReactNode }) => (
  <Tooltip.Provider>
    <Tooltip.Root>
      <Tooltip.Trigger render={<span className="inline-flex" />}>{children}</Tooltip.Trigger>
      <Tooltip.Portal>
        <Tooltip.Positioner sideOffset={6} className="z-50">
          <Tooltip.Popup className="max-w-[280px] rounded-shape-xs bg-inverse-surface px-2 py-1 text-body-small text-inverse-on-surface">{tooltip}</Tooltip.Popup>
        </Tooltip.Positioner>
      </Tooltip.Portal>
    </Tooltip.Root>
  </Tooltip.Provider>
);
