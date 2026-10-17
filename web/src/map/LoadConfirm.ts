import { itemCountText } from './PlanItems';

export type LoadStep = { readonly name: 'Confirm' | 'Load'; readonly ordinal: number };

export const LoadStep = {
  Confirm: { name: 'Confirm', ordinal: 0 },
  Load: { name: 'Load', ordinal: 1 },
} as const satisfies Record<string, LoadStep>;

export const LoadStepEntries: readonly LoadStep[] = [LoadStep.Confirm, LoadStep.Load];

export const loadStep = (dirty: boolean, armed: boolean): LoadStep => (dirty && !armed ? LoadStep.Confirm : LoadStep.Load);

export const replaceWarning = (items: number): string => `Your unsaved plan here (${itemCountText(items)}) will be replaced.`;
