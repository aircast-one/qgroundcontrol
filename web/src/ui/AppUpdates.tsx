import { getVersion } from '@tauri-apps/api/app';
import { relaunch } from '@tauri-apps/plugin-process';
import { check, type DownloadEvent, type Update } from '@tauri-apps/plugin-updater';
import { type ReactNode, useSyncExternalStore } from 'react';
import { type Json, isJson, optJSONArray, optText } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { commandFailed } from '@/core/desktopCommands';
import { getSharedPreferences, openUri } from '@/kotlin/Platform';
import { FilledTonalButton, LinearProgressIndicator, Surface, Switch, Text, TextButton } from '@/m3';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { useHasVehicle } from './FlyState';
import { SectionHeader, SettingsCard } from './SetupRow';

export const DISCONNECT_FIRST = 'Disconnect from the vehicle first';
export const INSTALL_NOTE = 'Aircast closes, installs the update and opens again.';
const CLOSE_CHECKS = 'view.closeChecks';
const BLOCKERS: Readonly<Record<string, string>> = {
  activeConnections: DISCONNECT_FIRST,
  unsavedMission: 'Save or send the mission first',
  pendingWrites: 'Wait for the parameter writes to finish first',
};
const RELEASES = 'https://github.com/aircast-one/qgroundcontrol/releases';
const FIRST_CHECK_AFTER_MS = 15_000;
const CHECK_EVERY_MS = 6 * 60 * 60 * 1000;
const BANNER_MAX_WIDTH = 560;
const BANNER_CORNER = 12;

export type AppUpdatePhase = 'idle' | 'checking' | 'upToDate' | 'available' | 'downloading' | 'installing' | 'failed';

export type AppUpdateState = {
  current: string;
  phase: AppUpdatePhase;
  latest: string;
  downloaded: number;
  total: number;
  error: string;
  autoCheck: boolean;
  skipped: string;
  later: boolean;
  updatedFrom: string;
};

export type UpdateBannerKind = 'updated' | 'offer' | 'installing' | 'failed';

const preferences = getSharedPreferences('appUpdate');

const store: { state: AppUpdateState; update: Update | null; started: boolean } = {
  state: {
    current: '',
    phase: 'idle',
    latest: '',
    downloaded: 0,
    total: 0,
    error: '',
    autoCheck: preferences.getBoolean('autoCheck', true),
    skipped: preferences.getString('skipped', null) ?? '',
    later: false,
    updatedFrom: '',
  },
  update: null,
  started: false,
};

const changes = new EventTarget();

export const channelOf = (version: string): string => (version.includes('-') ? 'beta' : 'stable');

export const percentOf = (state: AppUpdateState): number => (state.total > 0 ? Math.min(100, Math.floor((state.downloaded / state.total) * 100)) : 0);

export const updateStatus = (state: AppUpdateState): string => {
  switch (state.phase) {
    case 'checking':
      return 'Checking for updates…';
    case 'upToDate':
      return 'Aircast is up to date';
    case 'available':
      return `Aircast ${state.latest} is available`;
    case 'downloading':
      return `Downloading Aircast ${state.latest}… ${percentOf(state)}%`;
    case 'installing':
      return `Installing Aircast ${state.latest}…`;
    case 'failed':
      return state.error.trim() === '' ? 'The update failed.' : state.error;
    case 'idle':
      return '';
  }
};

export const updateBannerKind = (state: AppUpdateState): UpdateBannerKind | null => {
  if (state.updatedFrom !== '') return 'updated';
  if (state.later) return null;
  if (state.phase === 'downloading' || state.phase === 'installing') return 'installing';
  if (state.phase === 'failed' && state.latest !== '') return 'failed';
  return state.phase === 'available' && state.latest !== state.skipped ? 'offer' : null;
};

export const installBlocker = (checks: Json | null): string | null => {
  const prompt = (optJSONArray(checks, 'prompts') ?? []).find(isJson);
  return prompt === undefined ? null : (BLOCKERS[optText(prompt, 'id')] ?? optText(prompt, 'message'));
};

const publish = (next: Partial<AppUpdateState>) => {
  store.state = { ...store.state, ...next };
  changes.dispatchEvent(new Event('change'));
};

const busy = () => ['checking', 'downloading', 'installing'].includes(store.state.phase);

const failed = (error: unknown) => {
  commandFailed('updater')(error);
  publish({ phase: 'failed', error: String(error) });
};

export const checkForUpdate = async (): Promise<void> => {
  if (busy()) return;
  publish({ phase: 'checking', error: '' });
  try {
    const found = await check();
    store.update = found;
    publish(found === null ? { phase: 'upToDate', latest: '' } : { phase: 'available', latest: found.version });
  } catch (error) {
    failed(error);
  }
};

const progressed = (event: DownloadEvent) => {
  if (event.event === 'Started') return publish({ total: event.data.contentLength ?? 0 });
  if (event.event === 'Finished') return publish({ phase: 'installing' });
  const before = percentOf(store.state);
  store.state = { ...store.state, downloaded: store.state.downloaded + event.data.chunkLength };
  if (percentOf(store.state) !== before) changes.dispatchEvent(new Event('change'));
};

export const installUpdate = async (): Promise<void> => {
  const update = store.update;
  if (update === null || busy()) return;
  publish({ phase: 'downloading', downloaded: 0, total: 0, error: '', later: false });
  try {
    await update.downloadAndInstall(progressed);
    await relaunch();
  } catch (error) {
    failed(error);
  }
};

const retry = () => (store.update === null ? checkForUpdate() : installUpdate());

const later = () => publish({ later: true });

const skip = () => {
  preferences.putString('skipped', store.state.latest);
  publish({ skipped: store.state.latest });
};

const setAutoCheck = (on: boolean) => {
  preferences.putBoolean('autoCheck', on);
  publish({ autoCheck: on });
};

const dismissUpdated = () => publish({ updatedFrom: '' });

const automatically = () => void (store.state.autoCheck ? checkForUpdate() : undefined);

const start = async () => {
  const current = await getVersion();
  const last = preferences.getString('lastVersion', null);
  preferences.putString('lastVersion', current);
  publish({ current, updatedFrom: last !== null && last !== current ? last : '' });
  setTimeout(automatically, FIRST_CHECK_AFTER_MS);
  setInterval(automatically, CHECK_EVERY_MS);
};

const subscribe = (changed: () => void) => {
  if (__DESKTOP__ && !store.started) {
    store.started = true;
    start().catch(commandFailed('updater'));
  }
  changes.addEventListener('change', changed);
  return () => changes.removeEventListener('change', changed);
};

const snapshot = () => store.state;

const useAppUpdate = (): AppUpdateState => useSyncExternalStore(subscribe, snapshot);

const releasePage = (version: string): string => `${RELEASES}/tag/aircast-v${version.replace(/^v/, '')}`;

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

const InstallButton = ({ label, blocker }: { label: string; blocker: string | null }) => (
  <FilledTonalButton enabled={blocker === null} onClick={() => void installUpdate()} title={blocker ?? INSTALL_NOTE}>
    {label}
  </FilledTonalButton>
);

export const UpdatesCard = () => {
  const state = useAppUpdate();
  const blocker = installBlocker(useQgcPath(CLOSE_CHECKS));
  if (!__DESKTOP__ || state.current === '') return null;
  const status = updateStatus(state);
  return (
    <>
      <SectionHeader text="Updates" />
      <SettingsCard>
        <UpdateLine headline="Aircast version" supporting={`${state.current} · ${channelOf(state.current)} channel`}>
          <TextButton enabled={!busy()} onClick={() => void checkForUpdate()}>
            Check now
          </TextButton>
        </UpdateLine>
        <UpdateLine headline="Check for updates automatically" supporting="">
          <Switch checked={state.autoCheck} onCheckedChange={setAutoCheck} label="Check for updates automatically" />
        </UpdateLine>
        {status === '' ? null : (
          <UpdateLine headline={status} supporting={state.phase === 'available' ? (blocker ?? INSTALL_NOTE) : ''}>
            {state.phase === 'available' ? (
              <>
                <TextButton onClick={() => openUri(releasePage(state.latest))}>Release notes</TextButton>
                {state.latest !== state.skipped ? <TextButton onClick={skip}>Skip this version</TextButton> : null}
                <InstallButton label="Update" blocker={blocker} />
              </>
            ) : null}
            {state.phase === 'failed' ? <TextButton onClick={() => void retry()}>Try again</TextButton> : null}
          </UpdateLine>
        )}
        {state.phase === 'downloading' ? <LinearProgressIndicator className="w-full" progress={percentOf(state) / 100} label={status} /> : null}
      </SettingsCard>
    </>
  );
};

export const UpdateBanner = () => {
  const state = useAppUpdate();
  const blocker = installBlocker(useQgcPath(CLOSE_CHECKS));
  const connected = useHasVehicle();
  const kind = __DESKTOP__ ? updateBannerKind(state) : null;
  if (kind === null || (connected && kind === 'offer')) return null;
  const text = kind === 'updated' ? `Updated to Aircast ${state.current}` : updateStatus(state);
  return (
    <Surface className="w-full" style={{ maxWidth: BANNER_MAX_WIDTH }} shape={BANNER_CORNER} color={md('secondaryContainer')} contentColor={md('onSecondaryContainer')}>
      <div className="flex min-h-12 flex-wrap items-center gap-2 px-3 py-2">
        <Drawable name={kind === 'updated' ? 'ic_check_circle' : 'ic_download'} size={24} className="shrink-0" />
        <Text typography="titleSmall" className="min-w-0 flex-1">
          {text}
        </Text>
        {kind === 'updated' ? (
          <>
            <TextButton onClick={dismissUpdated}>Dismiss</TextButton>
            <TextButton onClick={() => openUri(releasePage(state.current))}>What&apos;s new</TextButton>
          </>
        ) : null}
        {kind === 'offer' || kind === 'failed' ? (
          <>
            <TextButton onClick={later}>Later</TextButton>
            <InstallButton label={kind === 'offer' ? 'Update' : 'Try again'} blocker={blocker} />
          </>
        ) : null}
      </div>
      {kind === 'installing' && state.phase === 'downloading' ? <LinearProgressIndicator className="w-full" progress={percentOf(state) / 100} label={text} /> : null}
    </Surface>
  );
};
