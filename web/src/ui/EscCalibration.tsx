import { useState } from 'react';
import { type Json, optBoolean, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { SetupCommands } from '@/bridge/BridgeCommands';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { AlertDialog, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const DIALOG_CONTROL = 'dialog';
const ESC_CAL_VIEW = 'view.escCalibration';
const ESC_CAL_POLL_MS = 500;

export type EscCalibrationState = { highlight: string; text: string; running: boolean };

export const EscCalibrationState = (fields: { highlight: string; text: string; running?: boolean }): EscCalibrationState => ({ running: false, ...fields });

export const escCalibration = (view: Json | null): EscCalibrationState | null =>
  view !== null && optBoolean(view, 'open') ? EscCalibrationState({ highlight: optText(view, 'highlight'), text: optText(view, 'text'), running: optBoolean(view, 'running') }) : null;

export const EscCalibrationDialog = ({ onClose }: { onClose: () => void }) => {
  const [state, setState] = useState<EscCalibrationState | null>(null);
  useLaunchedEffect([], async (scope) => {
    await SetupCommands.startEscCalibration();
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const read = escCalibration(await Qgc.get(ESC_CAL_VIEW));
      if (scope.isActive()) setState(read);
      await delay(ESC_CAL_POLL_MS);
      return poll();
    };
    await poll();
  });
  return (
    <AlertDialog
      onDismissRequest={() => undefined}
      title="ESC Calibration"
      text={
        <div className="flex flex-col gap-2">
          <span className="whitespace-pre-line">
            <span className="font-bold" style={{ color: md('error') }}>
              {state?.highlight ?? ''}
            </span>
            {state?.text ?? 'Starting ESC calibration...'}
          </span>
        </div>
      }
      confirmButton={
        <TextButton
          enabled={state?.running === false}
          onClick={() => {
            void SetupCommands.closeEscCalibration();
            onClose();
          }}
        >
          OK
        </TextButton>
      }
    />
  );
};
