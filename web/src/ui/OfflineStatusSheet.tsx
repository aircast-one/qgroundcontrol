import { useContext, useEffect, useMemo, useState } from 'react';
import { LinkCommands } from '@/bridge/BridgeCommands';
import { type Json, has, jsonObjects, optBoolean, optInt, optJSONArray, optString } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useTicker } from '@/kotlin/Compose';
import { openUri } from '@/kotlin/Platform';
import { Button, CircularProgressIndicator, HorizontalDivider, ListItem, Surface, Text, TextButton } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { type AppNavigationState, LocalAppNavigation } from './AppNavigation';
import { deviceRequested, useDeviceStatus } from './DeviceStatus';
import { LOOKING_TITLE, useRememberLookingHint } from './ConnectingCard';
import { FLASHER_PAGE } from './FlasherScreen';
import { openCameraSetup, useCameraMissing } from './NoVideoPanel';

export const OFFLINE_STATUS_VIEW = 'view.offlineStatus';
const OFFLINE_LINKS_PATH = 'links.linkConfigurations';
const CONNECTIONS_SETTINGS_PAGE = 'Connections';
const OFFLINE_POLL_MS = 1000;

export type OfflineLink = { index: number; text: string; description: string; action: string; actionLabel: string; failed: boolean; connected: boolean };

export type OfflineStatus = { title: string; footnote: string; busy: boolean; noLinks: boolean; editAddress: boolean; onDevice: boolean; signInUrl: string; links: readonly OfflineLink[] };

export const offlineStatus = (view: Json | null): OfflineStatus | null =>
  view !== null && has(view, 'title')
    ? {
        title: optString(view, 'title'),
        footnote: optString(view, 'footnote'),
        busy: optBoolean(view, 'busy'),
        noLinks: optBoolean(view, 'noLinks'),
        editAddress: optBoolean(view, 'editAddress'),
        onDevice: optBoolean(view, 'onDevice'),
        signInUrl: optString(view, 'signInUrl'),
        links: jsonObjects(optJSONArray(view, 'links')).map((link) => ({
          index: optInt(link, 'index'),
          text: optString(link, 'text'),
          description: optString(link, 'description'),
          action: optString(link, 'action'),
          actionLabel: optString(link, 'actionLabel'),
          failed: optBoolean(link, 'failed'),
          connected: optBoolean(link, 'connected'),
        })),
      }
    : null;

export const OFFLINE_SHEET = 'offline';

export const CHECK_DEVICE = 'Check device';

export const DEVICE_TELEMETRY = '/telemetry';

export const openDeviceTelemetry = (): void => {
  deviceRequested.value = DEVICE_TELEMETRY;
};

export const openAddLink = (navigation: AppNavigationState): void => {
  navigation.addLinkRequested.value = true;
  navigation.settingsPage.value = CONNECTIONS_SETTINGS_PAGE;
};

export type OfflineGuide = { title: string; text: string; onDevice: boolean; camera: boolean; links: readonly OfflineLink[]; busy: boolean };

export const NO_CAMERA_GUIDE = { title: 'No camera yet', text: 'Add a camera and its picture shows here.' };

export const offlineGuide = (status: OfflineStatus | null, hint: string, noCamera = false): OfflineGuide => {
  const onDevice = status?.onDevice === true;
  const links = onDevice || noCamera ? [] : (status?.links ?? []);
  return noCamera
    ? { ...NO_CAMERA_GUIDE, onDevice, camera: true, links, busy: false }
    : {
        title: status === null || !onDevice ? LOOKING_TITLE : sentenceCase(status.title),
        text: status !== null && (onDevice || links.some((link) => link.connected || link.failed)) ? status.footnote : hint,
        onDevice,
        camera: false,
        links,
        busy: status?.busy === true,
      };
};

const openedSignIns = new Set<string>();

const useOpenSignIn = (url: string): void =>
  useEffect(() => {
    if (url === '' || openedSignIns.has(url)) return;
    openedSignIns.add(url);
    openUri(url);
  }, [url]);

export const linkAction = (link: OfflineLink, edit: () => void): Promise<unknown> =>
  link.connected
    ? LinkCommands.disconnect(link.index)
    : link.action === 'signIn'
      ? LinkCommands.signIn(`@${OFFLINE_LINKS_PATH}.${link.index}`)
      : link.action === 'edit'
        ? Promise.resolve(edit())
        : LinkCommands.connect(`@${OFFLINE_LINKS_PATH}.${link.index}`);

const OfflineLinkRow = ({
  link,
  busy,
  onEdit,
  onActed,
  className,
  containerColor,
}: {
  link: OfflineLink;
  busy: boolean;
  onEdit: () => void;
  onActed?: () => Promise<void>;
  className?: string;
  containerColor?: string;
}) => {
  const [acting, setActing] = useState(false);
  const act = () => {
    setActing(true);
    void linkAction(link, onEdit)
      .then(() => onActed?.())
      .finally(() => setActing(false));
  };
  return (
    <ListItem
      className={className}
      containerColor={containerColor}
      headlineContent={link.text}
      supportingContent={
        link.description === '' ? undefined : (
          <span className="whitespace-pre-line" style={{ color: link.failed ? md('error') : md('onSurfaceVariant') }}>
            {link.description}
          </span>
        )
      }
      trailingContent={
        acting || (link.connected && busy) ? (
          <CircularProgressIndicator size={16} strokeWidth={2} />
        ) : link.connected ? null : (
          <span className="inline-flex h-10 items-center rounded-full bg-secondary-container px-6 text-label-large text-on-secondary-container">{link.actionLabel}</span>
        )
      }
      onClick={act}
    />
  );
};

const CONNECTION_SETTINGS = 'Connection settings';

export const OfflineGuideCard = ({ inVideo }: { inVideo: boolean }) => {
  const navigation = useContext(LocalAppNavigation);
  const json = useQgcPath(OFFLINE_STATUS_VIEW);
  const status = useMemo(() => offlineStatus(json), [json]);
  const hint = useRememberLookingHint(status !== null && !status.noLinks);
  const noCamera = useCameraMissing() && inVideo;
  const onDevice = useDeviceStatus() !== null;
  const guide = useMemo(() => offlineGuide(status, hint, noCamera), [status, hint, noCamera]);
  useOpenSignIn(status?.signInUrl ?? '');
  const settings = () => {
    navigation.settingsPage.value = CONNECTIONS_SETTINGS_PAGE;
  };
  const icon = guide.camera ? 'ic_videocam' : guide.onDevice ? 'ic_developer_board' : 'ic_link';
  const label = guide.camera ? 'Add a camera' : guide.onDevice ? CHECK_DEVICE : guide.links.length > 0 ? CONNECTION_SETTINGS : 'Add a link';
  const open = () => (guide.camera ? openCameraSetup(onDevice, navigation) : guide.onDevice ? openDeviceTelemetry() : guide.links.length > 0 ? settings() : openAddLink(navigation));
  return (
    <div className="flex size-full items-center justify-center p-4">
      <Surface shape="extraLarge" color={md('surfaceContainer')} contentColor={md('onSurface')} className="w-full max-w-[420px]">
        <div className={`flex flex-col items-center gap-3 px-6 pt-6 ${guide.links.length > 0 ? 'pb-3' : 'pb-6'}`}>
          <div className="flex size-14 items-center justify-center rounded-full bg-primary-container text-on-primary-container">
            <Drawable name={icon} size={28} />
          </div>
          <Text typography="titleLarge" textAlign="center">
            {guide.title}
          </Text>
          <Text typography="bodyMedium" textAlign="center" color={md('onSurfaceVariant')}>
            {guide.text}
          </Text>
          {guide.links.length > 0 ? (
            <>
              <div className="mt-2 flex w-full flex-col gap-2">
                {guide.links.map((link) => (
                  <OfflineLinkRow key={link.index} link={link} busy={guide.busy} onEdit={settings} className="rounded-shape-lg" containerColor={md('surfaceContainerHigh')} />
                ))}
              </div>
              <TextButton onClick={open}>{label}</TextButton>
            </>
          ) : (
            <Button onClick={open}>{label}</Button>
          )}
        </div>
      </Surface>
    </div>
  );
};

const openConnectionSettings = (navigation: AppNavigationState, onDismiss: () => void) => {
  navigation.settingsPage.value = CONNECTIONS_SETTINGS_PAGE;
  onDismiss();
};

export const OfflineStatusSheet = ({ onDismiss }: { onDismiss: () => void }) => {
  const navigation = useContext(LocalAppNavigation);
  const [status, setStatus] = useState<OfflineStatus | null>(null);
  const hint = useRememberLookingHint(status !== null && !status.noLinks);
  const refresh = async () => setStatus(offlineStatus(await Qgc.get(OFFLINE_STATUS_VIEW)));
  useTicker(OFFLINE_POLL_MS, [], refresh);
  useOpenSignIn(status?.signInUrl ?? '');
  return (
    <AircastSheet onDismissRequest={onDismiss} label={status?.title ?? 'Connection'}>
      {status !== null ? (
        <div className="flex w-full flex-col gap-1 pb-6">
          <Text typography="titleMedium" className="px-sheet">
            {status.title}
          </Text>
          {status.onDevice ? null : (
            <Text typography="bodyMedium" className="px-sheet">
              {hint}
            </Text>
          )}
          {status.editAddress ? (
            <button type="button" className="flex items-center gap-2 px-sheet text-left" onClick={() => openConnectionSettings(navigation, onDismiss)}>
              <Footnote status={status} />
            </button>
          ) : (
            <div className="flex items-center gap-2 px-sheet">
              <Footnote status={status} />
            </div>
          )}
          {status.onDevice ? (
            <div className="w-full px-sheet py-2">
              <Button
                className="w-full"
                onClick={() => {
                  openDeviceTelemetry();
                  onDismiss();
                }}
              >
                {`${CHECK_DEVICE} telemetry`}
              </Button>
            </div>
          ) : null}
          {status.noLinks ? (
            <div className="w-full px-sheet py-2">
              <Button
                className="w-full"
                onClick={() => {
                  openAddLink(navigation);
                  onDismiss();
                }}
              >
                Add a link
              </Button>
            </div>
          ) : null}
          {status.links.map((link) => (
            <OfflineLinkRow key={link.index} link={link} busy={status.busy} onEdit={() => openConnectionSettings(navigation, onDismiss)} onActed={refresh} />
          ))}
          <HorizontalDivider />
          <div className="px-sheet pt-2">
            {status.noLinks && !status.onDevice ? (
              <TextButton
                onClick={() => {
                  navigation.settingsPage.value = FLASHER_PAGE;
                  onDismiss();
                }}
              >
                New drone? Flash an Aircast card
              </TextButton>
            ) : (
              <TextButton onClick={() => openConnectionSettings(navigation, onDismiss)}>{CONNECTION_SETTINGS}</TextButton>
            )}
          </div>
        </div>
      ) : null}
    </AircastSheet>
  );
};

const Footnote = ({ status }: { status: OfflineStatus }) => (
  <>
    {status.busy ? <CircularProgressIndicator size={16} strokeWidth={2} /> : null}
    <Text typography="bodySmall" color={md('onSurfaceVariant')} className="whitespace-pre-line">
      {status.footnote}
    </Text>
  </>
);
