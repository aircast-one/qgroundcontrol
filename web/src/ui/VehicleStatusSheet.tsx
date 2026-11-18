import { type KeyboardEvent, useContext, useMemo, useState } from 'react';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { optBoolean } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { OutlinedButton, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { Drawable, MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { STATUS_SETTINGS_PAGE } from './ActiveVehicle';
import { useAdvancedUiShown } from './AdvancedUi';
import { LocalAppNavigation } from './AppNavigation';
import { SENSOR_HEALTH, sensorHealth } from './CalibrationView';
import { LocalFlyScreenState } from './FlyScreenState';
import { FLY_STATE, type FlyState, flyState, inAir, useInAir } from './FlyState';
import { PayloadFixButton } from './GripperPanel';
import type { GuidedOffer, PayloadProblem } from './GuidedActions';
import { HoldToConfirm } from './HoldToConfirm';
import { useRedIndicators, withRedIndicators } from './OsdDetails';
import { ParameterForm } from './ParameterForm';
import { EDIT_PARAMETER_TITLE, LinkedText, ParameterEditDialog } from './ParameterLinks';
import { AIRCRAFT_SETUP } from './SettingsScreen';
import { SETUP_OVERVIEW_PAGE, SETUP_PARAMETERS_PAGE } from './SetupScreen';
import { type ArmingCheck, MESSAGES, MessageSeverity, WARNINGS, armingChecks, severitySummary, vehicleMessages } from './VehicleMessages';

const SENSOR_FAULT_STATE = 'unhealthy';

export const ARM_REQUEST = 'arm';
export const FORCE_ARM_REQUEST = 'forceArm';

export const ARM_UNAVAILABLE = 'Arming is not available right now.';

export const deckRequestRefusal = (offer: GuidedOffer | null): string => (offer === null || offer.reason.trim() === '' ? ARM_UNAVAILABLE : offer.reason);

export type ArmControls = {
  holdText: string;
  holdEnabled: boolean;
  mayBeRefused: boolean;
  forceLink: boolean;
  forceHold: boolean;
};

export const ArmControls = (holdText: string, holdEnabled: boolean, mayBeRefused: boolean, forceLink: boolean, forceHold: boolean): ArmControls => ({
  holdText,
  holdEnabled,
  mayBeRefused,
  forceLink,
  forceHold,
});

export const armControls = (state: FlyState, forceOpen: boolean): ArmControls =>
  ArmControls(
    state.armed ? 'Slide to disarm' : 'Slide to arm',
    state.canArm,
    !state.armed && !state.nominal && state.canArm && !forceOpen,
    !state.armed && !forceOpen && (!state.canArm || state.fault),
    !state.armed && forceOpen,
  );

export const armSlideShown = (state: FlyState | null): state is FlyState => state !== null && state.connected && !inAir(state);

export const shownSensors = <T extends { state: string }>(sensors: readonly T[], showAll: boolean): readonly T[] => (showAll ? sensors : sensors.filter((it) => it.state === SENSOR_FAULT_STATE));

const ChevronRight = ({ size = 24 }: { size?: number }) => <MaterialIcon name="keyboard_arrow_right" filled size={size} style={{ color: md('onSurfaceVariant') }} />;

export const VehicleStatusSheet = ({ onMessages, onDismiss }: { onMessages: () => void; onDismiss: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const healthJson = useQgcPath(SENSOR_HEALTH);
  const health = useMemo(() => sensorHealth(healthJson), [healthJson]);
  const [showAll, setShowAll] = useState(false);
  const advanced = useAdvancedUiShown();
  const airborne = useInAir();
  useLaunchedEffect([], () => void VehicleCommands.resetAllMessages());
  const open = (page: string) => {
    navigation.setupPage.value = page;
    onDismiss();
  };
  const reading = health !== null && !optBoolean(healthJson, 'healthChecksSupported') && health.available && health.sensors.length > 0 ? health : null;
  const more = reading === null ? 0 : reading.sensors.length - shownSensors(reading.sensors, false).length;

  return (
    <AircastSheet onDismissRequest={onDismiss} label="Vehicle status">
      <div className="flex w-full flex-col gap-1 pb-4">
        <StatusSummary />
        <MessagesRow onClick={onMessages} />
        <ArmSection onDismiss={onDismiss} />
        {reading !== null ? (
          <>
            <Text typography="titleSmall" className="block px-sheet py-1">
              Sensors
            </Text>
            {shownSensors(reading.sensors, showAll).map((sensor) => {
              const fault = sensor.state === SENSOR_FAULT_STATE;
              const look = 'flex w-full items-center gap-2 px-sheet py-1.5 text-left';
              const row = (
                <>
                  <Text typography="bodyMedium" className="flex-1" color={fault ? md('onSurface') : md('onSurfaceVariant')}>
                    {sensor.name}
                  </Text>
                  <Text typography="bodySmall" color={fault ? md('error') : md('onSurfaceVariant')}>
                    {sensor.label}
                  </Text>
                </>
              );
              return airborne ? (
                <div key={sensor.name} className={look}>
                  {row}
                </div>
              ) : (
                <button key={sensor.name} type="button" onClick={() => open(SETUP_OVERVIEW_PAGE)} className={look}>
                  {row}
                  <ChevronRight size={18} />
                </button>
              );
            })}
            {more > 0 ? (
              <TextButton onClick={() => setShowAll(!showAll)} className="mx-3 self-start">
                {showAll ? 'Show less' : `Show ${more} more`}
              </TextButton>
            ) : null}
          </>
        ) : null}
        <OverallStatus />
        {airborne ? null : (
          <TextButton onClick={() => open(SETUP_OVERVIEW_PAGE)} className="mx-3 self-start">
            {AIRCRAFT_SETUP}
          </TextButton>
        )}
        <ParameterForm page={STATUS_SETTINGS_PAGE} />
        {advanced && !airborne
          ? [['Vehicle parameters', SETUP_PARAMETERS_PAGE] as const].map(([label, page]) => (
              <div key={page} className="flex w-full items-center px-sheet py-0.5">
                <Text typography="bodyMedium" className="flex-1">
                  {label}
                </Text>
                <OutlinedButton onClick={() => open(page)}>Configure</OutlinedButton>
              </div>
            ))
          : null}
      </div>
    </AircastSheet>
  );
};

const MessagesRow = ({ onClick }: { onClick: () => void }) => {
  const messagesJson = useQgcPath(MESSAGES);
  const lines = useMemo(() => vehicleMessages(messagesJson), [messagesJson]);
  const tone = lines.some((it) => it.level === MessageSeverity.Error) ? md('error') : lines.some((it) => it.level === MessageSeverity.Warning) ? aircast('warning') : md('onSurfaceVariant');
  return (
    <button type="button" onClick={onClick} className="flex w-full items-center gap-3 px-sheet py-3 text-left">
      <Drawable name="ic_notifications" size={24} />
      <Text className="flex-1">Messages</Text>
      {lines.length > 0 ? (
        <Text typography="bodySmall" color={tone}>
          {severitySummary(lines)}
        </Text>
      ) : null}
      <ChevronRight />
    </button>
  );
};

export const expandedAfterTap = (expanded: ReadonlySet<number>, index: number, check: ArmingCheck): ReadonlySet<number> => {
  if (check.description.trim() === '') return expanded;
  if (expanded.has(index)) return new Set([...expanded].filter((it) => it !== index));
  return new Set([...expanded, index]);
};

const checkColour = (severity: string): string => {
  switch (severity) {
    case 'error':
      return md('error');
    case 'warning':
      return aircast('warning');
    default:
      return md('onSurface');
  }
};

const activate = (event: KeyboardEvent, run: () => void) => {
  if (event.key !== 'Enter' && event.key !== ' ') return;
  event.preventDefault();
  run();
};

const OverallStatus = () => {
  const warnings = useQgcPath(WARNINGS);
  const checks = useMemo(() => armingChecks(warnings) ?? [], [warnings]);
  const [expanded, setExpanded] = useKeyedState<ReadonlySet<number>>([JSON.stringify(checks)], () => new Set());
  const [editing, setEditing] = useState<string | null>(null);
  if (checks.length === 0) return null;
  return (
    <>
      {editing !== null ? <ParameterEditDialog name={editing} title={EDIT_PARAMETER_TITLE} onDismiss={() => setEditing(null)} /> : null}
      <Text typography="titleSmall" className="block px-sheet py-1">
        Overall status
      </Text>
      {checks.map((check, index) => {
        const detailed = check.description.trim() !== '';
        const open = expanded.has(index);
        const toggle = () => setExpanded(expandedAfterTap(expanded, index, check));
        return (
          <div
            key={index}
            role={detailed ? 'button' : undefined}
            tabIndex={detailed ? 0 : undefined}
            aria-expanded={detailed ? open : undefined}
            onClick={detailed ? toggle : undefined}
            onKeyDown={detailed ? (event) => activate(event, toggle) : undefined}
            className={`flex w-full flex-col gap-0.5 px-sheet py-1.5 ${detailed ? 'cursor-pointer' : ''}`}
          >
            <div className="flex items-center">
              <LinkedText html={check.message} style="bodyMedium" color={checkColour(check.severity)} onParameter={setEditing} className="flex-1" />
              {detailed ? <Drawable name="ic_arrow_drop_down" size={24} label={open ? 'Hide details' : 'Show details'} /> : null}
            </div>
            {open ? <LinkedText html={check.description} style="bodySmall" color={md('onSurfaceVariant')} onParameter={setEditing} /> : null}
          </div>
        );
      })}
    </>
  );
};

export type StatusSummaryLines = { readonly title: string; readonly detail: string; readonly problem: PayloadProblem | null };

export const statusSummary = (state: FlyState | null): StatusSummaryLines | null =>
  state === null || state.summaryDetail.trim() === '' ? null : { title: state.stateText, detail: state.summaryDetail, problem: state.summaryProblem };

const StatusSummary = () => {
  const stateJson = useQgcPath(FLY_STATE);
  const state = useMemo(() => flyState(stateJson), [stateJson]);
  const red = useRedIndicators(state);
  const summary = statusSummary(withRedIndicators(state, red));
  const [refusal, setRefusal] = useState<string | null>(null);
  if (summary === null) return null;
  return (
    <div className="flex flex-col px-sheet py-1">
      <Text typography="titleMedium" color={red.length > 0 ? aircast('warning') : undefined}>
        {summary.title}
      </Text>
      <div className="flex items-center gap-2">
        <Text className="flex-1" typography="bodySmall" color={md('onSurfaceVariant')}>
          {summary.detail}
        </Text>
        {summary.problem === null ? null : <PayloadFixButton problem={summary.problem} onRefused={setRefusal} />}
      </div>
      {refusal === null ? null : (
        <Text typography="bodySmall" color={md('error')}>
          {refusal}
        </Text>
      )}
    </div>
  );
};

const ArmSection = ({ onDismiss }: { onDismiss: () => void }) => {
  const stateJson = useQgcPath(FLY_STATE);
  const state = useMemo(() => flyState(stateJson), [stateJson]);
  return armSlideShown(state) ? <ArmButtons state={state} onDismiss={onDismiss} /> : null;
};

const ArmButtons = ({ state, onDismiss }: { state: FlyState; onDismiss: () => void }) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const [forceOpen, setForceOpen] = useState(false);
  const controls = armControls(state, forceOpen);
  const request = (action: string) => {
    flyScreen.deckRequest.value = action;
    onDismiss();
  };
  return (
    <div className="flex w-full flex-col gap-1 px-sheet py-1">
      <HoldToConfirm label={controls.holdText} destructive={state.armed} enabled={controls.holdEnabled} onConfirm={() => request(ARM_REQUEST)} />
      {controls.mayBeRefused ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Arming may be refused.
        </Text>
      ) : null}
      {controls.forceLink ? (
        <TextButton onClick={() => setForceOpen(true)} className="self-start">
          Force arm…
        </TextButton>
      ) : null}
      {controls.forceHold ? <HoldToConfirm label="Slide to force arm" destructive onConfirm={() => request(FORCE_ARM_REQUEST)} /> : null}
    </div>
  );
};
