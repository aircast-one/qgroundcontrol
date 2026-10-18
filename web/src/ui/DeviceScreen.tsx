import { useContext, useEffect, useRef, useState } from 'react';
import { useBackHandler, useKeyedState } from '@/kotlin/Compose';
import { useValue } from '@/kotlin/State';
import { FilledTonalButton, Surface, Text } from '@/m3';
import { Color, aircast, md, withAlpha } from '@/theme/Colors';
import { LocalAppNavigation } from '@/ui/AppNavigation';
import { useOsdBackdrop } from '@/ui/Osd';
import { FootNote, OverridePageHeading, SectionHeader, SetupRow, SetupState } from '@/ui/SetupRow';
import { DEVICE_HOME, DEVICE_PAGE, DeviceLevel, type DeviceStatus, deviceAnswered, deviceNeedsAttention, devicePageIcon, deviceRequested, deviceStatus } from '@/ui/DeviceStatus';
import { DEVICE_SCREENS, type DeviceScreenEntry, devicePath } from '@/ui/DeviceScreens';

const TITLE_PREFIX = 'aircast · ';

export const framedTitle = (documentTitle: string | undefined, fallback: string): string => {
  const title = (documentTitle ?? '').replace(TITLE_PREFIX, '').trim();
  return title === '' ? fallback : title;
};

export const openedTitle = (status: DeviceStatus | null, href: string): string =>
  status?.pages.find((page) => page.href === href)?.title ?? status?.checks.find((check) => check.href === href)?.label ?? status?.action?.label ?? 'Device';

const levelColor = (level: number): string => (level === DeviceLevel.Ready ? aircast('success') : level === DeviceLevel.Warn ? aircast('warning') : aircast('alert'));

const checkState = (level: number): SetupState => (level === DeviceLevel.Ready ? SetupState.Done : SetupState.NeedsAttention);

export const DeviceTab = ({ open, onOpen, onClose }: { open: string | null; onOpen: (href: string) => void; onClose: () => void }) => {
  const status = useValue(deviceStatus);
  const requested = useValue(deviceRequested);
  const [stack, setStack] = useKeyedState<readonly string[]>([open], () => (open === null ? [] : [open]));
  useEffect(() => {
    if (requested === null) return;
    deviceRequested.value = null;
    onOpen(requested);
  }, [requested]);
  const current = stack.at(-1) ?? null;
  const back = () => (stack.length > 1 ? setStack(stack.slice(0, -1)) : onClose());
  if (current !== null) {
    const { path, query } = devicePath(current);
    const entry = DEVICE_SCREENS.get(path);
    return entry !== undefined ? (
      <NativeDevicePage key={current} entry={entry} href={current} query={query} onOpen={(href) => setStack([...stack, href])} onBack={back} />
    ) : (
      <DevicePageFrame key={current} href={current} title={openedTitle(status, current)} onBack={back} />
    );
  }
  if (status === null) return <FootNote text="Waiting for the device to answer…" />;
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
    </div>
  );
};

const NativeDevicePage = ({ entry, href, query, onOpen, onBack }: { entry: DeviceScreenEntry; href: string; query: URLSearchParams; onOpen: (href: string) => void; onBack: () => void }) => {
  useBackHandler(true, onBack);
  return (
    <>
      <OverridePageHeading title={entry.title} onBack={onBack} />
      <div className="h-full w-full overflow-y-auto">{entry.screen({ href, query, onOpen, onClose: onBack })}</div>
    </>
  );
};

export const framedElsewhere = (href: string, at: Location | undefined): boolean => {
  if (at === undefined) return false;
  const opened = new URL(href, at.href);
  return at.pathname + at.search !== opened.pathname + opened.search;
};

const DevicePageFrame = ({ href, title, onBack }: { href: string; title: string; onBack: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const [shown, setShown] = useState(title);
  const frame = useRef<HTMLIFrameElement>(null);
  const back = () => {
    const inner = frame.current?.contentWindow ?? null;
    if (inner !== null && framedElsewhere(href, inner.location)) inner.history.back();
    else onBack();
  };
  useBackHandler(true, back);
  useEffect(() => {
    const home = (event: MessageEvent) => {
      if (event.origin === location.origin && event.data === DEVICE_HOME) navigation.settingsOpen.value = false;
    };
    window.addEventListener('message', home);
    return () => window.removeEventListener('message', home);
  }, []);
  return (
    <>
      <OverridePageHeading title={shown} onBack={back} />
      <iframe
        ref={frame}
        src={href}
        title={title}
        allow="clipboard-write; fullscreen"
        className="h-full w-full border-0 bg-transparent"
        onLoad={(event) => setShown(framedTitle(event.currentTarget.contentDocument?.title, title))}
      />
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
  if (status === null || !deviceNeedsAttention(status)) return null;
  return (
    <Surface
      shape="full"
      color={backdrop}
      contentColor={aircast('outdoorForeground')}
      className="min-w-0 shrink px-3 py-1.5"
      label={`${status.name}: ${status.headline}`}
      onClick={() => {
        navigation.settingsPage.value = DEVICE_PAGE;
        navigation.settingsOpen.value = true;
      }}
    >
      <span className="flex min-w-0 items-center gap-2">
        <span className="size-2 shrink-0 rounded-full" style={{ background: levelColor(status.level) }} />
        <Text typography="labelLarge" maxLines={1} ellipsis>
          {chipText(status)}
        </Text>
      </span>
    </Surface>
  );
};
