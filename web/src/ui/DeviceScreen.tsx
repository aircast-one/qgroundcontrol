import { type ReactNode, useContext, useEffect, useState } from 'react';
import { useBackHandler, useKeyedState } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { FilledTonalButton, OutlinedTextField, Surface, Text, TextButton } from '@/m3';
import { DeviceLink, deviceServed } from '@/app/DeviceLink';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { useOsdBackdrop } from '@/ui/Osd';
import { useFlyIsPortrait } from '@/ui/FlyShell';
import { MaterialIcon } from '@/res/Icon';
import { FootNote, OverridePageHeading, SectionHeader, SetupRow, SetupState } from '@/ui/SetupRow';
import { type DeviceHealth, type DeviceStatus, type DeviceUplink, deviceNeedsAttention, devicePageIcon, deviceRequested, useDeviceStatus } from '@/ui/DeviceStatus';
import { useDeviceReading } from '@/ui/DeviceView';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { DEVICE_PAGE } from '@/ui/DevicePage';
import { DEVICE_SCREENS, type DeviceScreenEntry, devicePath } from '@/ui/DeviceScreens';
import { DeviceChart, DeviceFact, DeviceNotice, DeviceSection, DeviceText, levelColor } from '@/ui/DeviceKit';
import { DASHBOARD_MAX_WIDTH, DETAIL_PANE_MAX_WIDTH } from '@/ui/DesktopLayout';

const checkState = (level: number): SetupState => (level === DeviceLevel.Ready ? SetupState.Done : SetupState.NeedsAttention);

const PageColumn = ({ wide = false, children }: { wide?: boolean; children: ReactNode }) => (
  <div className="h-full w-full" style={{ maxWidth: wide ? DASHBOARD_MAX_WIDTH : DETAIL_PANE_MAX_WIDTH }}>
    {children}
  </div>
);

export const DeviceTab = ({ open, onOpen, onClose }: { open: string | null; onOpen: (href: string) => void; onClose: () => void }) => {
  const reading = useDeviceReading();
  const status = useDeviceStatus();
  const served = useValue(deviceServed);
  const hosted = served ? null : <HostedDevice host={reading.host} />;
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
    return (
      <PageColumn wide={entry.narrow !== true}>
        <NativeDevicePage
          key={current}
          entry={entry}
          href={current}
          query={opened.query}
          onOpen={(href) => setStack([...stack, href])}
          onOpenFrom={(href, returnTo) => setStack([...stack.slice(0, -1), returnTo, href])}
          onBack={back}
        />
      </PageColumn>
    );
  if (!served && reading.origin === '')
    return (
      <PageColumn>
        <ConnectDevice />
      </PageColumn>
    );
  if (status === null)
    return (
      <PageColumn>
        <div className="h-full w-full overflow-y-auto">
          <FootNote text="Waiting for the device to answer…" />
          {hosted}
        </div>
      </PageColumn>
    );
  return (
    <div className="@container h-full w-full overflow-y-auto">
      <div style={{ maxWidth: DASHBOARD_MAX_WIDTH }}>
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
        <div className="grid items-start @min-[880px]:grid-cols-2 @min-[880px]:grid-rows-[auto_1fr] @min-[880px]:gap-x-6">
          <section aria-label="Before you fly">
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
            <SetupRow title={status.events !== '' ? status.events : 'Flight log'} status="" state={status.eventsAlert ? SetupState.NeedsAttention : SetupState.Neutral} onClick={() => onOpen('/flights')} />
          </section>
          <div className="@min-[880px]:col-start-2 @min-[880px]:row-span-2 @min-[880px]:row-start-1">
            {status.link !== null ? <UplinkSection link={status.link} /> : null}
            {status.health !== null ? <HealthSection health={status.health} /> : null}
          </div>
          <div>
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
            {hosted}
          </div>
        </div>
      </div>
    </div>
  );
};

const ConnectDevice = () => {
  const [address, setAddress] = useState('');
  const [busy, setBusy] = useState(false);
  const [refusal, setRefusal] = useState('');
  const connect = () => {
    setBusy(true);
    void DeviceLink.connect(address).then((refused) => {
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

const HostedDevice = ({ host }: { host: string }) => (
  <DeviceSection title="This app">
    <DeviceFact label="Connected to" value={host} />
    <div className="px-list py-2">
      <TextButton onClick={() => void DeviceLink.disconnect()}>Disconnect</TextButton>
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

const NativeDevicePage = ({
  entry,
  href,
  query,
  onOpen,
  onOpenFrom,
  onBack,
}: {
  entry: DeviceScreenEntry;
  href: string;
  query: URLSearchParams;
  onOpen: (href: string) => void;
  onOpenFrom: (href: string, returnTo: string) => void;
  onBack: () => void;
}) => {
  useBackHandler(true, onBack);
  return (
    <>
      <OverridePageHeading title={entry.title} onBack={onBack} />
      <div className="h-full w-full overflow-y-auto">{entry.screen({ href, query, onOpen, onOpenFrom, onClose: onBack })}</div>
    </>
  );
};

export const DeviceDeepLink = () => {
  const navigation = useContext(LocalAppNavigation);
  const { answered } = useDeviceReading();
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
  const status = useDeviceStatus();
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
