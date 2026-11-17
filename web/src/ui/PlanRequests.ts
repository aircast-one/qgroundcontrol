import { useEffect } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { type MutableState, useValue } from '@/kotlin/State';
import type { PlanRequest } from '@/ui/AppNavigation';

export type PlanRequestHandlers = Readonly<Record<PlanRequest, (() => void) | null>>;

export const planRequestRefusal = (request: PlanRequest): string => {
  switch (request) {
    case 'new':
      return 'A new plan cannot be started right now.';
    case 'open':
      return 'A plan cannot be opened right now.';
    case 'save':
    case 'saveAs':
      return 'This plan cannot be saved right now.';
  }
};

export const usePlanRequests = (requests: MutableState<PlanRequest | null>, ready: boolean, handlers: PlanRequestHandlers, onRefused: (message: string) => void): void => {
  const request = useValue(requests);
  const latest = useUpdatedRef({ handlers, onRefused });
  useEffect(() => {
    if (request === null || !ready) return;
    requests.value = null;
    const run = latest.current.handlers[request];
    if (run === null) latest.current.onRefused(planRequestRefusal(request));
    else run();
  }, [request, ready]);
};
