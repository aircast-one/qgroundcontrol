import { useState } from 'react';
import { type MutableState, mutableStateOf } from '@/kotlin/State';

export class PreflightChecklistState {
  readonly ticked: MutableState<ReadonlySet<string>>;
  readonly shown = mutableStateOf(false);
  readonly popupShownFor = mutableStateOf<number | null>(null);
  readonly stateSent = mutableStateOf<boolean | null>(null);

  constructor(tickedState: MutableState<ReadonlySet<string>>) {
    this.ticked = tickedState;
  }

  open(): void {
    this.shown.value = true;
  }
}

export const useRememberPreflightChecklist = (): PreflightChecklistState => useState(() => new PreflightChecklistState(mutableStateOf<ReadonlySet<string>>(new Set())))[0];

export const PreflightChecklistReset = (_props: { checklist: PreflightChecklistState; available: boolean }) => null;

export const PreflightChecklist = (_props: { checklist: PreflightChecklistState; deciding: boolean }) => null;
