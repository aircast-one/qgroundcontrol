import { useContext, useMemo, useState } from 'react';
import { LinkCommands } from '@/bridge/BridgeCommands';
import { type Json, has, jsonObjects, optBoolean, optInt, optJSONArray, optString } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { useTicker } from '@/kotlin/Compose';
import { Button, CircularProgressIndicator, HorizontalDivider, ListItem, Surface, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { type AppNavigationState, LocalAppNavigation } from './AppNavigation';
import { deviceRequested } from './DeviceStatus';
import { LOOKING_TITLE, useRememberLookingHint } from './ConnectingCard';
import { FLASHER_PAGE } from './FlasherScreen';

export const OFFLINE_STATUS_VIEW = 'view.offlineStatus';
const OFFLINE_LINKS_PATH = 'links.linkConfigurations';
const CONNECTIONS_SETTINGS_PAGE = 'Connections';
const OFFLINE_POLL_MS = 1000;

export type OfflineLink = { index: number; text: string; description: string; retry: boolean; failed: boolean; connected: boolean };

export type OfflineStatus = { title: string; footnote: string; busy: boolean; noLinks: boolean; editAddress: boolean; onDevice: boolean; links: readonly OfflineLink[] };

export const offlineStatus = (view: Json | null): OfflineStatus | null =>
  view !== null && has(view, 'title')
    ? {
        title: optString(view, 'title'),
        footnote: optString(view, 'footnote'),
        busy: optBoolean(view, 'busy'),
        noLinks: optBoolean(view, 'noLinks'),
        editAddress: optBoolean(view, 'editAddress'),
        onDevice: optBoolean(view, 'onDevice'),
        links: jsonObjects(optJSONArray(view, 'links')).map((link) => ({
          index: optInt(link, 'index'),
          text: optString(link, 'text'),
          description: optString(link, 'description'),
          retry: optBoolean(link, 'retry'),
          failed: optBoolean(link, 'failed'),
          connected: optBoolean(link, 'connected'),
        })),
      }
    : null;

export const OFFLINE_SHEET = 'offline';

export const CHECK_DEVICE = 'Check device';

export const openDeviceTelemetry = (): void => {
  deviceRequested.value = '/telemetry';
};

export const openAddLink = (navigation: AppNavigationState): void => {
  navigation.addLinkRequested.value = true;
  navigation.settingsPage.value = CONNECTIONS_SETTINGS_PAGE;
};

export type OfflineGuide = { title: string; text: string; onDevice: boolean };

export const offlineGuide = (status: OfflineStatus | null, hint: string): OfflineGuide => ({
  title: status === null ? LOOKING_TITLE : sentenceCase(status.title),
  text: status?.onDevice === true ? status.footnote : hint,
  onDevice: status?.onDevice === true,
});

export const OfflineGuideCard = () => {
  const navigation = useContext(LocalAppNavigation);
  const hint = useRememberLookingHint();
  const json = useQgcPath(OFFLINE_STATUS_VIEW);
  const guide = useMemo(() => offlineGuide(offlineStatus(json), hint), [json, hint]);
  return (
    <div className="flex size-full items-center justify-center p-4">
      <Surface shape="extraLarge" color={md('surfaceContainer')} contentColor={md('onSurface')} className="w-full max-w-[420px]">
        <div className="flex flex-col items-center gap-3 p-6">
          <div className="flex size-14 items-center justify-center rounded-full bg-primary-container text-on-primary-container">
            <Drawable name={guide.onDevice ? 'ic_developer_board' : 'ic_link'} size={28} />
          </div>
          <Text typography="titleLarge" textAlign="center">
            {guide.title}
          </Text>
          <Text typography="bodyMedium" textAlign="center" color={md('onSurfaceVariant')}>
            {guide.text}
          </Text>
          <Button onClick={() => (guide.onDevice ? openDeviceTelemetry() : openAddLink(navigation))}>{guide.onDevice ? CHECK_DEVICE : 'Add a link'}</Button>
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
  const hint = useRememberLookingHint();
  useTicker(OFFLINE_POLL_MS, [], async () => setStatus(offlineStatus(await Qgc.get(OFFLINE_STATUS_VIEW))));
  return (
    <AircastSheet onDismissRequest={onDismiss}>
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
                {'Add link…'}
              </Button>
            </div>
          ) : null}
          {status.links.map((link) => (
            <ListItem
              key={link.index}
              headlineContent={link.text}
              supportingContent={
                <span className="whitespace-pre-line" style={{ color: link.failed ? md('error') : md('onSurfaceVariant') }}>
                  {link.description}
                </span>
              }
              trailingContent={link.connected && status.busy ? <CircularProgressIndicator size={16} strokeWidth={2} /> : link.retry ? <Text typography="labelMedium">Retry</Text> : null}
              onClick={() => void (link.connected ? LinkCommands.disconnect(link.index) : LinkCommands.connect(`@${OFFLINE_LINKS_PATH}.${link.index}`))}
            />
          ))}
          <HorizontalDivider />
          <ListItem headlineContent="Connection settings" onClick={() => openConnectionSettings(navigation, onDismiss)} />
          {status.onDevice ? null : (
            <ListItem
              headlineContent="New drone? Flash an Aircast card"
              onClick={() => {
                navigation.settingsPage.value = FLASHER_PAGE;
                onDismiss();
              }}
            />
          )}
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
