import { useContext, useEffect, useMemo, useRef, useState } from 'react';
import { type Json, optBoolean, optInt, optText } from '@/bridge/Json';
import { type Fact, Qgc, makeFact } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { VehicleCommands } from '@/bridge/VehicleCommands';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { OutlinedButton, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { aircast, md } from '@/theme/Colors';
import { useAdvancedUiShown } from '@/ui/AdvancedUi';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { FactRow, settingsPagePath, settingsSections } from '@/ui/SettingsScreen';

export const REMOTE_ID_STATUS_PATH = 'view.remoteIdStatus';
export const REMOTE_ID_SETTINGS_PAGE = 'Remote ID';
const SEND_SELF_ID = 'sendSelfID';
const SELF_ID_FACTS = [SEND_SELF_ID, 'selfIDType', 'selfIDFree', 'selfIDExtended', 'selfIDEmergency'];
const BROADCAST_GATED: ReadonlySet<string> = new Set(['selfIDType', 'selfIDFree', 'selfIDExtended']);
const SELF_ID_LABELS: ReadonlyMap<string, string> = new Map([
  [SEND_SELF_ID, 'Broadcast'],
  ['selfIDType', 'Broadcast message'],
]);
const SELF_ID_NOTE = 'If an emergency is declared, Emergency Text will be broadcast even if Broadcast setting is not enabled.';

export const selfIdFacts = (page: readonly Fact[]): readonly Fact[] => {
  const byName = new Map(page.map((fact) => [fact.name, fact] as const));
  const broadcasting = byName.get(SEND_SELF_ID)?.boolValue === true;
  return SELF_ID_FACTS.flatMap((name) => {
    const fact = byName.get(name);
    return fact === undefined ? [] : [fact];
  })
    .map((fact) => {
      const label = SELF_ID_LABELS.get(fact.name);
      return label === undefined ? fact : makeFact({ ...fact, description: label });
    })
    .map((fact) => (BROADCAST_GATED.has(fact.name) && !broadcasting ? makeFact({ ...fact, enabled: false, disabledReason: 'Broadcast is off' }) : fact));
};

export type RemoteIdStatus = {
  state: string;
  comms: boolean;
  arm: boolean;
  armError: string;
  gps: boolean;
  basicId: boolean;
  operatorIdShown: boolean;
  operatorId: boolean;
  emergency: boolean;
  holdMs: number;
};

export const remoteIdStatus = (view: Json | null): RemoteIdStatus | null =>
  view !== null && optBoolean(view, 'shown')
    ? {
        state: optText(view, 'state'),
        comms: optBoolean(view, 'comms'),
        arm: optBoolean(view, 'armStatus'),
        armError: optText(view, 'armStatusError'),
        gps: optBoolean(view, 'gcsGps'),
        basicId: optBoolean(view, 'basicId'),
        operatorIdShown: optBoolean(view, 'operatorIdShown'),
        operatorId: optBoolean(view, 'operatorId'),
        emergency: optBoolean(view, 'emergency'),
        holdMs: optInt(view, 'emergencyHoldMs', 800),
      }
    : null;

type RemoteIdRow = readonly [string, boolean];

export const remoteIdRows = (status: RemoteIdStatus): readonly RemoteIdRow[] => {
  const rows: readonly (RemoteIdRow | null)[] = [
    [status.comms ? 'RID COMMS' : 'NOT CONNECTED', status.comms],
    status.comms ? ['ARM STATUS', status.arm] : null,
    status.comms ? ['GCS GPS', status.gps] : null,
    status.comms ? ['BASIC ID', status.basicId] : null,
    status.comms && status.operatorIdShown ? ['OPERATOR ID', status.operatorId] : null,
  ];
  return rows.filter((row): row is RemoteIdRow => row !== null);
};

const stateColour = (state: string): string => {
  switch (state) {
    case 'healthy':
      return aircast('success');
    case 'warning':
      return aircast('warning');
    case 'error':
      return md('error');
    default:
      return md('onSurfaceVariant');
  }
};

export const RemoteIdIndicatorCell = () => {
  const navigation = useContext(LocalAppNavigation);
  const view = useQgcPath(REMOTE_ID_STATUS_PATH);
  const status = useMemo(() => remoteIdStatus(view), [view]);
  const [open, setOpen] = useState(false);
  const advanced = useAdvancedUiShown();
  const hold = useRef<ReturnType<typeof setTimeout> | null>(null);
  const stopHold = () => {
    if (hold.current !== null) clearTimeout(hold.current);
    hold.current = null;
  };
  useEffect(() => stopHold, [status?.emergency, status?.holdMs]);
  if (status === null) return null;

  const startHold = () => {
    stopHold();
    const declare = !status.emergency;
    hold.current = setTimeout(() => {
      hold.current = null;
      void VehicleCommands.declareRemoteIdEmergency(declare);
    }, status.holdMs);
  };

  const configure = () => {
    navigation.settingsPage.value = REMOTE_ID_SETTINGS_PAGE;
    setOpen(false);
  };

  const emergencyLabel = status.emergency ? 'Clear Emergency' : 'EMERGENCY';

  return (
    <>
      <button type="button" onClick={() => setOpen(true)}>
        <Text typography="labelMedium" color={stateColour(status.state)} maxLines={1}>
          RID
        </Text>
      </button>

      {open ? (
        <AircastSheet onDismissRequest={() => setOpen(false)}>
          <div className="flex w-full flex-col gap-2 overflow-y-auto px-5 pb-6">
            <Text typography="titleMedium">RemoteID Status</Text>
            {remoteIdRows(status).map(([label, good]) => (
              <button key={label} type="button" className="text-left" onClick={configure}>
                <Text typography="labelLarge" color={good ? aircast('success') : md('error')}>
                  {label}
                </Text>
              </button>
            ))}
            {status.armError.trim() !== '' ? <Text typography="bodySmall">{`Arm Status Error  ${status.armError}`}</Text> : null}
            {status.comms ? (
              <>
                <Text typography="bodySmall">{status.emergency ? 'EMERGENCY HAS BEEN DECLARED, Press and Hold for 3 seconds to cancel' : 'Press and Hold below button to declare emergency'}</Text>
                <div
                  role="button"
                  tabIndex={0}
                  aria-label={emergencyLabel}
                  className="flex w-full touch-none select-none items-center justify-center rounded-shape-md bg-error-container p-4"
                  onPointerDown={(event) => {
                    event.currentTarget.setPointerCapture(event.pointerId);
                    startHold();
                  }}
                  onPointerUp={stopHold}
                  onPointerCancel={stopHold}
                  onKeyDown={(event) => {
                    if ((event.key === 'Enter' || event.key === ' ') && !event.repeat) {
                      event.preventDefault();
                      startHold();
                    }
                  }}
                  onKeyUp={stopHold}
                  onBlur={stopHold}
                >
                  <Text typography="titleMedium" color={md('onErrorContainer')}>
                    {emergencyLabel}
                  </Text>
                </div>
              </>
            ) : null}
            <SelfIdSection />
            {advanced ? (
              <div className="flex items-center">
                <Text className="flex-1">Remote ID</Text>
                <OutlinedButton onClick={configure}>Configure</OutlinedButton>
              </div>
            ) : null}
          </div>
        </AircastSheet>
      ) : null}
    </>
  );
};

const SelfIdSection = () => {
  const [reloads, setReloads] = useState(0);
  const [facts, setFacts] = useState<readonly Fact[]>([]);
  useLaunchedEffect([reloads], async (scope) => {
    const loaded = selfIdFacts(settingsSections(await Qgc.get(settingsPagePath(REMOTE_ID_SETTINGS_PAGE))).flatMap((section) => section.blocks.flatMap((block) => block.facts)));
    if (scope.isActive()) setFacts(loaded);
  });
  if (facts.length === 0) return null;
  return (
    <>
      <Text typography="titleSmall">Self ID</Text>
      {facts.find((fact) => fact.name === SEND_SELF_ID)?.boolValue !== true ? <Text typography="bodySmall">{SELF_ID_NOTE}</Text> : null}
      {facts.map((fact) => (
        <FactRow key={fact.path} fact={fact} onWrite={() => setReloads((count) => count + 1)} />
      ))}
    </>
  );
};
