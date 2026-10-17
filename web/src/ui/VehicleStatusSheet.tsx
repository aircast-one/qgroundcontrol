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
import { FLY_STATE, type FlyState, flyState } from './FlyState';
import type { GuidedOffer } from './GuidedActions';
import { HoldToConfirm } from './HoldToConfirm';
import { ParameterForm } from './ParameterForm';
import { EDIT_PARAMETER_TITLE, LinkedText, ParameterEditDialog } from './ParameterLinks';
import { AIRCRAFT_SETUP } from './SettingsScreen';
import { SETUP_OVERVIEW_PAGE, SETUP_PARAMETERS_PAGE } from './SetupScreen';
import { type ArmingCheck, MESSAGES, MessageLine, WARNINGS, armingChecks, vehicleMessages } from './VehicleMessages';

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

export const SENSOR_HEALTHY_STATE = 'healthy';

export const messagesToggleText = (shown: boolean): string => (shown ? 'Hide messages' : 'Show messages');

export const shownSensors = <T extends { state: string }>(sensors: readonly T[], showAll: boolean): readonly T[] => (showAll ? sensors : sensors.filter((it) => it.state !== SENSOR_HEALTHY_STATE));

const ChevronRight = ({ size = 24 }: { size?: number }) => <MaterialIcon name="keyboard_arrow_right" filled size={size} style={{ color: md('onSurfaceVariant') }} />;

export const VehicleStatusSheet = ({ onMessages, onDismiss }: { onMessages: () => void; onDismiss: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const healthJson = useQgcPath(SENSOR_HEALTH);
  const health = useMemo(() => sensorHealth(healthJson), [healthJson]);
  const [showAll, setShowAll] = useState(false);
  const advanced = useAdvancedUiShown();
  const open = (page: string) => {
    navigation.setupPage.value = page;
    onDismiss();
  };
  const reading = health !== null && !optBoolean(healthJson, 'healthChecksSupported') && health.available && health.sensors.length > 0 ? health : null;
  const normal = reading === null ? 0 : reading.sensors.filter((it) => it.state === SENSOR_HEALTHY_STATE).length;

  return (
    <AircastSheet onDismissRequest={onDismiss}>
      <div className="flex w-full flex-col gap-1 pb-4">
        <StatusSummary />
        <button type="button" onClick={onMessages} className="flex w-full items-center gap-3 px-5 py-3 text-left">
          <Drawable name="ic_notifications" size={24} />
          <Text className="flex-1">Messages</Text>
          <ChevronRight />
        </button>
        <ArmSection onDismiss={onDismiss} />
        {reading !== null ? (
          <>
            <Text typography="titleSmall" className="block px-5 py-1">
              Sensors
            </Text>
            {shownSensors(reading.sensors, showAll).map((sensor) => {
              const fault = sensor.state === SENSOR_FAULT_STATE;
              return (
                <button key={sensor.name} type="button" onClick={() => open(SETUP_OVERVIEW_PAGE)} className="flex w-full items-center gap-2 px-5 py-1.5 text-left">
                  <Text typography="bodyMedium" className="flex-1" color={fault ? md('onSurface') : md('onSurfaceVariant')}>
                    {sensor.name}
                  </Text>
                  <Text typography="bodySmall" color={fault ? md('error') : md('onSurfaceVariant')}>
                    {sensor.label}
                  </Text>
                  <ChevronRight size={18} />
                </button>
              );
            })}
            {normal > 0 ? (
              <TextButton onClick={() => setShowAll(!showAll)} className="mx-3 self-start">
                {showAll ? 'Show less' : `Show ${normal} more`}
              </TextButton>
            ) : null}
          </>
        ) : null}
        <StatusMessages />
        <OverallStatus />
        <TextButton onClick={() => open(SETUP_OVERVIEW_PAGE)} className="mx-3 self-start">
          {AIRCRAFT_SETUP}
        </TextButton>
        <ParameterForm page={STATUS_SETTINGS_PAGE} className="max-h-[360px] overflow-auto" />
        {advanced
          ? [['Vehicle parameters', SETUP_PARAMETERS_PAGE] as const].map(([label, page]) => (
              <div key={page} className="flex w-full items-center px-5 py-0.5">
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

const StatusMessages = () => {
  const messagesJson = useQgcPath(MESSAGES);
  const lines = useMemo(() => vehicleMessages(messagesJson).toReversed(), [messagesJson]);
  const [shown, setShown] = useState(false);
  useLaunchedEffect([], () => void VehicleCommands.resetAllMessages());
  if (lines.length === 0) return null;
  return (
    <>
      <div className="flex w-full items-center px-5">
        <Text typography="titleSmall" className="flex-1">
          Messages
        </Text>
        {shown ? <TextButton onClick={() => void VehicleCommands.clearMessages()}>Clear</TextButton> : null}
      </div>
      <TextButton onClick={() => setShown(!shown)} className="mx-3 self-start">
        {messagesToggleText(shown)}
      </TextButton>
      {shown ? (
        <div className="max-h-[320px] overflow-auto px-5">
          {lines.map((message, at) => (
            <MessageLine key={at} level={message.level} time={message.time}>
              <Text typography="bodyMedium">{message.text}</Text>
            </MessageLine>
          ))}
        </div>
      ) : null}
    </>
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
      <Text typography="titleSmall" className="block px-5 py-1">
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
            className={`flex w-full flex-col gap-0.5 px-5 py-1.5 ${detailed ? 'cursor-pointer' : ''}`}
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

const StatusSummary = () => {
  const stateJson = useQgcPath(FLY_STATE);
  const lines = useMemo(() => {
    const state = flyState(stateJson);
    return state === null ? null : ([state.stateText, state.summaryDetail] as const);
  }, [stateJson]);
  if (lines === null || lines[1].trim() === '') return null;
  const [title, detail] = lines;
  return (
    <div className="flex flex-col px-5 py-1">
      <Text typography="titleMedium">{title}</Text>
      <Text typography="bodySmall" color={md('onSurfaceVariant')}>
        {detail}
      </Text>
    </div>
  );
};

const ArmSection = ({ onDismiss }: { onDismiss: () => void }) => {
  const stateJson = useQgcPath(FLY_STATE);
  const state = useMemo(() => flyState(stateJson), [stateJson]);
  return state !== null && state.connected ? <ArmButtons state={state} onDismiss={onDismiss} /> : null;
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
    <div className="flex w-full flex-col gap-1 px-5 py-1">
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
