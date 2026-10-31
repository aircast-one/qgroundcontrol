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
import { Drawable } from '@/res/Icon';
import { type DeviceCheck, type DeviceHealth, type DevicePage, type DeviceStatus, type DeviceUplink, deviceNeedsAttention, devicePageIcon, deviceRequested, useDeviceStatus } from '@/ui/DeviceStatus';
import { useDeviceReading } from '@/ui/DeviceView';
import { DeviceLevel } from '@/ui/DeviceLevel';
import { DEVICE_PAGE } from '@/ui/DevicePage';
import { DEVICE_SCREENS, type DeviceScreenEntry, devicePath } from '@/ui/DeviceScreens';
import { DeviceChart, DeviceFact, DeviceNotice, DeviceSection, DeviceText, levelColor } from '@/ui/DeviceKit';
import { DASHBOARD_MAX_WIDTH, DETAIL_PANE_MAX_WIDTH, useDesktopLayout } from '@/ui/DesktopLayout';

const checkState = (level: number): SetupState => (level === DeviceLevel.Ready ? SetupState.Done : SetupState.NeedsAttention);

const PageColumn = ({ wide = false, children }: { wide?: boolean; children: ReactNode }) => (
  <div className="h-full w-full" style={{ maxWidth: wide ? DASHBOARD_MAX_WIDTH : DETAIL_PANE_MAX_WIDTH }}>
    {children}
  </div>
);

export const DeviceTab = ({ shown = true, open, onOpen, onClose }: { shown?: boolean; open: string | null; onOpen: (href: string) => void; onClose: () => void }) => {
  const reading = useDeviceReading();
  const status = useDeviceStatus();
  const served = useValue(deviceServed);
  const hosted = served ? null : <HostedDevice host={reading.host} />;
  const requested = useValue(deviceRequested);
  const desktop = useDesktopLayout();
  const [stack, setStack] = useKeyedState<readonly string[]>([open], () => (open === null ? [] : [open]));
  useEffect(() => {
    if (requested === null || !shown) return;
    deviceRequested.value = null;
    onOpen(requested);
  }, [requested, shown]);
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
  if (desktop) return <DesktopHome status={status} hosted={hosted} onOpen={onOpen} />;
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

const TILE = 'rounded-shape-md bg-surface-container-low text-left';
const PRESSABLE = 'hover:bg-surface-container-high focus-visible:outline-2 focus-visible:outline-[var(--md-primary)]';

const flightLogCheck = (status: DeviceStatus): DeviceCheck => ({
  label: 'Flight log',
  value: status.events !== '' ? status.events : 'All runs',
  level: status.eventsAlert ? DeviceLevel.Warn : DeviceLevel.Neutral,
  href: '/flights',
  action: '',
});

const DesktopPanel = ({ title, children }: { title: string; children: ReactNode }) => (
  <section aria-label={title} className="flex flex-col gap-2">
    <Text typography="labelLarge" color={md('onSurfaceVariant')}>
      {title}
    </Text>
    {children}
  </section>
);

const CheckTile = ({ check, onOpen }: { check: DeviceCheck; onOpen: (href: string) => void }) => {
  const content = (
    <>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block">
        {check.label}
      </Text>
      <Text typography="titleMedium" color={check.level === DeviceLevel.Neutral ? md('onSurface') : levelColor(check.level)} className="block break-words">
        {check.value}
      </Text>
    </>
  );
  return check.href !== '' ? (
    <button type="button" onClick={() => onOpen(check.href)} className={`${TILE} ${PRESSABLE} min-w-[150px] flex-auto px-4 py-3`}>
      {content}
    </button>
  ) : (
    <div className={`${TILE} min-w-[150px] flex-auto px-4 py-3`}>
      {content}
    </div>
  );
};

const PageCard = ({ page, onOpen }: { page: DevicePage; onOpen: (href: string) => void }) => (
  <button type="button" onClick={() => onOpen(page.href)} className={`${TILE} ${PRESSABLE} flex min-w-0 items-center gap-3 p-3`}>
    <span className="grid size-10 shrink-0 place-items-center rounded-full bg-surface-container-highest">
      <Drawable name={devicePageIcon(page.icon)} size={22} />
    </span>
    <span className="flex min-w-0 flex-col">
      <Text typography="titleSmall">{page.title}</Text>
      {page.summary !== '' ? (
        <Text typography="bodySmall" color={page.needs ? levelColor(DeviceLevel.Warn) : md('onSurfaceVariant')} maxLines={1} ellipsis>
          {page.summary}
        </Text>
      ) : null}
    </span>
  </button>
);

const DesktopHome = ({ status, hosted, onOpen }: { status: DeviceStatus; hosted: ReactNode; onOpen: (href: string) => void }) => (
  <div className="h-full w-full overflow-y-auto">
    <div className="flex flex-col gap-6 px-list py-4" style={{ maxWidth: DASHBOARD_MAX_WIDTH }}>
      <div className={`${TILE} flex items-center gap-4 px-5 py-4`}>
        <span className="size-3 shrink-0 rounded-full" style={{ background: levelColor(status.level) }} />
        <div className="flex min-w-0 flex-1 flex-col gap-0.5">
          <Text typography="titleLarge" color={levelColor(status.level)}>
            {status.headline}
          </Text>
          {status.detail !== '' ? (
            <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
              {status.detail}
            </Text>
          ) : null}
        </div>
        {status.setup !== null ? <TextButton onClick={() => status.setup !== null && onOpen(status.setup.href)}>{status.setup.label}</TextButton> : null}
        {status.action !== null ? <FilledTonalButton onClick={() => status.action !== null && onOpen(status.action.href)}>{status.action.label}</FilledTonalButton> : null}
      </div>
      <div className="grid grid-cols-[minmax(0,2fr)_minmax(0,1fr)] items-start gap-6">
        <div className="flex min-w-0 flex-col gap-6">
          <DesktopPanel title="Before you fly">
            <div className="flex flex-wrap gap-3">
              {[...status.checks, flightLogCheck(status)].map((check) => (
                <CheckTile key={check.label} check={check} onOpen={onOpen} />
              ))}
            </div>
          </DesktopPanel>
          <DesktopPanel title={status.name !== '' ? status.name : 'Device'}>
            <div className="grid grid-cols-[repeat(auto-fill,minmax(220px,1fr))] gap-3">
              {status.pages.map((page) => (
                <PageCard key={page.href} page={page} onOpen={onOpen} />
              ))}
            </div>
          </DesktopPanel>
          {hosted}
        </div>
        <div className="flex min-w-0 flex-col gap-4">
          {status.link !== null ? (
            <div className={`${TILE} pb-2`}>
              <UplinkSection link={status.link} />
            </div>
          ) : null}
          {status.health !== null ? (
            <div className={`${TILE} pb-2`}>
              <HealthSection health={status.health} />
            </div>
          ) : null}
        </div>
      </div>
    </div>
  </div>
);

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
