import { useMemo } from 'react';
import { useProduceState } from '@/kotlin/Compose';
import { coerceAtLeast, delay } from '@/kotlin/Std';

const MILLIS_PER_SECOND = 1000;
const SECONDS_PER_MINUTE = 60;

const elapsedRealtime = (): number => performance.now();

export const useSilentSeconds = (lost: boolean): number | null => {
  const since = useMemo(() => (lost ? elapsedRealtime() : null), [lost]);
  const now = useProduceState(elapsedRealtime(), [since], async (set, scope) => {
    const tick = async (): Promise<void> => {
      if (since === null || !scope.isActive()) return;
      set(elapsedRealtime());
      await delay(MILLIS_PER_SECOND);
      return tick();
    };
    await tick();
  });
  return since === null ? null : Math.trunc(coerceAtLeast(now - since, 0) / MILLIS_PER_SECOND);
};

export const silenceDuration = (seconds: number): string =>
  seconds < SECONDS_PER_MINUTE ? `${seconds} s` : `${Math.trunc(seconds / SECONDS_PER_MINUTE)} min`;

export const lastSeenText = (seconds: number): string => `Last seen ${silenceDuration(seconds)} ago`;
