import { invoke } from '@tauri-apps/api/core';
import { type ReactNode, useState } from 'react';
import { type Json, optBoolean, optDouble, optInt, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { openUri } from '@/kotlin/Platform';
import { FilledTonalButton, LinearProgressIndicator, Surface, Switch, Text, TextButton } from '@/m3';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { SectionHeader, SettingsCard } from './SetupRow';

export const APP_UPDATE = 'view.appUpdate';
export const DISCONNECT_FIRST = 'Disconnect from the vehicle first';
export const INSTALL_NOTE = 'Aircast closes, installs the update and opens again.';
const BANNER_MAX_WIDTH = 560;
const BANNER_CORNER = 12;
const MEGABYTE = 1_000_000;

export type AppUpdatePhase = 'idle' | 'checking' | 'upToDate' | 'available' | 'downloading' | 'ready' | 'failed';

export type AppUpdateReading = {
  current: string;
  channel: string;
  phase: AppUpdatePhase;
  latest: string;
  releaseUrl: string;
  sizeBytes: number;
  progress: number;
  error: string;
  autoCheck: boolean;
  skipped: boolean;
  later: boolean;
  updatedFrom: string;
  canInstall: boolean;
};

const PHASES: readonly AppUpdatePhase[] = ['idle', 'checking', 'upToDate', 'available', 'downloading', 'ready', 'failed'];

const phaseOf = (token: string): AppUpdatePhase => PHASES.find((phase) => phase === token) ?? 'idle';

export const appUpdateReading = (view: Json | null): AppUpdateReading | null =>
  view === null || optText(view, 'current') === ''
    ? null
    : {
        current: optText(view, 'current'),
        channel: optText(view, 'channel'),
        phase: phaseOf(optText(view, 'phase')),
        latest: optText(view, 'latest'),
        releaseUrl: optText(view, 'releaseUrl'),
        sizeBytes: optInt(view, 'sizeBytes'),
        progress: optDouble(view, 'progress', 0),
        error: optText(view, 'error'),
        autoCheck: optBoolean(view, 'autoCheck', true),
        skipped: optBoolean(view, 'skipped'),
        later: optBoolean(view, 'later'),
        updatedFrom: optText(view, 'updatedFrom'),
        canInstall: optBoolean(view, 'canInstall'),
      };

export const sizeText = (bytes: number): string | null => (bytes > 0 ? `${Math.max(1, Math.round(bytes / MEGABYTE))} MB` : null);

export const sized = (label: string, bytes: number): string => {
  const size = sizeText(bytes);
  return size === null ? label : `${label} (${size})`;
};

export const updateStatus = (reading: AppUpdateReading): string => {
  switch (reading.phase) {
    case 'checking':
      return 'Checking for updates…';
    case 'upToDate':
      return 'Aircast is up to date';
    case 'available':
      return `Aircast ${reading.latest} is available`;
    case 'downloading':
      return `Downloading Aircast ${reading.latest}… ${Math.round(reading.progress * 100)}%`;
    case 'ready':
      return `Aircast ${reading.latest} is ready to install`;
    case 'failed':
      return reading.error.trim() === '' ? 'The update check failed.' : reading.error;
    case 'idle':
      return '';
  }
};

export type UpdateBannerKind = 'updated' | 'offer' | 'downloading' | 'ready';

export const updateBannerKind = (reading: AppUpdateReading | null): UpdateBannerKind | null => {
  if (reading === null) return null;
  if (reading.updatedFrom.trim() !== '') return 'updated';
  if (reading.later) return null;
  if (reading.phase === 'downloading') return 'downloading';
  if (reading.phase === 'ready') return 'ready';
  return reading.phase === 'available' && !reading.skipped ? 'offer' : null;
};

const act = (path: string, ...args: unknown[]) => void Qgc.invoke(path, ...args);

const useInstaller = (): { refusal: string | null; install: () => void } => {
  const [refusal, setRefusal] = useState<string | null>(null);
  const install = () => {
    setRefusal(null);
    invoke('install_update').catch((error: unknown) => setRefusal(String(error)));
  };
  return { refusal, install };
};

const releasePage = (reading: AppUpdateReading): string => (reading.releaseUrl.trim() === '' ? `https://github.com/aircast-one/qgroundcontrol/releases` : reading.releaseUrl);

const UpdateLine = ({ headline, supporting, children }: { headline: string; supporting: string; children?: ReactNode }) => (
  <div className="flex min-h-14 flex-wrap items-center gap-2 px-list py-2">
    <span className="flex min-w-0 flex-1 flex-col">
      <Text typography="bodyLarge">{headline}</Text>
      {supporting.trim() !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          {supporting}
        </Text>
      ) : null}
    </span>
    {children}
  </div>
);

const UpdateActions = ({ reading, refusal, install }: { reading: AppUpdateReading; refusal: string | null; install: () => void }) => (
  <>
    {reading.phase === 'available' ? (
      <>
        <TextButton onClick={() => openUri(releasePage(reading))}>Release notes</TextButton>
        <TextButton onClick={() => act('appUpdate.skip')}>Skip this version</TextButton>
        <FilledTonalButton onClick={() => act('appUpdate.download')}>{sized('Download', reading.sizeBytes)}</FilledTonalButton>
      </>
    ) : null}
    {reading.phase === 'downloading' ? <TextButton onClick={() => act('appUpdate.cancel')}>Cancel</TextButton> : null}
    {reading.phase === 'ready' ? (
      <FilledTonalButton enabled={reading.canInstall} onClick={install} title={reading.canInstall ? INSTALL_NOTE : DISCONNECT_FIRST}>
        {refusal === null ? 'Install and restart' : 'Try again'}
      </FilledTonalButton>
    ) : null}
    {reading.phase === 'failed' ? <TextButton onClick={() => act('appUpdate.check')}>Try again</TextButton> : null}
  </>
);

const installNote = (reading: AppUpdateReading, refusal: string | null): string => refusal ?? (reading.canInstall ? INSTALL_NOTE : DISCONNECT_FIRST);

export const UpdatesCard = () => {
  const reading = appUpdateReading(useQgcPath(APP_UPDATE));
  const { refusal, install } = useInstaller();
  if (!__DESKTOP__ || reading === null) return null;
  const status = updateStatus(reading);
  return (
    <>
      <SectionHeader text="Updates" />
      <SettingsCard>
        <UpdateLine headline="Aircast version" supporting={`${reading.current} · ${reading.channel} channel`}>
          <TextButton enabled={reading.phase !== 'checking' && reading.phase !== 'downloading'} onClick={() => act('appUpdate.check')}>
            Check now
          </TextButton>
        </UpdateLine>
        <UpdateLine headline="Check for updates automatically" supporting="">
          <Switch checked={reading.autoCheck} onCheckedChange={(on) => act('appUpdate.setAutoCheck', on)} label="Check for updates automatically" />
        </UpdateLine>
        {status === '' ? null : (
          <UpdateLine headline={status} supporting={reading.phase === 'ready' ? installNote(reading, refusal) : ''}>
            <UpdateActions reading={reading} refusal={refusal} install={install} />
          </UpdateLine>
        )}
        {reading.phase === 'downloading' ? <LinearProgressIndicator className="w-full" progress={reading.progress} label={status} /> : null}
      </SettingsCard>
    </>
  );
};

export const UpdateBanner = () => {
  const reading = appUpdateReading(useQgcPath(APP_UPDATE));
  const { refusal, install } = useInstaller();
  const kind = __DESKTOP__ ? updateBannerKind(reading) : null;
  if (reading === null || kind === null) return null;
  const text = kind === 'updated' ? `Updated to Aircast ${reading.current}` : updateStatus(reading);
  const detail = kind === 'ready' ? installNote(reading, refusal) : '';
  return (
    <Surface className="w-full" style={{ maxWidth: BANNER_MAX_WIDTH }} shape={BANNER_CORNER} color={md('secondaryContainer')} contentColor={md('onSecondaryContainer')}>
      <div className="flex min-h-12 flex-wrap items-center gap-2 px-3 py-2">
        <Drawable name={kind === 'updated' ? 'ic_check_circle' : 'ic_download'} size={24} className="shrink-0" />
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="titleSmall">{text}</Text>
          {detail.trim() !== '' ? <Text typography="bodyMedium">{detail}</Text> : null}
        </div>
        {kind === 'updated' ? (
          <>
            <TextButton onClick={() => act('appUpdate.dismissUpdated')}>Dismiss</TextButton>
            <TextButton onClick={() => openUri(`https://github.com/aircast-one/qgroundcontrol/releases/tag/aircast-${reading.current.startsWith('v') ? reading.current : `v${reading.current}`}`)}>What&apos;s new</TextButton>
          </>
        ) : null}
        {kind === 'offer' ? (
          <>
            <TextButton onClick={() => act('appUpdate.later')}>Later</TextButton>
            <FilledTonalButton onClick={() => act('appUpdate.download')}>{sized('Update', reading.sizeBytes)}</FilledTonalButton>
          </>
        ) : null}
        {kind === 'downloading' ? <TextButton onClick={() => act('appUpdate.cancel')}>Cancel</TextButton> : null}
        {kind === 'ready' ? (
          <>
            <TextButton onClick={() => act('appUpdate.later')}>Later</TextButton>
            <FilledTonalButton enabled={reading.canInstall} onClick={install}>
              {refusal === null ? 'Install and restart' : 'Try again'}
            </FilledTonalButton>
          </>
        ) : null}
      </div>
    </Surface>
  );
};
