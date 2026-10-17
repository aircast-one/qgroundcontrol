import { useMemo, useState } from 'react';
import { LinkCommands } from '@/bridge/BridgeCommands';
import { type Json, optBoolean, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { settingControl, useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { Button, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';
import { EmptyState } from '@/ui/SetupRow';

const HOST_FACT = 'settings.mavlinkSettings.forwardMavlinkAPMSupportHostName';

export const SUPPORT_HOST_DEBOUNCE_MS = 250;

export const FORWARDING_UNTIL_RESTART = 'Forwarding traffic: Mavlink traffic will keep being forwarded until application restarts';

export type SupportHostVerdict = { valid: boolean; error: string };

export const supportHostPath = (host: string): string => `view.supportHost(${host})`;

export const supportHostCannotBeAsked = (host: string): string | null => {
  if (host.trim() === '') return null;
  if (host !== host.trim()) return 'Remove the space before or after the address.';
  return null;
};

export const supportHostVerdict = (view: Json | null): SupportHostVerdict | null =>
  view === null || optText(view, 'class') !== 'SupportHost' ? null : { valid: optBoolean(view, 'valid'), error: optText(view, 'error') };

export const RemoteSupportScreen = ({ className }: { className?: string }) => {
  const linksJson = useQgcPath('view.links');
  const forwarding = optBoolean(linksJson, 'supportForwarding');
  const json = useQgcPath(settingControl(HOST_FACT));
  const host = useMemo(() => (json !== null && optText(json, 'kind') === 'object' ? factFromControl(json) : null), [json]);
  const [verdict, setVerdict] = useState<SupportHostVerdict | null>(null);
  const typed = host?.valueString ?? '';

  useLaunchedEffect([typed], async (scope) => {
    const refusal = supportHostCannotBeAsked(typed);
    if (refusal !== null) {
      setVerdict({ valid: false, error: refusal });
      return;
    }
    await delay(SUPPORT_HOST_DEBOUNCE_MS);
    if (!scope.isActive()) return;
    const answer = supportHostVerdict(await Qgc.get(supportHostPath(typed)));
    if (scope.isActive()) setVerdict(answer);
  });

  return (
    <div className={`flex size-full flex-col gap-2 overflow-y-auto ${className ?? ''}`}>
      <EmptyState icon="ic_link" title={forwarding ? 'Forwarding to support' : 'Not forwarding'} text="Sends live telemetry, including position, to an ArduPilot support engineer." />

      {host === null ? (
        <Text typography="bodyMedium" className="px-4">
          Reading the support address.
        </Text>
      ) : (
        <>
          <FactRow fact={host} />

          {!forwarding && verdict?.valid !== true ? (
            <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-8">
              {verdict !== null && verdict.error.trim() !== '' ? verdict.error : 'Enter the address your support engineer gave you first.'}
            </Text>
          ) : null}

          <div className="flex w-full justify-end px-4 py-2">
            <Button onClick={() => void LinkCommands.createSupportForwarding()} enabled={!forwarding && verdict?.valid === true}>
              Connect
            </Button>
          </div>
          {forwarding ? (
            <Text typography="bodyMedium" className="px-4">
              {FORWARDING_UNTIL_RESTART}
            </Text>
          ) : null}
        </>
      )}
    </div>
  );
};
