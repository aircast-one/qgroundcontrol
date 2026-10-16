import { useEffect, useMemo, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcBool, useQgcPath } from '@/bridge/QgcState';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { type MutableState, mutableStateOf, useValue } from '@/kotlin/State';
import { delay } from '@/kotlin/Std';
import { AlertDialog, TextButton } from '@/m3';
import { VEHICLES_VIEW } from '@/map/VehiclePicker';
import { activeVehicleId } from './ActiveVehicle';
import { CHECKLIST_POPUP_DELAY_MS, PREFLIGHT, checklistIsComplete, checklistPopupIsDue, preflight, preflightOffered } from './Preflight';
import { PreflightScreen } from './PreflightScreen';
import { checklistStateValue } from './VehicleUi';

const CHECKLIST_CLOSE_DELAY_MS = 1000;

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

export const PreflightChecklistReset = ({ checklist, available }: { checklist: PreflightChecklistState; available: boolean }) => {
  useEffect(() => {
    if (!available) {
      checklist.ticked.value = new Set();
      checklist.popupShownFor.value = null;
    }
  }, [available]);
  return null;
};

export const PreflightChecklist = ({ checklist, deciding }: { checklist: PreflightChecklistState; deciding: boolean }) => {
  const enforceChecklist = useQgcBool(settingControl('settings.appSettings.enforceChecklist'));
  const enforceNow = useUpdatedRef(enforceChecklist);
  const preflightJson = useQgcPath(PREFLIGHT);
  const offered = useMemo(() => preflightOffered(preflightJson), [preflightJson]);
  const checks = useMemo(() => preflight(preflightJson), [preflightJson]);
  const vehiclesJson = useQgcPath(VEHICLES_VIEW);
  const vehicleId = useMemo(() => activeVehicleId(vehiclesJson), [vehiclesJson]);
  const ticked = useValue(checklist.ticked);
  const shown = useValue(checklist.shown);
  useEffect(() => {
    checklist.ticked.value = new Set();
    checklist.stateSent.value = null;
  }, [vehicleId]);
  const passed = checklistIsComplete(checks, ticked);
  useLaunchedEffect([passed], async (scope) => {
    if (passed && checklist.shown.value) {
      await delay(CHECKLIST_CLOSE_DELAY_MS);
      if (scope.isActive()) checklist.shown.value = false;
    }
  });
  useLaunchedEffect([passed, vehicleId], async () => {
    if (vehicleId === null || checklist.stateSent.value === passed) return;
    if (checklist.stateSent.value === null && !passed) {
      checklist.stateSent.value = false;
      return;
    }
    checklist.stateSent.value = passed;
    await VehicleCommands.setChecklistState(checklistStateValue(passed));
  });
  useLaunchedEffect([vehicleId, deciding], async (scope) => {
    const id = vehicleId;
    if (id === null) return;
    if (checklist.popupShownFor.value === id || deciding) return;
    await delay(CHECKLIST_POPUP_DELAY_MS);
    if (!scope.isActive()) return;
    const complete = checklistIsComplete(preflight(await Qgc.get(PREFLIGHT)), checklist.ticked.value);
    if (!scope.isActive()) return;
    if (checklistPopupIsDue(true, offered, enforceNow.current, complete, deciding)) {
      checklist.popupShownFor.value = id;
      checklist.shown.value = true;
    }
  });
  if (!shown) return null;
  return (
    <AlertDialog
      onDismissRequest={() => {
        checklist.shown.value = false;
      }}
      title="Pre-flight checklist"
      text={
        <PreflightScreen
          className="w-full"
          ticked={ticked}
          onTicked={(it) => {
            checklist.ticked.value = it;
          }}
        />
      }
      dismissButton={
        <TextButton
          onClick={() => {
            checklist.shown.value = false;
          }}
        >
          Close
        </TextButton>
      }
    />
  );
};
