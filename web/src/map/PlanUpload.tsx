import type { ReactNode } from 'react';
import { type Json, optBoolean, optJSONObject, optText } from '@/bridge/Json';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Button, TextButton } from '@/m3';
import { useMapPath } from './MapBridge';
import { PlanBridge } from './PlanBridge';
import { FAILURE_MESSAGE_MS, type PlanFeedback } from './PlanEditing';
import { repositionMission } from './PlanStrip';
import { useRemembered } from './Remembered';
import { UploadOutcome, uploadMessage, uploadOutcome } from './UploadOutcome';
import { VEHICLES_VIEW, uploadHeading, vehicleChoices } from './VehiclePicker';
import type { TrackPoint } from './VehicleMap';
import { PlanFocus, UPLOADED, type UploadGate, freshPlanView, notReadyToSend, syncRefusal, uploadGate, uploadLabel, uploadStep, vehicleSyncState } from './VehicleSync';

const CONVERTED = 'Plan converted to this vehicle. Check it, then upload.';
const FAR_FROM_AIRCRAFT_TEXT = 'The aircraft is not where this route begins. Move the plan, with its fence and rally points, so it starts at the aircraft, or upload it as it is.';

export type PlanUpload = {
  readonly enabled: boolean;
  readonly emphasised: boolean;
  readonly label: string;
  readonly shown: boolean;
  readonly downloading: boolean;
  readonly onClick: () => void;
  readonly done: boolean;
};

export const PlanUpload = (fields: { enabled: boolean; emphasised: boolean; label: string; shown: boolean; downloading: boolean; onClick: () => void }): PlanUpload => ({ ...fields, done: fields.label === UPLOADED });

export const pauseRefusedText = (reason: string): string => `The aircraft did not pause, so the plan was not uploaded. ${reason}`;

export const sendPlan = (say: (message: string | null) => void, done: () => void, pauseFirst = false): void => {
  say(pauseFirst ? 'Pausing the aircraft' : 'Uploading to vehicle');
  void (async () => {
    const refused = pauseFirst ? await PlanBridge.pauseRefusal() : null;
    if (refused !== null) {
      say(pauseRefusedText(refused));
      await delay(FAILURE_MESSAGE_MS);
      done();
      return;
    }
    if (pauseFirst) say('Uploading to vehicle');
    const outcome = uploadOutcome(await PlanBridge.sendToVehicle());
    say(uploadMessage(outcome));
    if (outcome === UploadOutcome.NotStarted) await delay(FAILURE_MESSAGE_MS);
    done();
  })();
};

export const usePlanUpload = ({
  planStatus,
  aircraftAt,
  farAway,
  farText,
  feedback,
  onMoved,
}: {
  planStatus: Json | null;
  aircraftAt: TrackPoint | null;
  farAway: number | null;
  farText: string | null;
  feedback: PlanFeedback;
  onMoved: (at: TrackPoint) => void;
}): { readonly planUpload: PlanUpload; readonly dialogs: ReactNode } => {
  const { busy, edits, say, onBridge, onRefusal } = feedback;
  const uploadAsk = useRemembered<UploadGate | null>(null);
  const farAsk = useRemembered(false);
  const vehiclesJson = useMapPath(VEHICLES_VIEW);
  const planHasItems = optBoolean(planStatus, 'containsItems');
  const planOffline = optBoolean(planStatus, 'offline');
  const planDirty = optBoolean(planStatus, 'dirty');
  const planSyncing = optText(optJSONObject(planStatus, 'sync'), 'state') === 'busy';
  const uploadBlocked = syncRefusal(vehicleSyncState(planOffline, planSyncing), 'upload to') !== null;

  const sendUpload = (): void => {
    const refusal = syncRefusal(vehicleSyncState(planOffline, planSyncing), 'upload to');
    if (refusal !== null) {
      say(refusal);
      return;
    }
    void (async () => {
      const view = await freshPlanView();
      const step = uploadStep(uploadGate(view), notReadyToSend(view));
      switch (step.kind) {
        case 'Refuse':
          PlanFocus.notReady(view);
          say(step.reason);
          return;
        case 'Confirm':
          uploadAsk.value = step.gate;
          return;
        case 'Send':
          sendPlan(
            (message) => {
              busy.value = message;
            },
            () => {
              busy.value = null;
            },
          );
      }
    })();
  };

  const upload = (): void => {
    if (farAway !== null) farAsk.value = true;
    else sendUpload();
  };

  const downloading = planSyncing && optBoolean(optJSONObject(planStatus, 'sync'), 'downloading');
  const uploadEnabled = planHasItems && !planOffline && !planSyncing;
  const planUpload = PlanUpload({
    enabled: uploadEnabled,
    emphasised: uploadEnabled && !uploadBlocked,
    label: uploadLabel(planOffline, planSyncing, planDirty, planHasItems, downloading),
    shown: !planOffline,
    downloading,
    onClick: upload,
  });

  const dialogs = (
    <>
      {farAsk.value && aircraftAt !== null ? (
        <AlertDialog
          onDismissRequest={() => {
            farAsk.value = false;
          }}
          title={farText ?? ''}
          text={FAR_FROM_AIRCRAFT_TEXT}
          confirmButton={
            <Button
              onClick={() => {
                farAsk.value = false;
                onBridge(() => repositionMission(aircraftAt), {
                  label: 'Moving the plan to the aircraft',
                  done: 'Plan moved to the aircraft',
                  edited: true,
                  then: () => onMoved(aircraftAt),
                });
              }}
            >
              Move it to the aircraft
            </Button>
          }
          dismissButton={
            <TextButton
              onClick={() => {
                farAsk.value = false;
                sendUpload();
              }}
            >
              Upload anyway
            </TextButton>
          }
        />
      ) : null}

      {uploadAsk.value === null ? null : (
        <AlertDialog
          onDismissRequest={() => {
            uploadAsk.value = null;
          }}
          title={uploadHeading(uploadAsk.value, vehicleChoices(vehiclesJson))}
          text={uploadAsk.value.refusal}
          confirmButton={
            <div className="flex flex-wrap items-center justify-end gap-2">
              <TextButton
                onClick={() => {
                  const pauses = uploadAsk.value?.pausesFirst ?? false;
                  uploadAsk.value = null;
                  sendPlan(
                    (next) => {
                      busy.value = next;
                    },
                    () => {
                      busy.value = null;
                    },
                    pauses,
                  );
                }}
              >
                {uploadAsk.value.proceedTitle.trim() === '' ? 'Upload' : uploadAsk.value.proceedTitle}
              </TextButton>
              {uploadAsk.value.convertible ? (
                <Button
                  onClick={() => {
                    uploadAsk.value = null;
                    onRefusal(async () => {
                      const refusal = await PlanBridge.convertToVehicle();
                      if (refusal === null) edits.value += 1;
                      return refusal ?? CONVERTED;
                    }, 'Converting the plan');
                  }}
                >
                  Convert to this vehicle
                </Button>
              ) : null}
            </div>
          }
          dismissButton={
            <TextButton
              onClick={() => {
                uploadAsk.value = null;
              }}
            >
              Cancel
            </TextButton>
          }
        />
      )}
    </>
  );

  return { planUpload, dialogs };
};
