import { useEffect, useRef, useState } from 'react';
import { AccountCommands } from '@/bridge/BridgeCommands';
import { type Json, optBoolean, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useTicker } from '@/kotlin/Compose';
import { openUri } from '@/kotlin/Platform';
import { OutlinedButton, OutlinedTextField, Text } from '@/m3';

export const AIRCAST_CLOUD_NAME = 'Aircast cloud';
const ACCOUNT = 'account';
const ACCOUNT_POLL_MS = 1000;
const API_BASE = /^https?:\/\/[^/]+/;
export const DEFAULT_ACCOUNT_SERVER = 'https://api.aircast.one';

export type AccountState = { signedIn: boolean; signingIn: boolean; status: string; userCode: string; verificationUrl: string };

export const accountState = (view: Json | null): AccountState | null =>
  optText(view, 'kind') === 'object'
    ? {
        signedIn: optBoolean(view, 'signedIn'),
        signingIn: optBoolean(view, 'signingIn'),
        status: optText(view, 'status'),
        userCode: optText(view, 'userCode'),
        verificationUrl: optText(view, 'verificationUrl'),
      }
    : null;

export const accountLine = (state: AccountState): string => (state.signedIn ? 'Signed in' : state.status.trim() === '' ? 'Not signed in' : state.status);

export const cloudApiBaseValid = (apiBase: string): boolean => API_BASE.test(apiBase.trim());

export const cloudDeviceValid = (deviceId: string): boolean => deviceId.trim() !== '';

export const AircastCloudFields = ({
  apiBase,
  deviceId,
  showErrors,
  onApiBase,
  onDeviceId,
}: {
  apiBase: string;
  deviceId: string;
  showErrors: boolean;
  onApiBase: (value: string) => void;
  onDeviceId: (value: string) => void;
}) => {
  const [account, setAccount] = useState<AccountState | null>(null);
  const opened = useRef('');
  useTicker(ACCOUNT_POLL_MS, [], async () => setAccount(accountState(await Qgc.get(ACCOUNT))));
  const apiBaseError = showErrors && !cloudApiBaseValid(apiBase) ? 'Enter the account server address, starting with https://' : null;
  const deviceError = showErrors && !cloudDeviceValid(deviceId) ? 'Set up from the device to fill this in' : null;
  const pendingUrl = account !== null && account.signingIn && account.verificationUrl.trim() !== '' ? account.verificationUrl : null;
  useEffect(() => {
    if (pendingUrl === null || pendingUrl === opened.current) return;
    opened.current = pendingUrl;
    openUri(pendingUrl);
  }, [pendingUrl]);

  return (
    <div className="flex flex-col gap-form">
      <OutlinedTextField
        value={apiBase}
        onValueChange={onApiBase}
        label="Account server"
        placeholder={DEFAULT_ACCOUNT_SERVER}
        singleLine
        keyboardType="uri"
        isError={apiBaseError !== null}
        supportingText={apiBaseError ?? undefined}
      />
      <OutlinedTextField value={deviceId} onValueChange={onDeviceId} label="Device" singleLine isError={deviceError !== null} supportingText={deviceError ?? undefined} />
      {account !== null ? (
        <>
          <div className="flex w-full items-center">
            <Text className="flex-1">{`Account  ${accountLine(account)}`}</Text>
            {account.signedIn ? (
              <OutlinedButton onClick={() => void AccountCommands.signOut()}>Sign out</OutlinedButton>
            ) : account.signingIn ? (
              <OutlinedButton onClick={() => void AccountCommands.cancelSignIn()}>Cancel</OutlinedButton>
            ) : (
              <OutlinedButton
                onClick={() =>
                  void (async () => {
                    await AccountCommands.setApiBase(apiBase);
                    await AccountCommands.signIn();
                  })()
                }
              >
                Sign in
              </OutlinedButton>
            )}
          </div>
          {account.signingIn ? <Text typography="bodySmall">{`Your browser opened ${account.verificationUrl} — approve code ${account.userCode} there.`}</Text> : null}
        </>
      ) : null}
    </div>
  );
};
