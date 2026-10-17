import { useState } from 'react';
import { type Json, optBoolean, optDouble, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Button, LinearProgressIndicator, Text, TextButton } from '@/m3';

export const AUTOTUNE_VIEW = 'view.autotune';
export const AUTOTUNE_REQUEST = 'vehicle.autotune.autotuneRequest';

export type AutotuneState = { canStart: boolean; status: string; progress: number; warning: string };

export const autotuneState = (view: Json | null): AutotuneState | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : { canStart: optBoolean(view, 'canStart'), status: optText(view, 'status'), progress: Math.fround(optDouble(view, 'progress', 0.0)), warning: optText(view, 'warning') };

export const AutotuneSection = () => {
  const view = useQgcPath(AUTOTUNE_VIEW);
  const [confirming, setConfirming] = useState(false);
  const state = autotuneState(view);
  if (state === null) return null;
  return (
    <>
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title="Start AutoTune"
          text={<span className="whitespace-pre-line">{state.warning}</span>}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                void Qgc.invoke(AUTOTUNE_REQUEST);
              }}
            >
              Ok
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
      <div className="flex flex-col gap-2">
        <Button onClick={() => setConfirming(true)} enabled={state.canStart} className="self-start">
          Start AutoTune
        </Button>
        <Text>{state.status}</Text>
        <LinearProgressIndicator progress={state.progress} className="w-full" label="AutoTune progress" />
      </div>
    </>
  );
};
