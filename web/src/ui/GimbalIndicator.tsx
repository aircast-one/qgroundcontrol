import { type PointerEvent, useEffect, useMemo, useRef, useState } from 'react';
import { type Json, isNull, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc, makeFact, refusal } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { SetupCommands } from '@/bridge/BridgeCommands';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { mutableStateOf, useValue } from '@/kotlin/State';
import { coerceIn } from '@/kotlin/Std';
import { AlertDialog, Button, FilterChip, OutlinedButton, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { MaterialIcon } from '@/res/Icon';
import { Color, md, withAlpha } from '@/theme/Colors';
import { JOYSTICK_VIEW } from '@/ui/JoystickScreen';
import { APPROXIMATE_AIM, LENS_VIEW, lensReading } from '@/ui/Lens';
import { FactRow, type SettingsBlock, type SettingsSectionRows, sentenceCase, settingsPagePath, settingsSections } from '@/ui/SettingsScreen';

export const GIMBAL_INDICATOR_PATH = 'view.gimbalIndicator';
export const OTHERS_HAVE_CONTROL = 'othersHaveControl';

export const gimbalAsksForControl = mutableStateOf(false);

export const serialAsks = (seen: number | null, serial: number): boolean => seen !== null && serial > seen;

export const gimbalRefusal = (answer: Json | null): string | null => {
  if (optText(answer, 'refusal') === OTHERS_HAVE_CONTROL) {
    gimbalAsksForControl.value = true;
    return null;
  }
  return refusal(answer);
};

export type GimbalChoice = { name: string; managerCompid: number; deviceId: number; active: boolean };

export type GimbalIndicatorState = {
  statusText: string;
  pitchText: string;
  yawText: string;
  yawLockLabel: string;
  yawLocked: boolean;
  yawLockOffered: boolean;
  retractOffered: boolean;
  controlOffered: boolean;
  controlLabel: string;
  haveControl: boolean;
  gimbals: readonly GimbalChoice[];
  pitchDegrees: number | null;
  yawDegrees: number | null;
};

const finiteOrNull = (value: number): number | null => (Number.isFinite(value) ? value : null);

export const gimbalIndicator = (view: Json | null): GimbalIndicatorState | null =>
  view !== null && optBoolean(view, 'shown')
    ? {
        statusText: optText(view, 'statusText'),
        pitchText: optText(view, 'pitchText'),
        yawText: optText(view, 'yawText'),
        yawLockLabel: optText(view, 'yawLockLabel'),
        yawLocked: optBoolean(view, 'yawLocked'),
        yawLockOffered: optBoolean(view, 'yawLockOffered'),
        retractOffered: optBoolean(view, 'retractOffered'),
        controlOffered: optBoolean(view, 'controlOffered'),
        controlLabel: optText(view, 'controlLabel'),
        haveControl: optBoolean(view, 'haveControl'),
        pitchDegrees: finiteOrNull(optDouble(view, 'pitchDegrees', NaN)),
        yawDegrees: finiteOrNull(optDouble(view, 'bodyYawDegrees', NaN)),
        gimbals: jsonObjects(optJSONArray(view, 'gimbals')).map((g) => ({
          name: optText(g, 'name'),
          managerCompid: optInt(g, 'managerCompid'),
          deviceId: optInt(g, 'deviceId'),
          active: optBoolean(g, 'active'),
        })),
      }
    : null;

export const gimbalCellText = (state: GimbalIndicatorState): string =>
  [state.gimbals.length > 1 ? (state.gimbals.find((gimbal) => gimbal.active)?.name ?? '') : '', state.statusText, state.pitchText, state.yawText]
    .filter((part) => part.trim() !== '')
    .join(' · ');

export const GimbalTakeControlDialog = () => {
  const asking = useValue(gimbalAsksForControl);
  const view = useQgcPath(GIMBAL_INDICATOR_PATH);
  const serial = view === null ? -1 : optInt(view, 'askSerial', -1);
  const seen = useRef<number | null>(null);
  useEffect(() => {
    if (serialAsks(seen.current, serial)) gimbalAsksForControl.value = true;
    if (serial >= 0) seen.current = serial;
  }, [serial]);
  return asking ? (
    <AlertDialog
      onDismissRequest={() => (gimbalAsksForControl.value = false)}
      title="Request Gimbal Control?"
      text="Command not sent. Another user has control of the gimbal."
      confirmButton={
        <TextButton
          onClick={() => {
            gimbalAsksForControl.value = false;
            void SetupCommands.takeGimbalControlRefusal();
          }}
        >
          Yes
        </TextButton>
      }
      dismissButton={<TextButton onClick={() => (gimbalAsksForControl.value = false)}>No</TextButton>}
    />
  ) : null;
};

export const GimbalIndicatorCell = () => {
  const view = useQgcPath(GIMBAL_INDICATOR_PATH);
  const state = useMemo(() => gimbalIndicator(view), [view]);
  const [open, setOpen] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const [settingsOpen, setSettingsOpen] = useState(false);
  if (state === null) return null;

  const act = (path: string, ...args: unknown[]) =>
    void (async () => {
      const reason = gimbalRefusal(await Qgc.call(path, ...args));
      setRefused(reason);
      if (reason === null) setOpen(false);
    })();

  return (
    <>
      <button type="button" className="text-left" onClick={() => setOpen(true)}>
        <Text typography="labelMedium" color={md('onSurfaceVariant')} maxLines={1}>
          {gimbalCellText(state)}
        </Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-2 px-sheet pb-6">
            <Text typography="titleMedium">Gimbal</Text>
            {state.gimbals.length > 1 ? (
              <div className="flex gap-1">
                {state.gimbals.map((gimbal) => (
                  <FilterChip key={`${gimbal.managerCompid}-${gimbal.deviceId}`} selected={gimbal.active} onClick={() => act('gimbal.select', gimbal.managerCompid, gimbal.deviceId)} label={gimbal.name} />
                ))}
              </div>
            ) : null}
            <Text typography="bodyMedium">{gimbalCellText(state)}</Text>
            {state.yawLockOffered ? (
              <OutlinedButton className="w-full" onClick={() => act('gimbal.yawLock', !state.yawLocked)}>
                {state.yawLockLabel}
              </OutlinedButton>
            ) : null}
            <OutlinedButton className="w-full" onClick={() => act('gimbal.center')}>
              Center
            </OutlinedButton>
            <OutlinedButton className="w-full" onClick={() => act('gimbal.tilt90')}>
              Tilt 90
            </OutlinedButton>
            <OutlinedButton className="w-full" onClick={() => act('gimbal.pointHome')}>
              Point home
            </OutlinedButton>
            {state.retractOffered ? (
              <OutlinedButton className="w-full" onClick={() => act('gimbal.retract')}>
                Retract
              </OutlinedButton>
            ) : null}
            {state.controlOffered ? (
              <Button className="w-full" onClick={() => act('gimbal.control', !state.haveControl)}>
                {state.controlLabel}
              </Button>
            ) : null}
            {refused !== null ? <Text color={md('error')}>{refused}</Text> : null}
            <div>
              <TextButton
                onClick={() => {
                  setOpen(false);
                  setSettingsOpen(true);
                }}
              >
                Gimbal settings
              </TextButton>
            </div>
          </div>
        </AircastSheet>
      ) : null}

      {settingsOpen ? (
        <AircastSheet onDismissRequest={() => setSettingsOpen(false)}>
          <GimbalSettings />
        </AircastSheet>
      ) : null}
    </>
  );
};

const GIMBAL_CONTROLLER_SETTINGS = 'gimbalControllerSettings';
const GIMBAL_SETTINGS_PAGE = 'Gimbal Controller';
const JOYSTICK_BUTTONS_SPEED = 'joystickButtonsSpeed';

export const joystickButtonsAvailable = (view: Json | null): boolean => view !== null && !isNull(view, 'active') && optBoolean(view, 'vehicle') && optBoolean(view, 'enabled');

export const gimbalSettingsBlocks = (sections: readonly SettingsSectionRows[], joystickButtons: boolean): readonly SettingsBlock[] =>
  sections
    .filter((section) => section.group === GIMBAL_CONTROLLER_SETTINGS)
    .flatMap((section) => section.blocks)
    .map((block) => ({
      ...block,
      facts: block.facts.map((fact) => (fact.name !== JOYSTICK_BUTTONS_SPEED || joystickButtons ? fact : makeFact({ ...fact, enabled: false, disabledReason: 'No joystick is enabled for this vehicle.' }))),
    }));

export const GimbalSettings = () => {
  const joystick = useQgcPath(JOYSTICK_VIEW);
  const [reloads, setReloads] = useState(0);
  const [sections, setSections] = useState<readonly SettingsSectionRows[]>([]);
  useLaunchedEffect([reloads], async (scope) => {
    const loaded = settingsSections(await Qgc.get(settingsPagePath(GIMBAL_SETTINGS_PAGE)));
    if (scope.isActive()) setSections(loaded);
  });
  return (
    <div className="flex w-full flex-col pb-6">
      {gimbalSettingsBlocks(sections, joystickButtonsAvailable(joystick)).map((block, index) => (
        <div key={`${block.title}-${index}`} className="flex flex-col">
          {block.title.trim() !== '' ? (
            <Text typography="titleMedium" className="px-sheet py-2">
              {sentenceCase(block.title)}
            </Text>
          ) : null}
          {block.facts.map((fact) => (
            <FactRow key={fact.path} fact={fact} onWrite={() => setReloads((count) => count + 1)} />
          ))}
        </div>
      ))}
    </div>
  );
};

export const GIMBAL_DRAG_REPEAT_MS = 100;

export const screenFraction = (x: number, y: number, width: number, height: number): readonly [number, number] => [(x / width) * 2 - 1, -((y / height) * 2 - 1)];

export type OnScreenGimbal = { enabled: boolean; clickAndDrag: boolean };

export const onScreenGimbal = (view: Json | null): OnScreenGimbal | null => {
  const onScreen = view !== null && optBoolean(view, 'shown') ? optJSONObject(view, 'onScreen') : null;
  return onScreen === null ? null : { enabled: optBoolean(onScreen, 'enabled'), clickAndDrag: optBoolean(onScreen, 'clickAndDrag') };
};

export type VideoGimbalMode = 'drag' | 'point' | 'hold';

const ON_SCREEN_SETTING = 'settings.gimbalControllerSettings.enableOnScreenControl';
const CLICK_AND_DRAG_SETTING = 'settings.gimbalControllerSettings.clickAndDrag';

type VideoGimbalModeChoice = { mode: VideoGimbalMode; label: string; icon: string; hint: string; onScreen: boolean; clickAndDrag: boolean };

export const VIDEO_GIMBAL_MODES: readonly VideoGimbalModeChoice[] = [
  { mode: 'drag', label: 'Drag to move', icon: 'pan_tool', hint: 'The camera follows your drag on the video.', onScreen: false, clickAndDrag: false },
  { mode: 'point', label: 'Tap to point', icon: 'ads_click', hint: 'The camera turns to where you tap on the video.', onScreen: true, clickAndDrag: false },
  { mode: 'hold', label: 'Hold and drag', icon: 'joystick', hint: 'Press, then drag away from that spot; the further you drag, the faster the camera turns.', onScreen: true, clickAndDrag: true },
];

export const videoGimbalMode = (control: OnScreenGimbal): VideoGimbalMode => (!control.enabled ? 'drag' : control.clickAndDrag ? 'hold' : 'point');

const modeChoice = (mode: VideoGimbalMode): VideoGimbalModeChoice => VIDEO_GIMBAL_MODES.find((it) => it.mode === mode)!;

export const nextVideoGimbalMode = (mode: VideoGimbalMode): VideoGimbalMode => VIDEO_GIMBAL_MODES[(VIDEO_GIMBAL_MODES.indexOf(modeChoice(mode)) + 1) % VIDEO_GIMBAL_MODES.length]!.mode;

const chooseVideoGimbalMode = (mode: VideoGimbalMode) =>
  void (async () => {
    const choice = modeChoice(mode);
    await Qgc.set(ON_SCREEN_SETTING, choice.onScreen);
    await Qgc.set(CLICK_AND_DRAG_SETTING, choice.clickAndDrag);
  })();

export const GimbalVideoMode = () => {
  const view = useQgcPath(GIMBAL_INDICATOR_PATH);
  const control = useMemo(() => onScreenGimbal(view), [view]);
  if (control === null) return null;
  const current = videoGimbalMode(control);
  return (
    <div className="flex flex-col gap-1">
      <Text typography="titleSmall">On the video</Text>
      <div className="flex flex-wrap gap-2">
        {VIDEO_GIMBAL_MODES.map((mode) => (
          <FilterChip key={mode.mode} selected={current === mode.mode} onClick={() => chooseVideoGimbalMode(mode.mode)} label={mode.label} />
        ))}
      </div>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {modeChoice(current).hint}
      </Text>
    </div>
  );
};

export const GimbalModeSwitch = ({ className }: { className?: string }) => {
  const view = useQgcPath(GIMBAL_INDICATOR_PATH);
  const control = useMemo(() => onScreenGimbal(view), [view]);
  if (control === null) return null;
  const current = modeChoice(videoGimbalMode(control));
  const next = modeChoice(nextVideoGimbalMode(current.mode));
  return (
    <button
      type="button"
      aria-label={`Video gimbal mode, ${current.label}`}
      aria-description={`Switch to ${next.label}`}
      title={current.hint}
      onPointerDown={(event) => event.stopPropagation()}
      onClick={(event) => {
        event.stopPropagation();
        chooseVideoGimbalMode(next.mode);
      }}
      className={`flex items-center gap-1.5 rounded-full py-1.5 ps-2 pe-3 ${className ?? ''}`}
      style={{ background: withAlpha(Color.Black, 0.6), color: Color.White }}
    >
      <MaterialIcon name={current.icon} filled size={18} />
      <Text typography="labelMedium" color={Color.White}>
        {current.label}
      </Text>
    </button>
  );
};

export const aimFraction = (delta: number, width: number): number => delta / (Math.max(width, 1) / 2);

type Point = { x: number; y: number };

const pointIn = (event: PointerEvent<HTMLDivElement>): Point & { width: number; height: number } => {
  const box = event.currentTarget.getBoundingClientRect();
  return { x: event.clientX - box.left, y: event.clientY - box.top, width: box.width, height: box.height };
};

export const GimbalScreenControl = ({ className }: { className?: string }) => {
  const view = useQgcPath(GIMBAL_INDICATOR_PATH);
  const control = useMemo(() => onScreenGimbal(view), [view]);
  const lensJson = useQgcPath(LENS_VIEW);
  const approximate = useMemo(() => lensReading(lensJson)?.approximate === true, [lensJson]);
  const drag = useRef<Point | null>(null);
  const dragBegun = useRef(false);
  const tapping = useRef(false);
  const press = useRef<{ latest: Point; width: number; height: number; repeating: ReturnType<typeof setInterval> } | null>(null);
  useEffect(() => () => {
    if (press.current !== null) clearInterval(press.current.repeating);
  }, []);
  if (control === null) return null;

  const send = (pan: number, tilt: number, point: boolean) => void (async () => gimbalRefusal(await Qgc.call('gimbal.onScreen', pan, tilt, point)))();
  const sendDrag = (pan: number, tilt: number, begin: boolean) => void (async () => gimbalRefusal(await Qgc.call('gimbal.drag', pan, tilt, begin)))();

  const release = () => {
    if (press.current !== null) clearInterval(press.current.repeating);
    press.current = null;
    drag.current = null;
  };

  if (!control.enabled) {
    return (
      <div
        className={`touch-none ${className ?? ''}`}
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId);
          drag.current = { x: event.clientX, y: event.clientY };
          dragBegun.current = false;
        }}
        onPointerMove={(event) => {
          const from = drag.current;
          if (from === null) return;
          const box = event.currentTarget.getBoundingClientRect();
          const begin = !dragBegun.current;
          dragBegun.current = true;
          sendDrag(aimFraction(event.clientX - from.x, box.width), -aimFraction(event.clientY - from.y, box.height), begin);
        }}
        onPointerUp={release}
        onPointerCancel={release}
      />
    );
  }

  if (!control.clickAndDrag) {
    return (
      <div
        className={className}
        onPointerDown={() => {
          tapping.current = true;
        }}
        onPointerLeave={() => {
          tapping.current = false;
        }}
        onPointerCancel={() => {
          tapping.current = false;
        }}
        onPointerUp={(event) => {
          if (!tapping.current) return;
          tapping.current = false;
          const at = pointIn(event);
          const [pan, tilt] = screenFraction(at.x, at.y, at.width, at.height);
          send(pan, tilt, true);
        }}
      >
        {approximate ? (
          <Text typography="labelMedium" color="#ffffff" className="pointer-events-none absolute start-1/2 top-[72px] -translate-x-1/2 rounded-lg bg-black/55 px-2.5 py-1.5 whitespace-nowrap">
            {APPROXIMATE_AIM}
          </Text>
        ) : null}
      </div>
    );
  }

  return (
    <div
      className={`touch-none ${className ?? ''}`}
      onPointerDown={(event) => {
        event.currentTarget.setPointerCapture(event.pointerId);
        release();
        const at = pointIn(event);
        const start = screenFraction(at.x, at.y, at.width, at.height);
        const repeating = setInterval(() => {
          const held = press.current;
          if (held === null) return;
          const now = screenFraction(held.latest.x, held.latest.y, held.width, held.height);
          send(coerceIn(now[0] - start[0], -1, 1), coerceIn(now[1] - start[1], -1, 1), false);
        }, GIMBAL_DRAG_REPEAT_MS);
        press.current = { latest: { x: at.x, y: at.y }, width: at.width, height: at.height, repeating };
      }}
      onPointerMove={(event) => {
        const held = press.current;
        if (held === null) return;
        const at = pointIn(event);
        press.current = { ...held, latest: { x: at.x, y: at.y } };
      }}
      onPointerUp={release}
      onPointerCancel={release}
    />
  );
};
