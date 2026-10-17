import { useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect, useProduceState } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Surface, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const APM_SUB_FRAME_SCREEN = 'apmSubFrame';
export const APM_SUB_FRAME_VIEW = 'view.apmSubFrame';
export const APM_SUB_FRAME_SET = 'apmSubFrame.set';
export const APM_SUB_FRAME_LOAD = 'apmSubFrame.loadDefaults';
const APM_SUB_FRAME_POLL_MS = 1000;

export type SubFrame = { name: string; value: number; hasDefaults: boolean };

export type SubFrames = { frames: readonly SubFrame[]; selected: number | null; confirmFirst: boolean; loading: boolean; loadError: string };

export const subFrames = (view: Json | null): SubFrames | null =>
  view === null || !optBoolean(view, 'available')
    ? null
    : {
        frames: jsonObjects(optJSONArray(view, 'frames')).map((f) => ({ name: optText(f, 'name'), value: optInt(f, 'value'), hasDefaults: optBoolean(f, 'hasDefaults') })),
        selected: isNull(view, 'selected') ? null : optInt(view, 'selected'),
        confirmFirst: optBoolean(view, 'confirmFirst'),
        loading: optBoolean(view, 'loadingDefaults'),
        loadError: optText(view, 'loadError'),
      };

export const frameImage = async (value: number): Promise<string | null> => {
  const png = optText(await Qgc.get(`view.apmSubFrameImage(${value})`), 'png');
  return png.trim() === '' ? null : `data:image/png;base64,${png}`;
};

export const ApmSubFrameScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<SubFrames | null>(null);
  const [picked, setPicked] = useState<SubFrame | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  useLaunchedEffect([revision], async (scope) => {
    const next = subFrames(await Qgc.get(APM_SUB_FRAME_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(APM_SUB_FRAME_POLL_MS);
    if (scope.isActive()) setRevision((it) => it + 1);
  });
  const act = (path: string, frame: SubFrame) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, frame.value));
    })();
  const state = read;
  if (state === null) {
    return <Text className={`block p-4 ${className ?? ''}`}>This page is for an ArduSub vehicle.</Text>;
  }
  const error = refusal ?? (state.loadError.trim() !== '' ? state.loadError : null);
  return (
    <>
      <div className={`flex w-full flex-col gap-2 p-3 ${className ?? ''}`}>
        {state.loading ? <Text typography="bodySmall">{"Loading the frame's default parameters…"}</Text> : null}
        {error !== null ? <Text color={md('error')}>{error}</Text> : null}
        <div className="grid min-h-0 flex-1 content-start gap-2 overflow-y-auto" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(160px, 1fr))' }}>
          {state.frames.map((frame) => (
            <FrameTile
              key={frame.value}
              frame={frame}
              chosen={frame.value === state.selected}
              onClick={() => {
                if (state.confirmFirst) setPicked(frame);
                else act(APM_SUB_FRAME_SET, frame);
              }}
            />
          ))}
        </div>
      </div>
      {picked !== null ? (
        <AlertDialog
          onDismissRequest={() => setPicked(null)}
          title="Frame selection"
          text={picked.hasDefaults ? 'Would you like to load the default parameters for the frame?' : 'Would you like to set the desired frame?'}
          confirmButton={
            <div className="flex flex-col items-end">
              {picked.hasDefaults ? (
                <TextButton
                  onClick={() => {
                    setPicked(null);
                    act(APM_SUB_FRAME_LOAD, picked);
                  }}
                >
                  {`Yes, Load default parameter set for ${picked.name}`}
                </TextButton>
              ) : null}
              <TextButton
                onClick={() => {
                  setPicked(null);
                  act(APM_SUB_FRAME_SET, picked);
                }}
              >
                {picked.hasDefaults ? 'No, set frame only' : `Confirm frame ${picked.name}`}
              </TextButton>
            </div>
          }
          dismissButton={<TextButton onClick={() => setPicked(null)}>Close</TextButton>}
        />
      ) : null}
    </>
  );
};

const FrameTile = ({ frame, chosen, onClick }: { frame: SubFrame; chosen: boolean; onClick: () => void }) => {
  const image = useProduceState<string | null>(null, [frame.value], async (set) => set(await frameImage(frame.value)));
  return (
    <Surface onClick={onClick} color={chosen ? md('primaryContainer') : md('surfaceVariant')} className="flex flex-col p-2">
      <Text typography="titleSmall">{frame.name}</Text>
      {image !== null ? <img src={image} alt={frame.name} className="aspect-[1.2] w-full object-contain" /> : null}
    </Surface>
  );
};
