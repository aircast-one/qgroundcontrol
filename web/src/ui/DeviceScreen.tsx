import { useContext, useEffect, useState } from 'react';
import { useBackHandler, useKeyedState } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { FilledTonalButton, OutlinedTextField, Surface, Text, TextButton } from '@/m3';
import { DeviceLink, deviceAddress, deviceConnected, deviceServed, telemetryReachable } from '@/app/DeviceLink';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { useOsdBackdrop } from '@/ui/Osd';
import { useFlyIsPortrait } from '@/ui/FlyShell';
import { MaterialIcon } from '@/res/Icon';
import { FootNote, OverridePageHeading, SectionHeader, SetupRow, SetupState } from '@/ui/SetupRow';
import { type DeviceHealth, type DeviceStatus, type DeviceUplink, deviceAnswered, deviceNeedsAttention, devicePageIcon, deviceRequested, deviceStatus } from '@/ui/DeviceStatus';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { DEVICE_PAGE } from '@/ui/DevicePage';
import { DEVICE_SCREENS, type DeviceScreenEntry, devicePath } from '@/ui/DeviceScreens';
import { DeviceChart, DeviceFact, DeviceNotice, DeviceSection, DeviceText, levelColor } from '@/ui/DeviceKit';

const checkState = (level: number): SetupState => (level === DeviceLevel.Ready ? SetupState.Done : SetupState.NeedsAttention);

export const DeviceTab = ({ open, onOpen, onClose }: { open: string | null; onOpen: (href: string) => void; onClose: () => void }) => {
  const status = useValue(deviceStatus);
  const served = useValue(deviceServed);
  const connected = useValue(deviceConnected);
  const requested = useValue(deviceRequested);
  const [stack, setStack] = useKeyedState<readonly string[]>([open], () => (open === null ? [] : [open]));
  useEffect(() => {
    if (requested === null) return;
    deviceRequested.value = null;
    onOpen(requested);
  }, [requested]);
  const current = stack.at(-1) ?? null;
  const back = () => (stack.length > 1 ? setStack(stack.slice(0, -1)) : onClose());
  const opened = current === null ? null : devicePath(current);
  const entry = opened === null ? undefined : DEVICE_SCREENS.get(opened.path);
  if (current !== null && opened !== null && entry !== undefined)
    return <NativeDevicePage key={current} entry={entry} href={current} query={opened.query} onOpen={(href) => setStack([...stack, href])} onBack={back} />;
  if (!served && connected === null) return <ConnectDevice />;
  if (status === null)
    return (
      <div className="h-full w-full overflow-y-auto">
        <FootNote text="Waiting for the device to answer…" />
        {connected !== null ? <HostedDevice origin={connected} /> : null}
      </div>
    );
  return (
    <div className="h-full w-full overflow-y-auto">
      <div className="flex flex-col gap-1 px-list pt-4 pb-2">
        <Text typography="titleMedium" color={levelColor(status.level)}>
          {status.headline}
        </Text>
        {status.detail !== '' ? (
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {status.detail}
          </Text>
        ) : null}
        {status.action !== null ? (
          <div className="pt-2">
            <FilledTonalButton onClick={() => status.action !== null && onOpen(status.action.href)}>{status.action.label}</FilledTonalButton>
          </div>
        ) : null}
      </div>
      {status.setup !== null ? <SetupRow title={status.setup.label} icon="ic_build" state={SetupState.NeedsAttention} onClick={() => status.setup !== null && onOpen(status.setup.href)} /> : null}
      <SectionHeader text="Before you fly" />
      {status.checks.map((check) => (
        <SetupRow
          key={check.label}
          title={check.label}
          status={check.value}
          state={checkState(check.level)}
          onClick={check.href !== '' ? () => onOpen(check.href) : null}
        />
      ))}
      <SetupRow title={status.events !== '' ? status.events : 'Flight log'} status="" state={status.events !== '' ? SetupState.NeedsAttention : SetupState.Neutral} onClick={() => onOpen('/flights')} />
      {status.link !== null ? <UplinkSection link={status.link} /> : null}
      {status.health !== null ? <HealthSection health={status.health} /> : null}
      <SectionHeader text={status.name !== '' ? status.name : 'Device'} />
      {status.pages.map((page) => (
        <SetupRow
          key={page.href}
          title={page.title}
          status={page.summary}
          icon={devicePageIcon(page.icon)}
          state={page.needs ? SetupState.NeedsAttention : SetupState.Neutral}
          onClick={() => onOpen(page.href)}
        />
      ))}
      {connected !== null ? <HostedDevice origin={connected} /> : null}
    </div>
  );
};

const ConnectDevice = () => {
  const [address, setAddress] = useState('');
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState('');
  const connect = () => {
    const origin = deviceAddress(address);
    if (origin === null) {
      setRefusal('Type an address like 192.168.1.100 or falcon.local.');
      return;
    }
    setBusy(true);
    void DeviceLink.connect(origin).then((refused) => {
      setBusy(false);
      setRefusal(refused ?? '');
    });
  };
  return (
    <div className="flex h-full w-full flex-col gap-3 overflow-y-auto px-list py-4">
      <Text typography="titleMedium">Connect to an Aircast device</Text>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        Type the address of a device on this network. Its status, cameras and settings open here.
      </Text>
      <OutlinedTextField
        label="Device address"
        value={address}
        placeholder="192.168.1.100 or falcon.local"
        keyboardType="uri"
        autoCorrect={false}
        isError={refusal !== ''}
        supportingText={refusal !== '' ? refusal : undefined}
        onValueChange={setAddress}
        onDone={connect}
      />
      <div>
        <FilledTonalButton enabled={!busy} onClick={connect}>
          Connect
        </FilledTonalButton>
      </div>
    </div>
  );
};

export const flyHereText = (origin: string): string =>
  `To fly from here, open ${origin}/fly/ — a browser lets this page read the device's settings and video, but not hold a telemetry link to a device without https.`;

const HostedDevice = ({ origin }: { origin: string }) => (
  <DeviceSection title="This app">
    <DeviceFact label="Connected to" value={new URL(origin).host} />
    {!telemetryReachable(location.href, origin) ? <DeviceNotice notice={{ level: DeviceLevel.Neutral, text: flyHereText(origin) }} /> : null}
    <div className="px-list py-2">
      <TextButton onClick={() => DeviceLink.disconnect()}>Disconnect</TextButton>
    </div>
  </DeviceSection>
);

const UplinkSection = ({ link }: { link: DeviceUplink }) => (
  <DeviceSection title="Link">
    <DeviceFact label="Carrying video" value={link.carrier} />
    <DeviceText text={[link.backup, link.network].filter((it) => it !== '').join(' · ')} />
    {link.rate !== '' ? <DeviceFact label="Throughput" value={link.rate} /> : null}
    {link.standby !== '' ? <DeviceFact label="Cellular · standby" value={link.standby} /> : null}
    {link.trouble !== '' ? <DeviceNotice notice={{ level: DeviceLevel.Warn, text: link.trouble }} /> : null}
    {link.data !== '' ? <DeviceFact label="Data" value={link.data} /> : null}
    {link.charts.map((chart) => (
      <DeviceChart key={chart.label} chart={chart} />
    ))}
  </DeviceSection>
);

const HealthSection = ({ health }: { health: DeviceHealth }) => (
  <DeviceSection title="Device health">
    {health.memory !== '' ? <DeviceFact label="Memory in use" value={health.memory} /> : null}
    {health.warning !== null ? <DeviceNotice notice={health.warning} /> : null}
    {health.charts.map((chart) => (
      <DeviceChart key={chart.label} chart={chart} />
    ))}
  </DeviceSection>
);

const NativeDevicePage = ({ entry, href, query, onOpen, onBack }: { entry: DeviceScreenEntry; href: string; query: URLSearchParams; onOpen: (href: string) => void; onBack: () => void }) => {
  useBackHandler(true, onBack);
  return (
    <>
      <OverridePageHeading title={entry.title} onBack={onBack} />
      <div className="h-full w-full overflow-y-auto">{entry.screen({ href, query, onOpen, onClose: onBack })}</div>
    </>
  );
};

export const DeviceDeepLink = () => {
  const navigation = useContext(LocalAppNavigation);
  const answered = useValue(deviceAnswered);
  const requested = useValue(deviceRequested);
  useEffect(() => {
    if (!answered || requested === null || navigation.settingsOpen.value) return;
    navigation.settingsPage.value = DEVICE_PAGE;
    navigation.settingsOpen.value = true;
  }, [answered, requested]);
  return null;
};

export const chipText = (status: DeviceStatus): string => (status.name !== '' ? `${status.name} · ${status.headline}` : status.headline);

export const DeviceStatusChip = () => {
  const navigation = useContext(LocalAppNavigation);
  const status = useValue(deviceStatus);
  const backdrop = useOsdBackdrop(withAlpha(Color.Black, 0.45));
  const narrow = useFlyIsPortrait();
  if (status === null || !deviceNeedsAttention(status)) return null;
  const open = () => {
    navigation.settingsPage.value = DEVICE_PAGE;
    navigation.settingsOpen.value = true;
  };
  return narrow ? (
    <Surface shape="full" color={backdrop} contentColor={aircast('outdoorForeground')} className="relative grid size-10 shrink-0 place-items-center" label={`${status.name}: ${status.headline}`} title={chipText(status)} onClick={open}>
      <MaterialIcon name="developer_board" size={22} />
      <span className="absolute top-1.5 right-1.5 size-2.5 rounded-full ring-2 ring-black/40" style={{ background: levelColor(status.level) }} />
    </Surface>
  ) : (
    <Surface shape="full" color={backdrop} contentColor={aircast('outdoorForeground')} className="min-w-0 shrink px-3 py-1.5" label={`${status.name}: ${status.headline}`} onClick={open}>
      <span className="flex min-w-0 items-center gap-2">
        <span className="size-2 shrink-0 rounded-full" style={{ background: levelColor(status.level) }} />
        <Text typography="labelLarge" maxLines={1} ellipsis>
          {chipText(status)}
        </Text>
      </span>
    </Surface>
  );
};
