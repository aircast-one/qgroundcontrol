import { useEffect, useMemo, useRef, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { coerceAtLeast, coerceIn, delay } from '@/kotlin/Std';
import { AlertDialog, Button, CircularProgressIndicator, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import type { DrawableName } from '@/res/drawables';
import { Drawable } from '@/res/Icon';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { BlocksNavigation, CALIBRATION_BLOCK } from '@/ui/AppNavigation';
import {
  CALIBRATION,
  CANCEL_WAIT_TEXT,
  CANCEL_WAIT_TITLE,
  COMPASS_ROUTINE,
  type CalibrationRoutine,
  type CalibrationSide,
  type CalibrationState,
  type CompassResult,
  REBOOT_VEHICLE,
  SENSOR_HEALTH,
  calibrationState,
  healthSummary,
  runningTitle,
  sensorHealth,
} from '@/ui/CalibrationView';
import { type FastCompass, type FastCompassChoice, FastCompassBlock, fastCompassArguments, initialFastCompassChoice } from '@/ui/FastCompass';
import { useHasVehicle } from '@/ui/FlyState';
import { CompassOrientations, SensorSettingsBlock } from '@/ui/SensorSettings';
import { FootNote, SectionHeader, SetupRow, SetupState } from '@/ui/SetupRow';

const CAL = 'sensorsCal';
export const SENSOR_FACTORY_RESET = 'sensorSettings.factoryReset';

const CAL_START_MS = 5000;
const CAL_START_POLL_MS = 150;

export const calibrationFailure = (name: string, started: boolean): string | null => (started ? null : `${name} calibration did not start.`);

const calibrationStatus = async (): Promise<string> => calibrationState(await Qgc.get(CALIBRATION))?.statusText ?? '';

const calibrationRunning = async (): Promise<boolean> => calibrationState(await Qgc.get(CALIBRATION))?.inProgress === true;

export const calibrationBegan = (running: boolean, statusBefore: string, statusNow: string): boolean => running || statusNow !== statusBefore;

export type RoutineCopy = { instruction: string; warning: string };

export const RoutineCopy = (fields: { instruction: string; warning?: string }): RoutineCopy => ({ warning: '', ...fields });

export const routineCopy = (routine: CalibrationRoutine): RoutineCopy =>
  RoutineCopy({ instruction: routine.dialogHelp.trim() === '' ? routine.description : routine.dialogHelp, warning: routine.warning });

const SensorsNotice = ({ text, className }: { text: string; className?: string }) => (
  <Text typography="bodyLarge" textAlign="center" className={`w-full p-6 ${className ?? ''}`}>
    {text}
  </Text>
);

export const ACCEL_ROUTINE = 'accelerometer';
export const CALIBRATION_COMPLETE = 'Calibration complete';
const MUST_REBOOT = 'YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION.';
const COMPASS_QUALITY =
  'Shown in the indicator bars is the quality of the calibration for each compass.\n\n' +
  '- Green indicates a well functioning compass.\n' +
  '- Yellow indicates a questionable compass or calibration.\n' +
  '- Red indicates a compass which should not be used.\n\n';

export const PX4_COMPASS_COMPLETE = 'Compass calibration complete';
export const PX4_REBOOT = 'Reboot the vehicle prior to flight.';

export const postCalibrationTitle = (routine: string | null, px4: boolean): string => (px4 && routine === COMPASS_ROUTINE ? PX4_COMPASS_COMPLETE : CALIBRATION_COMPLETE);

export const postCalibrationPrompt = (routine: string | null, helpText: string, completed: string, px4: boolean): string | null =>
  px4
    ? helpText === CALIBRATION_COMPLETE && routine === COMPASS_ROUTINE
      ? PX4_REBOOT
      : null
    : completed === COMPASS_ROUTINE
      ? COMPASS_QUALITY + MUST_REBOOT
      : completed === ACCEL_ROUTINE
        ? MUST_REBOOT
        : null;

const StartDialog = ({
  calibration,
  fast,
  onConfirm,
  onDismiss,
}: {
  calibration: CalibrationRoutine;
  fast: FastCompass | null;
  onConfirm: (invocation: string, args: readonly unknown[]) => void;
  onDismiss: () => void;
}) => {
  const copy = routineCopy(calibration);
  const [simple, setSimple] = useState(false);
  const offersFast = calibration.id === COMPASS_ROUTINE ? fast : null;
  const [fastChoice, setFastChoice] = useKeyedState<FastCompassChoice | null>([offersFast !== null], () => (offersFast === null ? null : initialFastCompassChoice(offersFast)));
  const offersSimple = calibration.id === ACCEL_ROUTINE && calibration.arguments.length > 0;
  const orientationFirst = calibration.id === ACCEL_ROUTINE || calibration.id === COMPASS_ROUTINE;
  const confirm = () => {
    const north = offersFast !== null && fastChoice !== null && fastChoice.enabled ? ([offersFast, fastChoice] as const) : null;
    if (north !== null) onConfirm(north[0].invocation, fastCompassArguments(north[0], north[1]));
    else if (offersSimple) onConfirm(calibration.invocation, [simple]);
    else onConfirm(calibration.invocation, calibration.arguments);
    onDismiss();
  };
  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title={calibration.dialogTitle.trim() === '' ? calibration.title : calibration.dialogTitle}
      text={
        <div className="flex flex-col gap-3 overflow-y-auto">
          {orientationFirst ? (
            <SensorSettingsBlock calibrating showCompasses={calibration.id === COMPASS_ROUTINE} onSimpleAccel={offersSimple ? setSimple : null} />
          ) : null}
          <span className="whitespace-pre-line">{copy.instruction}</span>
          {offersFast !== null && fastChoice !== null ? <FastCompassBlock fast={offersFast} choice={fastChoice} onChange={setFastChoice} /> : null}
          {copy.warning.trim() !== '' ? (
            <Text typography="bodyMedium" color={md('error')} className="whitespace-pre-line">
              {copy.warning}
            </Text>
          ) : null}
        </div>
      }
      confirmButton={<TextButton onClick={confirm}>OK</TextButton>}
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export const positionsText = (sides: readonly CalibrationSide[]): string | null => {
  const shown = sides.filter((it) => it.visible);
  const reached = shown.filter((it) => it.stage === 'done' || it.stage === 'inProgress').length;
  return shown.length > 0 ? `${coerceAtLeast(reached, 1)} of ${shown.length} positions` : null;
};

export type SideImage =
  | 'cal_vehicle_upside_down_rotate'
  | 'cal_vehicle_upside_down'
  | 'cal_vehicle_left_rotate'
  | 'cal_vehicle_left'
  | 'cal_vehicle_right_rotate'
  | 'cal_vehicle_right'
  | 'cal_vehicle_nose_down_rotate'
  | 'cal_vehicle_nose_down'
  | 'cal_vehicle_tail_down_rotate'
  | 'cal_vehicle_tail_down'
  | 'cal_vehicle_down_rotate'
  | 'cal_vehicle_down';

const SIDE_IMAGE_PAIRS = new Map<string, readonly [SideImage, SideImage]>([
  ['UpsideDown', ['cal_vehicle_upside_down', 'cal_vehicle_upside_down_rotate']],
  ['Left', ['cal_vehicle_left', 'cal_vehicle_left_rotate']],
  ['Right', ['cal_vehicle_right', 'cal_vehicle_right_rotate']],
  ['NoseDown', ['cal_vehicle_nose_down', 'cal_vehicle_nose_down_rotate']],
  ['TailDown', ['cal_vehicle_tail_down', 'cal_vehicle_tail_down_rotate']],
]);

const DOWN_IMAGES: readonly [SideImage, SideImage] = ['cal_vehicle_down', 'cal_vehicle_down_rotate'];

export const sideImage = (key: string, rotating: boolean): SideImage => (SIDE_IMAGE_PAIRS.get(key) ?? DOWN_IMAGES)[rotating ? 1 : 0];

const SIDE_IMAGES = import.meta.glob<string>('../../../android/app/src/main/res/drawable-nodpi/cal_vehicle_*.png', { query: '?url', import: 'default', eager: true });

const sideImageUrl = (image: SideImage): string => Object.entries(SIDE_IMAGES).find(([path]) => path.endsWith(`/${image}.png`))?.[1] ?? '';

export const sideStateText = (side: CalibrationSide): string =>
  side.stage === 'inProgress' ? (side.rotate ? 'Rotate' : 'Hold still') : side.stage === 'done' ? 'Done' : 'Pending';

const OrientationGrid = ({ sides, px4 }: { sides: readonly CalibrationSide[]; px4: boolean }) => (
  <div className="flex flex-wrap gap-2">
    {sides
      .filter((it) => it.visible)
      .map((side) => {
        const current = side.stage === 'inProgress';
        const done = side.stage === 'done';
        const tint = current ? md('primary') : done ? aircast('success') : withAlpha(md('onSurface'), 0.5);
        return (
          <div
            key={side.key}
            className="flex w-[104px] flex-col items-center rounded-shape-md p-1.5"
            style={{ border: `2px solid ${current || done ? tint : md('outlineVariant')}` }}
          >
            <img src={sideImageUrl(sideImage(side.key, px4 && current && side.rotate))} alt={side.title} className="h-[72px] object-contain" />
            <Text typography="labelSmall">{side.title}</Text>
            <Text typography="labelMedium" color={tint} fontWeight={current ? 'bold' : undefined}>
              {sideStateText(side)}
            </Text>
          </div>
        );
      })}
  </div>
);

const CALIBRATION_RING = 180;

const ROUTINE_ICONS = new Map<string, DrawableName>([
  ['accelerometer', 'ic_vibration'],
  ['compass', 'ic_explore'],
  ['compassMot', 'ic_explore'],
  ['levelHorizon', 'ic_straighten'],
  ['gyro', 'ic_sensors'],
  ['pressure', 'ic_height'],
  ['airspeed', 'ic_speed'],
]);

export const routineIcon = (id: string): DrawableName => ROUTINE_ICONS.get(id) ?? 'ic_build';

const RunningCalibration = ({ name, state, className }: { name: string; state: CalibrationState; className?: string }) => {
  const progress = state.progress;
  const helpText = state.helpText;
  const statusText = state.statusText;
  const positions = state.showsSides ? positionsText(state.sides) : null;

  return (
    <div className={`flex h-full w-full flex-col gap-4 overflow-y-auto p-4 ${className ?? ''}`}>
      {positions !== null ? (
        <Text typography="labelLarge" color={md('primary')}>
          {positions}
        </Text>
      ) : null}
      <Text typography="titleLarge">{runningTitle(name)}</Text>

      {helpText.trim() !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="whitespace-pre-line">
          {helpText}
        </Text>
      ) : null}

      <div className="relative grid w-full place-items-center py-2">
        <CircularProgressIndicator progress={coerceIn(progress, 0, 1)} size={CALIBRATION_RING} strokeWidth={10} trackColor={md('surfaceContainerHighest')} label={runningTitle(name)} />
        <div className="absolute flex flex-col items-center">
          <Drawable name="ic_sensors" size={40} style={{ color: md('primary') }} />
          <Text typography="headlineMedium">{`${Math.trunc(progress * 100)}%`}</Text>
        </div>
      </div>

      {state.showsSides ? <OrientationGrid sides={state.sides} px4={state.px4} /> : null}

      {statusText.trim() !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="whitespace-pre-line">
          {statusText}
        </Text>
      ) : null}

      <div className="flex w-full justify-end gap-2">
        <TextButton onClick={() => void Qgc.invoke(`${CAL}.cancelCalibration`)} enabled={state.cancelEnabled}>
          Cancel
        </TextButton>
        <Button onClick={() => void Qgc.invoke(`${CAL}.nextClicked`)} enabled={state.nextEnabled}>
          Next
        </Button>
      </div>
      {state.waitingForCancel ? (
        <>
          <Text typography="titleSmall">{CANCEL_WAIT_TITLE}</Text>
          <Text typography="bodyMedium">{CANCEL_WAIT_TEXT}</Text>
        </>
      ) : null}
    </div>
  );
};

const waitForStart = async (before: string, deadline: number): Promise<boolean> => {
  if (calibrationBegan(await calibrationRunning(), before, await calibrationStatus())) return true;
  if (Date.now() + CAL_START_POLL_MS > deadline) return false;
  await delay(CAL_START_POLL_MS);
  return waitForStart(before, deadline);
};

const setupState = (status: string): SetupState => (status === 'Not calibrated' ? SetupState.NeedsAttention : status === 'Calibrated' ? SetupState.Done : SetupState.Neutral);

const healthState = (state: string): SetupState => (state === 'healthy' ? SetupState.Done : state === 'unhealthy' ? SetupState.NeedsAttention : SetupState.Neutral);

export const SensorsScreen = ({ className }: { className?: string }) => {
  const hasVehicle = useHasVehicle();
  const json = useQgcPath(CALIBRATION);
  const healthJson = useQgcPath(SENSOR_HEALTH);
  const health = useMemo(() => sensorHealth(healthJson), [healthJson]);
  const state = useMemo(() => calibrationState(json), [json]);
  const [pending, setPending] = useState<CalibrationRoutine | null>(null);
  const [showSettings, setShowSettings] = useState(false);
  const [confirmFactoryReset, setConfirmFactoryReset] = useState(false);
  const [runningName, setRunningName] = useState('');
  const [ranRoutine, setRanRoutine] = useState<string | null>(null);
  const [rebootPrompt, setRebootPrompt] = useState<string | null>(null);
  const [wasInProgress, setWasInProgress] = useState(false);
  const [notice, setNotice] = useState<string | null>(null);
  const shown = hasVehicle ? state : null;
  const running = shown?.inProgress === true;
  const stillRunning = useRef(false);
  if (shown !== null) stillRunning.current = running;

  useLaunchedEffect([shown !== null, running], () => {
    if (shown === null) return;
    if (wasInProgress && !shown.inProgress) setRebootPrompt(postCalibrationPrompt(ranRoutine, shown.helpText, shown.completed, shown.px4));
    setWasInProgress(shown.inProgress);
  });

  useEffect(
    () =>
      shown === null
        ? undefined
        : () => {
            if (stillRunning.current) void Qgc.invoke(`${CAL}.cancelCalibration`);
          },
    [shown !== null],
  );

  if (!hasVehicle) return <SensorsNotice text="Connect a vehicle to calibrate its sensors." className={className} />;

  if (state === null) return <SensorsNotice text="Reading the vehicle's calibration state." className={className} />;

  const confirmStart = (calibration: CalibrationRoutine) => (invocation: string, args: readonly unknown[]) => {
    setRunningName(calibration.title);
    setRanRoutine(calibration.id);
    setNotice(null);
    void (async () => {
      const before = await calibrationStatus();
      const dispatched = await Qgc.invoke(invocation, ...args);
      const started = dispatched && (await waitForStart(before, Date.now() + CAL_START_MS));
      setNotice(calibrationFailure(calibration.title, started));
    })();
  };

  return (
    <>
      {rebootPrompt !== null ? (
        <AlertDialog
          onDismissRequest={() => setRebootPrompt(null)}
          title={postCalibrationTitle(ranRoutine, state.px4)}
          text={
            <div className="flex flex-col gap-2 overflow-y-auto">
              <span className="whitespace-pre-line">{rebootPrompt}</span>
              {!state.px4 && state.completed === COMPASS_ROUTINE ? state.compassResults.map((it) => <CompassFitnessBar key={it.compass} result={it} />) : null}
              {state.px4 && ranRoutine === COMPASS_ROUTINE ? <CompassOrientations /> : null}
            </div>
          }
          confirmButton={
            <TextButton
              onClick={() => {
                setRebootPrompt(null);
                void Qgc.invoke(REBOOT_VEHICLE);
              }}
            >
              Reboot vehicle
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setRebootPrompt(null)}>Close</TextButton>}
        />
      ) : null}

      {showSettings ? (
        <AlertDialog
          onDismissRequest={() => setShowSettings(false)}
          title={state.settingsDialogTitle.trim() === '' ? state.settingsTitle : state.settingsDialogTitle}
          text={
            <div className="flex flex-col overflow-y-auto">
              <SensorSettingsBlock calibrating={false} showCompasses />
            </div>
          }
          confirmButton={<TextButton onClick={() => setShowSettings(false)}>OK</TextButton>}
        />
      ) : null}

      {confirmFactoryReset ? (
        <AlertDialog
          onDismissRequest={() => setConfirmFactoryReset(false)}
          title="Factory reset"
          text="Reset every parameter on the vehicle to its factory default?"
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirmFactoryReset(false);
                void (async () => setNotice(await Qgc.refusalOf(SENSOR_FACTORY_RESET)))();
              }}
            >
              Reset
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirmFactoryReset(false)}>Cancel</TextButton>}
        />
      ) : null}

      {pending !== null ? <StartDialog calibration={pending} fast={state.fastCompass} onConfirm={confirmStart(pending)} onDismiss={() => setPending(null)} /> : null}

      <BlocksNavigation blocking={running && state.px4} reason={CALIBRATION_BLOCK} />

      {running ? (
        <RunningCalibration name={runningName} state={state} className={className} />
      ) : (
        <div className={`flex h-full w-full flex-col overflow-y-auto ${className ?? ''}`}>
          {health !== null && health.available && health.sensors.length > 0 ? (
            <div className="flex flex-col">
              <SectionHeader text="Sensor health" />
              {healthSummary(health).trim() !== '' ? (
                <Text typography="bodyMedium" color={md('error')} className="block px-5 py-1">
                  {healthSummary(health)}
                </Text>
              ) : null}
              {health.sensors.map((sensor) => (
                <SetupRow key={sensor.name} title={sensor.name} status={sensor.label} state={healthState(sensor.state)} icon="ic_sensors" />
              ))}
            </div>
          ) : null}

          <SectionHeader text="Calibration" />

          {notice !== null ? (
            <Text typography="bodySmall" color={md('error')} className="block px-5 py-2">
              {notice}
            </Text>
          ) : null}

          {state.routines.map((routine) => (
            <SetupRow
              key={routine.id}
              title={sentenceCase(routine.title)}
              status={routine.spinsPropeller ? 'Spins the motors' : routine.status}
              state={setupState(routine.status)}
              onClick={routine.enabled ? () => setPending(routine) : null}
              icon={routineIcon(routine.id)}
            />
          ))}

          <SetupRow title={sentenceCase(state.settingsTitle)} status="" state={SetupState.Neutral} onClick={() => setShowSettings(true)} icon="ic_tune" />

          {state.px4 ? <SetupRow title="Factory reset" status="" state={SetupState.NeedsAttention} onClick={() => setConfirmFactoryReset(true)} icon="ic_delete" /> : null}

          {state.statusText.trim() !== '' ? (
            <div className="flex flex-col">
              <SectionHeader text="Last calibration" />
              <Text typography="bodyMedium" monospace className="block whitespace-pre-line px-5">
                {state.statusText}
              </Text>
            </div>
          ) : null}

          <FootNote text="Calibrate where the aircraft will fly, away from metal, with the propellers off. CompassMot is the exception and says so when you open it: it runs the motors, with the propellers inverted." />
        </div>
      )}
    </>
  );
};

const CompassFitnessBar = ({ result }: { result: CompassResult }) => {
  const dot = 11;
  return (
    <div className="flex flex-col gap-0.5">
      <Text typography="labelMedium">{`Compass ${result.compass}`}</Text>
      <div className="relative h-4 w-full">
        <div className="flex h-full w-full">
          <div className="h-full" style={{ flex: `${result.green / result.range} 1 0`, background: '#008000' }} />
          <div className="h-full" style={{ flex: `${(result.yellow - result.green) / result.range} 1 0`, background: Color.Yellow }} />
          <div className="h-full" style={{ flex: `${(result.range - result.yellow) / result.range} 1 0`, background: Color.Red }} />
        </div>
        <div
          className="absolute rounded-full"
          style={{ left: `calc(${result.position * 100}% - ${dot / 2}px)`, top: (16 - dot) / 2, width: dot, height: dot, background: Color.White, border: `1px solid ${Color.Black}` }}
        />
      </div>
    </div>
  );
};
