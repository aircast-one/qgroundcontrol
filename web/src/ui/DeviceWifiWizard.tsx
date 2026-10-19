import { useState } from 'react';
import { isJson, jsonObjects, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { FilledTonalButton, OutlinedButton, OutlinedTextField, Text } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';
import { DeviceDisclosure, SecretField } from '@/ui/DeviceKit';
import { useValue } from '@/kotlin/State';
import { deviceServed } from '@/app/DeviceLink';
import { type DeviceRefusal, NO_ANSWER, deviceCall, deviceRead } from '@/ui/DeviceView';

export type WifiNetwork = { readonly ssid: string; readonly signal: number; readonly secured: boolean; readonly active: boolean };

export type WifiReopen = { readonly host: string; readonly hostname: string };

export type AppPlace = { readonly pathname: string; readonly hash: string };

export type WifiJoin = { readonly attempt: number; readonly target: string; readonly status: string; readonly instructions: boolean };

type JoinProblem = { readonly message: string; readonly field: 'ssid' | 'psk' };

export const parseWifiScan = (json: unknown): readonly WifiNetwork[] | null =>
  isJson(json)
    ? jsonObjects(optJSONArray(json, 'networks')).map((it) => ({ ssid: optText(it, 'ssid'), signal: optInt(it, 'signal'), secured: optBoolean(it, 'secured'), active: optBoolean(it, 'active') }))
    : null;

export const wifiJoinProblem = (ssid: string, psk: string, pickedSecured: string): JoinProblem | null => {
  const name = ssid.trim();
  if (name === '') return { message: 'Enter your network name first.', field: 'ssid' };
  if (psk === '' && pickedSecured !== '' && pickedSecured === name) return { message: `“${name}” is a secured network — enter its password.`, field: 'psk' };
  if (psk !== '' && (psk.length < 8 || psk.length > 63)) return { message: 'Password must be 8–63 characters, or blank for an open network.', field: 'psk' };
  return null;
};

export const joinOutcome = (join: WifiJoin, refusal: DeviceRefusal | null): WifiJoin =>
  refusal === null
    ? { ...join, status: `✓ Connected to “${join.target}” — it's now the active link.` }
    : refusal === NO_ANSWER
      ? { ...join, status: "Lost contact with the device — it may be switching networks. If it doesn't come back, reconnect using the steps below." }
      : { ...join, instructions: false, status: `✗ ${refusal.message || "Couldn't join that network."}` };

export const reopenLink = (host: string, served: boolean, at: AppPlace): string => (host === '' ? '' : served ? `http://${host}${at.pathname}${at.hash}` : `http://${host}`);

export const reopenLabel = (url: string): string => (url === '' ? "the device's address on your network" : url.replace(/^https?:\/\//, ''));

const NO_JOIN: WifiJoin = { attempt: 0, target: '', status: '', instructions: false };

export const WifiWizard = ({ reopen }: { reopen: WifiReopen }) => {
  const [joining, setJoining] = useState(false);
  const [join, setJoin] = useState<WifiJoin>(NO_JOIN);
  const [ssid, setSsid] = useState('');
  const [psk, setPsk] = useState('');
  const [pickedSecured, setPickedSecured] = useState('');
  const [problem, setProblem] = useState<JoinProblem | null>(null);
  const [scanning, setScanning] = useState(false);
  const [networks, setNetworks] = useState<readonly WifiNetwork[] | null | 'failed'>(null);
  const scan = () => {
    setScanning(true);
    void deviceRead('/api/network/wifi/scan', parseWifiScan).then((found) => {
      setScanning(false);
      setNetworks(found ?? 'failed');
    });
  };
  const pick = (network: WifiNetwork) => {
    setSsid(network.ssid);
    setPsk('');
    setPickedSecured(network.secured ? network.ssid : '');
  };
  const connect = () => {
    const refused = wifiJoinProblem(ssid, psk, pickedSecured);
    setProblem(refused);
    if (refused !== null) return;
    const target = ssid.trim();
    const attempt = join.attempt + 1;
    setJoin({ attempt, target, status: `Switching to “${target}”…`, instructions: true });
    setJoining(true);
    void deviceCall('POST', '/api/network/wifi', { ssid: target, psk }).then((refusal) => setJoin((current) => (current.attempt === attempt ? joinOutcome(current, refusal) : current)));
  };
  const back = () => {
    setJoining(false);
    setJoin((current) => ({ ...current, attempt: current.attempt + 1 }));
  };
  if (joining) return <WifiJoining join={join} reopen={reopen} onBack={back} />;
  return (
    <div className="flex flex-col gap-3 px-list py-2" role="group" aria-label="Join a Wi-Fi network">
      <div className="flex items-center gap-3">
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="min-w-0 flex-1">
          Pick a nearby network or type its name. Scanning is limited while the hotspot is on, so typing is usually fastest.
        </Text>
        <OutlinedButton enabled={!scanning} onClick={scan}>
          {scanning ? 'Scanning…' : 'Scan'}
        </OutlinedButton>
      </div>
      {networks === 'failed' ? <Hint text="Scan failed — type your network name below." /> : null}
      {networks !== null && networks !== 'failed' && networks.length === 0 ? <Hint text="No networks found — type your network name below." /> : null}
      {networks !== null && networks !== 'failed' && networks.length > 0 ? (
        <ul aria-label="Nearby networks" className="flex flex-col gap-1.5">
          {networks.map((network) => (
            <li key={network.ssid}>
              <button
                type="button"
                onClick={() => pick(network)}
                className="flex min-h-12 w-full items-center justify-between gap-3 rounded-shape-md border border-outline-variant px-4 text-left hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]"
              >
                <span className="flex min-w-0 items-center gap-2">
                  <Text typography="bodyLarge" maxLines={1} ellipsis>
                    {network.ssid}
                  </Text>
                  {network.active ? (
                    <Text typography="labelSmall" color={aircast('success')}>
                      connected
                    </Text>
                  ) : null}
                </span>
                <span className="flex shrink-0 items-center gap-2">
                  <Text typography="bodySmall" color={md('onSurfaceVariant')}>{`${network.signal}%`}</Text>
                  {network.secured ? <MaterialIcon name="lock" size={18} label="secured" style={{ color: md('onSurfaceVariant') }} /> : null}
                </span>
              </button>
            </li>
          ))}
        </ul>
      ) : null}
      <div className="grid gap-3 sm:grid-cols-2">
        <OutlinedTextField
          label="Network name (SSID)"
          ariaLabel="Network name (SSID)"
          value={ssid}
          placeholder="your Wi-Fi name"
          autoCorrect={false}
          isError={problem?.field === 'ssid'}
          onValueChange={setSsid}
          onDone={connect}
        />
        <SecretField label="Password" value={psk} isError={problem?.field === 'psk'} supportingText="Leave blank for an open network." onValueChange={setPsk} />
      </div>
      {problem !== null ? (
        <div role="alert">
          <Text typography="bodyMedium" color={md('error')}>
            {problem.message}
          </Text>
        </div>
      ) : null}
      <div className="flex flex-wrap items-center justify-between gap-3">
        <Text typography="bodySmall" color={md('onSurfaceVariant')}>
          Saved with autoconnect, ahead of the hotspot.
        </Text>
        <FilledTonalButton onClick={connect}>Connect</FilledTonalButton>
      </div>
    </div>
  );
};

export const WifiJoin = ({ reopen, initiallyOpen = false }: { reopen: WifiReopen; initiallyOpen?: boolean }) => (
  <DeviceDisclosure title="Join a Wi-Fi network" summary="Connect this device to your Wi-Fi. It leaves the recovery hotspot and rejoins on every boot." initiallyOpen={initiallyOpen}>
    <WifiWizard reopen={reopen} />
  </DeviceDisclosure>
);

const Hint = ({ text }: { text: string }) => (
  <Text typography="bodySmall" color={md('onSurfaceVariant')}>
    {text}
  </Text>
);

const WifiJoining = ({ join, reopen, onBack }: { join: WifiJoin; reopen: WifiReopen; onBack: () => void }) => {
  const link = reopenLink(reopen.host, useValue(deviceServed), window.location);
  return (
    <div className="flex flex-col gap-3 px-list py-2" role="group" aria-label="Joining a Wi-Fi network">
      <div role="status">
        <Text typography="bodyLarge">{join.status}</Text>
      </div>
      {join.instructions ? (
        <div role="note" aria-label="How to reach the device again" className="flex flex-col gap-2 rounded-shape-md bg-surface-container-high px-4 py-3">
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            The recovery hotspot is turning off. To reach the device again:
          </Text>
          <ol className="ml-5 list-decimal">
            <li>
              <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
                On your phone or laptop, join <strong className="text-on-surface">{join.target}</strong>.
              </Text>
            </li>
            <li>
              <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
                Open{' '}
                {link === '' ? (
                  reopenLabel(link)
                ) : (
                  <a href={link} target="_blank" rel="noopener" className="font-mono break-all text-on-surface underline">
                    {reopenLabel(link)}
                  </a>
                )}{' '}
                on that network.
              </Text>
            </li>
          </ol>
          {link !== '' ? (
            <Hint text={`Doesn't open? Android and some Windows PCs can't open .local names — find ${reopen.hostname} in your router's list of connected devices and open its address instead.`} />
          ) : null}
          <Hint text="This page is served over the link that's going away, so it can't refresh itself." />
        </div>
      ) : null}
      <div>
        <OutlinedButton onClick={onBack}>← Back</OutlinedButton>
      </div>
    </div>
  );
};
