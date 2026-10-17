import { useRef, useState } from 'react';
import { type Json, jsonStrings, optBoolean, optDouble, optInt, optJSONArray, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { stageFile, unstageAll } from '@/core/files';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { openDocument } from '@/kotlin/Platform';
import { delay } from '@/kotlin/Std';
import { AlertDialog, Button, FilledTonalButton, FilterChip, Slider, Surface, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';

export const LOG_REPLAY_VIEW = 'view.logReplay';
export const LOG_REPLAY_START = 'logReplay.start';
export const LOG_REPLAY_TOGGLE = 'logReplay.togglePlay';
export const LOG_REPLAY_SPEED = 'logReplay.speed';
export const LOG_REPLAY_SEEK = 'logReplay.seek';
export const LOG_REPLAY_CLOSE = 'logReplay.close';
const REPLAY_FOLDER = 'log-replay';
const REPLAY_FALLBACK = 'log-replay.tlog';

export const replayFileName = (shown: string | null): string => {
  const name = shown === null ? null : shown.slice(shown.lastIndexOf('/') + 1);
  return name !== null && name.trim() !== '' && name !== '.' && name !== '..' ? name : REPLAY_FALLBACK;
};

const PLAYING_POLL_MS = 250;
const IDLE_POLL_MS = 1000;

export type LogReplay = {
  readonly shown: boolean;
  readonly loaded: boolean;
  readonly playing: boolean;
  readonly percent: number;
  readonly playheadTime: string;
  readonly totalTime: string;
  readonly speedIndex: number;
  readonly speeds: readonly string[];
  readonly canLoad: boolean;
  readonly loadRefusal: string;
  readonly error: string;
};

export const replayProgress = (replay: LogReplay): string => [replay.playheadTime, replay.totalTime].filter((it) => it.trim() !== '').join(' of ');

export const speedLabel = (label: string): string => `${label.endsWith('x') ? label.slice(0, -1) : label}×`;

export const logReplay = (view: Json | null): LogReplay | null =>
  view !== null && optBoolean(view, 'available')
    ? {
        shown: optBoolean(view, 'shown'),
        loaded: optBoolean(view, 'loaded'),
        playing: optBoolean(view, 'playing'),
        percent: optDouble(view, 'percent', 0.0),
        playheadTime: optText(view, 'playheadTime'),
        totalTime: optText(view, 'totalTime'),
        speedIndex: optInt(view, 'speedIndex', 3),
        speeds: jsonStrings(optJSONArray(view, 'speeds')),
        canLoad: optBoolean(view, 'canLoad'),
        loadRefusal: optText(view, 'loadRefusal'),
        error: optText(view, 'error'),
      }
    : null;

const stagedRefusal = async (chosen: File): Promise<string | null> => {
  const staged = `/${REPLAY_FOLDER}/${replayFileName(chosen.name)}`;
  await unstageAll(`/${REPLAY_FOLDER}/`);
  const copied = await stageFile(staged, chosen).catch(() => false);
  return copied ? Qgc.refusalOf(LOG_REPLAY_START, staged) : 'That file could not be read.';
};

export const LogReplayBar = () => {
  const [read, setRead] = useState<LogReplay | null>(null);
  const [refresh, setRefresh] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [dragged, setDragged] = useState(0);
  const draggedNow = useRef(0);
  const [message, setMessage] = useState<string | null>(null);

  useLaunchedEffect([refresh], async (scope) => {
    const next = logReplay(await Qgc.get(LOG_REPLAY_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(next?.playing === true ? PLAYING_POLL_MS : IDLE_POLL_MS);
    if (scope.isActive()) setRefresh((it) => it + 1);
  });

  const act = (path: string, ...args: unknown[]) => {
    void Qgc.refusalOf(path, ...args).then((refused) => {
      setMessage(refused);
      setRefresh((it) => it + 1);
    });
  };

  const pick = async () => {
    const chosen = await openDocument(['*/*']);
    if (chosen === null) return;
    setMessage(await stagedRefusal(chosen));
    setRefresh((it) => it + 1);
  };

  const replay = read;
  return (
    <>
      {replay !== null && replay.shown ? (
        <Surface color={md('surfaceContainer')} contentColor={md('onSurface')}>
          <div className="flex w-full flex-col gap-2 px-4 py-2">
            <div className="flex items-center">
              <Text typography="bodyMedium" className="min-w-0 flex-1">
                {replayProgress(replay)}
              </Text>
              <Text typography="labelLarge" color={md('primary')}>{`${Math.trunc(replay.percent)}%`}</Text>
            </div>
            <Slider
              value={dragging ? dragged : replay.percent}
              onValueChange={(it) => {
                setDragging(true);
                draggedNow.current = it;
                setDragged(it);
              }}
              onValueChangeFinished={() => {
                setDragging(false);
                act(LOG_REPLAY_SEEK, draggedNow.current);
              }}
              valueRange={[0, 100]}
              enabled={replay.loaded}
              label="Log replay"
            />
            <div className="flex gap-2 overflow-x-auto">
              {replay.speeds.map((label, index) => (
                <FilterChip key={index} selected={index === replay.speedIndex} onClick={() => act(LOG_REPLAY_SPEED, index)} label={speedLabel(label)} className="shrink-0" />
              ))}
            </div>
            <div className="flex w-full justify-end gap-2">
              <TextButton onClick={() => act(LOG_REPLAY_CLOSE)}>Close</TextButton>
              {!replay.loaded ? <FilledTonalButton onClick={() => (replay.canLoad ? void pick() : setMessage(replay.loadRefusal))}>Load telemetry log</FilledTonalButton> : null}
              <Button enabled={replay.loaded} onClick={() => act(LOG_REPLAY_TOGGLE)}>
                {replay.playing ? 'Pause' : 'Play'}
              </Button>
            </div>
          </div>
        </Surface>
      ) : null}
      {replay !== null && replay.shown && message !== null ? (
        <AlertDialog
          onDismissRequest={() => setMessage(null)}
          title="Log replay"
          text={message}
          confirmButton={<TextButton onClick={() => setMessage(null)}>OK</TextButton>}
        />
      ) : null}
    </>
  );
};
