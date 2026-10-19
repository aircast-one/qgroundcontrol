import { useState } from 'react';
import { type Json, type JsonArray, isNull, jsonObjects, jsonStrings, opt, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type FactSlider, Qgc } from '@/bridge/Qgc';
import { useQgcDerived } from '@/bridge/QgcState';
import { useKeyedState } from '@/kotlin/Compose';
import { AlertDialog, Button, Checkbox, DropdownMenu, DropdownMenuItem, LinearProgressIndicator, OutlinedButton, OutlinedTextField, RadioButton, Switch, Text, TextButton } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { md } from '@/theme/Colors';
import { factSlider } from '@/ui/ParameterForm';
import { FieldSlider } from '@/ui/SettingsScreen';
import { EmptyState, SectionHeader } from '@/ui/SetupRow';

export const JOYSTICK_VIEW = 'view.joystick';
export const JOYSTICK_SETUP_VIEW = 'view.joystickSetup';
export const JOYSTICK_SCREEN = 'joystick';
export const JOYSTICK_SELECT = 'joystick.select';
export const JOYSTICK_ENABLE = 'joystick.enable';
export const JOYSTICK_SETTING = 'joystick.setting';
export const JOYSTICK_CALIBRATION = 'joystick.calibration';
export const JOYSTICK_BUTTON_ACTION = 'joystick.buttonAction';
export const JOYSTICK_BUTTON_REPEAT = 'joystick.buttonRepeat';
export const NO_ACTION = 'No Action';
const AXIS_RANGE = 32767;

export type JoystickSetting = { name: string; type: string; label: string; units: string; value: unknown; slider: FactSlider | null };

export type JoystickAxis = { index: number; raw: number | null; label: string };

export type JoystickButton = { index: number; action: string; repeat: boolean; pressed: boolean };

export type AssignableAction = { action: string; canRepeat: boolean };

export type StickValues = { roll: number; pitch: number; yaw: number; throttle: number };

export type JoystickCalibration = {
  calibrating: boolean;
  status: StatusLines;
  nextText: string;
  nextEnabled: boolean;
  cancelEnabled: boolean;
  oneSidedVisible: boolean;
  stickPositions: readonly number[];
  singleStick: boolean;
  step: number | null;
  steps: number | null;
  holding: number | null;
  checking: boolean;
  warning: string | null;
  live: StickValues;
  livePositions: readonly number[];
};

type CalibrationExtras = 'stickPositions' | 'singleStick' | 'step' | 'steps' | 'holding' | 'checking' | 'warning' | 'live' | 'livePositions';

export const JoystickCalibration = (fields: Omit<JoystickCalibration, CalibrationExtras> & Partial<Pick<JoystickCalibration, CalibrationExtras>>): JoystickCalibration => ({
  stickPositions: [0, 0, 0, 0],
  singleStick: false,
  step: null,
  steps: null,
  holding: null,
  checking: false,
  warning: null,
  live: { roll: 0, pitch: 0, yaw: 0, throttle: 0 },
  livePositions: [0, 0, 0, 0],
  ...fields,
});

export type StatusLines = { bullets: boolean; lines: readonly string[] };

export const statusLines = (json: Json | null): StatusLines => ({ bullets: optBoolean(json, 'bullets'), lines: jsonStrings(optJSONArray(json, 'lines')) });

const intAt = (array: JsonArray, index: number): number => optInt({ value: array[index] }, 'value');

const numberAt = (array: JsonArray, index: number): number => optDouble({ value: array[index] }, 'value', 0);

const orNull = (json: Json | null, key: string, read: (json: Json | null, key: string) => number): number | null => (isNull(json, key) ? null : read(json, key));

const blankToNull = (text: string): string | null => (text.trim() === '' ? null : text);

export const joystickCalibration = (json: Json | null): JoystickCalibration => {
  const positions = optJSONArray(json, 'stickPositions');
  const live = optJSONObject(json, 'live');
  const livePositions = optJSONArray(json, 'livePositions');
  return JoystickCalibration({
    calibrating: optBoolean(json, 'calibrating'),
    status: statusLines(optJSONObject(json, 'status')),
    nextText: blankToNull(optText(json, 'nextText')) ?? 'Calibrate',
    nextEnabled: optBoolean(json, 'nextEnabled', true),
    cancelEnabled: optBoolean(json, 'cancelEnabled'),
    oneSidedVisible: optBoolean(json, 'oneSidedVisible'),
    stickPositions: positions === null ? [0, 0, 0, 0] : [0, 1, 2, 3].map((at) => intAt(positions, at)),
    singleStick: optBoolean(json, 'singleStickDisplay'),
    step: orNull(json, 'step', optInt),
    steps: orNull(json, 'steps', optInt),
    holding: orNull(json, 'holding', (j, k) => optDouble(j, k, 0)),
    checking: optBoolean(json, 'checking'),
    warning: blankToNull(optText(json, 'warning')),
    live: { roll: optDouble(live, 'roll', 0), pitch: optDouble(live, 'pitch', 0), yaw: optDouble(live, 'yaw', 0), throttle: optDouble(live, 'throttle', 0) },
    livePositions: livePositions === null ? [0, 0, 0, 0] : [0, 1, 2, 3].map((at) => numberAt(livePositions, at)),
  });
};

const FUNCTION_LABELS: Record<string, string> = { roll: 'Roll', pitch: 'Pitch', yaw: 'Yaw', throttle: 'Throttle' };

export const functionLabel = (id: string): string => FUNCTION_LABELS[id] ?? id;

const thisSite = (): string => (typeof location === 'undefined' ? 'this site' : location.host);

export type JoystickPage = {
  names: readonly string[];
  active: string | null;
  vehicle: boolean;
  enabled: boolean;
  calibrated: boolean;
  settings: readonly JoystickSetting[];
  axes: readonly JoystickAxis[];
  armed: boolean;
  calibration: JoystickCalibration;
  commanding: boolean;
  sendingText: string | null;
  transmitterMode: number;
  transmitterModeText: string | null;
  buttons: readonly JoystickButton[];
  actions: readonly AssignableAction[];
  standbyText: string | null;
};

export const joystickPage = (view: Json | null): JoystickPage | null => {
  if (view === null || !optBoolean(view, 'available')) return null;
  const state = optJSONObject(view, 'state');
  return {
    names: jsonStrings(optJSONArray(view, 'names')),
    active: isNull(view, 'active') ? null : optText(view, 'active'),
    vehicle: optBoolean(view, 'vehicle'),
    enabled: optBoolean(view, 'enabled'),
    calibrated: optBoolean(view, 'calibrated'),
    settings: jsonObjects(optJSONArray(view, 'settings'))
      .filter((s) => optBoolean(s, 'visible', true))
      .map((s) => {
        const range = optJSONObject(s, 'slider');
        return { name: optText(s, 'name'), type: optText(s, 'type'), label: optText(s, 'label'), units: optText(s, 'units'), value: opt(s, 'value'), slider: range === null ? null : factSlider(range, '') };
      }),
    axes: jsonObjects(optJSONArray(state, 'axes')).map((a) => ({ index: optInt(a, 'index'), raw: isNull(a, 'raw') ? null : optInt(a, 'raw'), label: optText(a, 'label') })),
    armed: optBoolean(view, 'armed'),
    calibration: joystickCalibration(optJSONObject(view, 'calibration')),
    commanding: optBoolean(state, 'commanding'),
    sendingText: blankToNull(optText(view, 'sendingText')),
    transmitterMode: optInt(view, 'transmitterMode', 2),
    transmitterModeText: blankToNull(optText(view, 'transmitterModeText')),
    buttons: jsonObjects(optJSONArray(state, 'buttons')).map((b) => {
      const event = optText(b, 'event');
      return { index: optInt(b, 'index'), action: isNull(b, 'action') ? NO_ACTION : optText(b, 'action'), repeat: optBoolean(b, 'repeat'), pressed: event === 'down' || event === 'repeat' };
    }),
    actions: jsonObjects(optJSONArray(view, 'assignableActions')).map((a) => ({ action: optText(a, 'action'), canRepeat: optBoolean(a, 'canRepeat') })),
    standbyText: blankToNull(optText(optJSONObject(view, 'standby'), 'text')),
  };
};

const BASIC_SETTINGS = ['standby', 'throttleModeCenterZero', 'throttleSmoothing', 'exponentialPct', 'negativeThrust'];
const ADVANCED_SETTINGS = ['circleCorrection', 'axisFrequencyHz', 'buttonFrequencyHz', 'useDeadband'];
const EXTENSION_SETTINGS = ['enableManualControlPitchExtension', 'enableManualControlRollExtension'];
const EXTENSION_LABELS = ['Pitch', 'Roll'];
const ADDITIONAL_SETTINGS = [1, 2, 3, 4, 5, 6].map((it) => `enableAdditionalAxis${it}`);

export const JoystickScreen = ({ className }: { className?: string }) => {
  const page = useQgcDerived(JOYSTICK_SETUP_VIEW, joystickPage);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [advanced, setAdvanced] = useState(false);
  const [offerEnable, setOfferEnable] = useState<string | null>(null);

  const act = (path: string, ...args: unknown[]) =>
    void (async () => {
      setRefusal(await Qgc.refusalOf(path, ...args));
    })();

  if (page === null) return null;
  const read = page;
  if (read.names.length === 0 || read.active === null) {
    return <EmptyState icon="ic_gamepad" title="No joystick" text="No joysticks or gamepads detected. Pair one over Bluetooth or plug it in over USB." className={className} />;
  }
  const setting = (name: string): JoystickSetting | null => read.settings.find((it) => it.name === name) ?? null;
  const present = (names: readonly string[]): JoystickSetting[] => names.map(setting).filter((it): it is JoystickSetting => it !== null);
  const additionalAxes = setting('additionalAxesFunction');
  const viaRc = additionalAxes === null || (typeof additionalAxes.value === 'number' && Math.trunc(additionalAxes.value) === 1);
  const calibrating = read.calibration.calibrating;
  const step = (op: string) =>
    void (async () => {
      const answer = await Qgc.call(JOYSTICK_CALIBRATION, op);
      setRefusal(answer !== null && !optBoolean(answer, 'ok') ? optText(answer, 'reason') : null);
      if (optBoolean(answer, 'completed') && !read.enabled) setOfferEnable(optText(answer, 'name'));
    })();

  return (
    <>
      <div className={`flex w-full flex-col gap-2 overflow-auto px-5 py-3 ${className ?? ''}`}>
        {read.names.length > 1 ? <JoystickPicker page={read} onPick={(it) => act(JOYSTICK_SELECT, it)} /> : null}
        <div className="flex items-center gap-3">
          <Checkbox checked={read.enabled} enabled={read.vehicle && (read.calibrated || read.enabled)} onCheckedChange={(it) => act(JOYSTICK_ENABLE, it)} label="Enable" />
          <Text>Enable</Text>
          {!read.vehicle ? <Text color={md('onSurfaceVariant')}>Not currently available</Text> : null}
        </div>
        <Text color={read.calibrated ? md('onSurface') : md('error')}>{read.calibrated ? `Calibrated · saved in this browser for ${thisSite()}` : 'Calibrate before enabling'}</Text>
        {read.sendingText !== null ? <Text color={read.commanding ? md('primary') : md('error')}>{read.sendingText}</Text> : null}
        {refusal !== null ? <Text color={md('error')}>{refusal}</Text> : null}

        {!read.armed ? (
          <>
            <SectionHeader text="Calibration" />
            {!calibrating ? <TransmitterModeRow mode={read.transmitterMode} text={read.transmitterModeText} onPick={(it) => act(JOYSTICK_SETTING, 'transmitterMode', it)} /> : null}
            <CalibrationPanel page={read} onStep={step} />
          </>
        ) : null}

        {calibrating ? null : (
          <>
            <SectionHeader text="Axis monitor" />
            {read.axes.map((axis) => (
              <div key={axis.index} className="flex items-center gap-2">
                <Text className="w-24 shrink-0">{axis.label}</Text>
                <LinearProgressIndicator progress={((axis.raw ?? 0) / AXIS_RANGE + 1) / 2} className="flex-1" />
              </div>
            ))}

            <SectionHeader text="Buttons" />
            <Text typography="bodySmall">Multiple buttons that have the same action must be pressed simultaneously to invoke the action.</Text>
            {read.buttons.map((button) => (
              <ButtonRow
                key={button.index}
                button={button}
                actions={read.actions}
                calibrated={read.calibrated}
                onAction={(it) => act(JOYSTICK_BUTTON_ACTION, button.index, it)}
                onRepeat={(it) => act(JOYSTICK_BUTTON_REPEAT, button.index, it)}
              />
            ))}

            <SectionHeader text="Settings" />
            {present(BASIC_SETTINGS).map((it) => (
              <SettingRow key={it.name} setting={it} onChange={(value) => act(JOYSTICK_SETTING, it.name, value)} />
            ))}
            {read.standbyText !== null ? <Text typography="bodySmall">{read.standbyText}</Text> : null}
            <OutlinedButton onClick={() => setAdvanced(!advanced)} className="self-start">
              Advanced settings
            </OutlinedButton>
            {advanced ? (
              <>
                {present(ADVANCED_SETTINGS).map((it) => (
                  <SettingRow key={it.name} setting={it} onChange={(value) => act(JOYSTICK_SETTING, it.name, value)} />
                ))}
                {setting('useDeadband')?.value === true ? <Text typography="bodySmall">Deadband can be set during the first step of calibration by gently wiggling each axis. </Text> : null}
                <Text typography="titleSmall">MANUAL_CONTROL Extensions</Text>
                {present(EXTENSION_SETTINGS)
                  .slice(0, EXTENSION_LABELS.length)
                  .map((s, index) => (
                    <SettingRow key={s.name} setting={{ ...s, label: EXTENSION_LABELS[index] ?? s.label }} onChange={(value) => act(JOYSTICK_SETTING, s.name, value)} />
                  ))}
                <Text typography="titleSmall">Additional axes</Text>
                {additionalAxes !== null ? (
                  <>
                    <div className="flex items-center">
                      <RadioButton selected={!viaRc} onClick={() => act(JOYSTICK_SETTING, 'additionalAxesFunction', 0)} label="Send using MANUAL_CONTROL" />
                      <Text className="ps-3">Send using MANUAL_CONTROL</Text>
                    </div>
                    <div className="flex items-center">
                      <RadioButton selected={viaRc} onClick={() => act(JOYSTICK_SETTING, 'additionalAxesFunction', 1)} label="Send using RC_CHANNELS_OVERRIDE" />
                      <Text className="ps-3">Send using RC_CHANNELS_OVERRIDE</Text>
                    </div>
                  </>
                ) : null}
                {present(ADDITIONAL_SETTINGS).map((s, index) => (
                  <SettingRow key={s.name} setting={{ ...s, label: viaRc ? `Channel ${index + 5}` : `Aux${index + 1}` }} onChange={(value) => act(JOYSTICK_SETTING, s.name, value)} />
                ))}
              </>
            ) : null}
          </>
        )}
      </div>
      {offerEnable !== null ? (
        <AlertDialog
          onDismissRequest={() => setOfferEnable(null)}
          title="Enable joystick"
          text={`${offerEnable} calibration is complete. Enable it now?`}
          confirmButton={
            <TextButton
              onClick={() => {
                setOfferEnable(null);
                act(JOYSTICK_ENABLE, true);
              }}
            >
              Yes
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setOfferEnable(null)}>No</TextButton>}
        />
      ) : null}
    </>
  );
};

const ButtonRow = ({ button, actions, calibrated, onAction, onRepeat }: { button: JoystickButton; actions: readonly AssignableAction[]; calibrated: boolean; onAction: (action: string) => void; onRepeat: (repeat: boolean) => void }) => {
  const [open, setOpen] = useState(false);
  const canRepeat = actions.find((it) => it.action === button.action)?.canRepeat === true;
  return (
    <div className="flex w-full items-center gap-2">
      <Text typography="titleSmall" color={button.pressed ? md('primary') : md('onSurface')} className="w-8 shrink-0">
        {String(button.index)}
      </Text>
      <div className="relative min-w-0 flex-1">
        <OutlinedButton onClick={() => setOpen(true)} className="w-full">
          <Text typography="labelLarge" maxLines={1}>
            {button.action}
          </Text>
        </OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {actions.map((option) => (
            <DropdownMenuItem
              key={option.action}
              text={option.action}
              onClick={() => {
                setOpen(false);
                onAction(option.action);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
      <Checkbox checked={button.repeat} enabled={canRepeat && calibrated} onCheckedChange={onRepeat} label="Repeat" />
      <Text typography="bodySmall">Repeat</Text>
    </div>
  );
};

const StatusText = ({ status }: { status: StatusLines }) => {
  const { bullets, lines } = status;
  if (lines.length === 0) return null;
  return bullets ? (
    <ul className="flex list-disc flex-col gap-1 ps-5">
      {lines.map((line) => (
        <li key={line}>
          <Text typography="bodyMedium">{line}</Text>
        </li>
      ))}
    </ul>
  ) : (
    <Text typography="bodyMedium">{lines.join(' ')}</Text>
  );
};

const CHECK_ROWS: readonly (readonly [keyof StickValues, string, string])[] = [
  ['roll', 'Left', 'Right'],
  ['pitch', 'Down', 'Up'],
  ['yaw', 'Left', 'Right'],
  ['throttle', 'Down', 'Up'],
];

const StickCheck = ({ live }: { live: StickValues }) => (
  <div className="flex flex-col gap-2">
    {CHECK_ROWS.map(([key, low, high]) => (
      <div key={key} className="flex items-center gap-2">
        <Text className="w-20 shrink-0">{functionLabel(key)}</Text>
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="w-10 shrink-0 text-end">
          {low}
        </Text>
        <LinearProgressIndicator progress={(live[key] + 1) / 2} className="flex-1" label={`${functionLabel(key)} stick`} />
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="w-10 shrink-0">
          {high}
        </Text>
      </div>
    ))}
  </div>
);

const CalibrationPanel = ({ page, onStep }: { page: JoystickPage; onStep: (op: string) => void }) => {
  const cal = page.calibration;
  return (
    <>
      {cal.calibrating && cal.step !== null && cal.steps !== null ? (
        <Text typography="labelLarge" color={md('onSurfaceVariant')}>
          {`Step ${cal.step} of ${cal.steps}`}
        </Text>
      ) : null}
      {cal.calibrating && !cal.checking ? (
        <>
          <StickDiagram positions={cal.stickPositions} live={cal.livePositions} single={cal.singleStick} />
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            Ring: where to move the stick. Dot: where your stick is.
          </Text>
        </>
      ) : null}
      <StatusText status={cal.status} />
      {cal.holding !== null ? (
        <div className="flex items-center gap-2">
          <LinearProgressIndicator progress={cal.holding} className="w-40" label="Holding the stick" />
          <Text typography="bodySmall">Got it, hold it there</Text>
        </div>
      ) : null}
      {cal.checking ? <StickCheck live={cal.live} /> : null}
      {cal.warning !== null ? <Text color={md('error')}>{cal.warning}</Text> : null}
      {cal.checking ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {`Save keeps this calibration in this browser for ${thisSite()}.`}
        </Text>
      ) : null}
      <div className="flex gap-2">
        {cal.calibrating ? (
          <OutlinedButton enabled={cal.cancelEnabled} onClick={() => onStep('cancel')}>
            Cancel
          </OutlinedButton>
        ) : null}
        {cal.oneSidedVisible ? <OutlinedButton onClick={() => onStep('oneSided')}>One-Sided</OutlinedButton> : null}
        <Button enabled={cal.nextEnabled} onClick={() => onStep('next')}>
          {cal.nextText}
        </Button>
      </div>
    </>
  );
};

const TransmitterModeRow = ({ mode, text, onPick }: { mode: number; text: string | null; onPick: (mode: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex items-center gap-3">
      <div className="flex min-w-0 flex-1 flex-col">
        <Text>Stick mode</Text>
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          {text ?? ''}
        </Text>
      </div>
      <div className="relative">
        <OutlinedButton onClick={() => setOpen(true)}>{`Mode ${mode}`}</OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {[1, 2, 3, 4].map((m) => (
            <DropdownMenuItem
              key={m}
              text={`Mode ${m}`}
              onClick={() => {
                setOpen(false);
                onPick(m);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};

const JoystickPicker = ({ page, onPick }: { page: JoystickPage; onPick: (name: string) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative self-start">
      <OutlinedButton onClick={() => setOpen(true)}>{page.active ?? ''}</OutlinedButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {page.names.map((name) => (
          <DropdownMenuItem
            key={name}
            text={name}
            onClick={() => {
              setOpen(false);
              onPick(name);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};

const SettingRow = ({ setting, onChange }: { setting: JoystickSetting; onChange: (value: unknown) => void }) => (
  <div className="flex w-full flex-col">
    <SettingField setting={setting} onChange={onChange} />
    {setting.slider !== null && setting.type !== 'bool' ? (
      <FieldSlider value={typeof setting.value === 'number' ? setting.value : null} slider={setting.slider} label={setting.label} enabled onWrite={onChange} />
    ) : null}
  </div>
);

const SettingField = ({ setting, onChange }: { setting: JoystickSetting; onChange: (value: unknown) => void }) => {
  const [typed, setTyped] = useKeyedState([setting.value], () => (typeof setting.value === 'number' ? String(setting.value) : ''));
  return (
    <div className="flex w-full items-center">
      <Text className="min-w-0 flex-1">{setting.label}</Text>
      {setting.type === 'bool' ? (
        <Switch checked={setting.value === true} onCheckedChange={onChange} label={setting.label} />
      ) : (
        <OutlinedTextField
          value={typed}
          onValueChange={setTyped}
          suffix={setting.units}
          keyboardType="decimal"
          onDone={() => {
            const value = toDoubleOrNull(typed);
            if (value !== null) onChange(value);
          }}
          className="w-[140px]"
        />
      )}
    </div>
  );
};

const STICK_BOX = 72;

export const StickDiagram = ({ positions, live, single }: { positions: readonly number[]; live?: readonly number[]; single: boolean }) => {
  const half = STICK_BOX / 2;
  const stick = (values: readonly number[], at: number) => ({ cx: half + (values[at] ?? 0) * half * 0.75, cy: half - (values[at + 1] ?? 0) * half * 0.75 });
  return (
    <div className="flex gap-6">
      {[0, 2].slice(0, single ? 1 : 2).map((at) => (
        <svg key={at} width={STICK_BOX} height={STICK_BOX} viewBox={`0 0 ${STICK_BOX} ${STICK_BOX}`} aria-hidden>
          <rect x={0.5} y={0.5} width={STICK_BOX - 1} height={STICK_BOX - 1} fill="none" style={{ stroke: md('outline') }} strokeWidth={1} />
          {live === undefined ? (
            <circle {...stick(positions, at)} r={half * 0.18} style={{ fill: md('primary') }} />
          ) : (
            <>
              <circle {...stick(positions, at)} r={half * 0.24} fill="none" style={{ stroke: md('primary') }} strokeWidth={2} strokeDasharray="4 3" />
              <circle {...stick(live, at)} r={half * 0.14} style={{ fill: md('primary') }} />
            </>
          )}
        </svg>
      ))}
    </div>
  );
};
