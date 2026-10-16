import { useState } from 'react';
import { type Json, type JsonArray, jsonObjects, optBoolean, optDouble, optInt, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect, useTicker } from '@/kotlin/Compose';
import { delay, format } from '@/kotlin/Std';
import { Button, OutlinedButton, Text, TextButton } from '@/m3';
import { argb, md } from '@/theme/Colors';
import { SectionHeader } from '@/ui/SetupRow';

export const NTRIP_VIEW = 'view.ntrip';
export const NTRIP_CONNECT_SETTING = 'settings.ntripSettings.ntripServerConnectEnabled';
const NTRIP_POLL_MS = 1000;
const NTRIP_GREEN = argb(0xff34c759);
const NTRIP_ORANGE = argb(0xffff9f0a);

export type NtripMountpointRow = { mountpoint: string; detail: string; selected: boolean };

export type NtripBrowser = { status: string; error: string; canBrowse: boolean; mountpoints: readonly NtripMountpointRow[] };

export type NtripStatus = {
  status: string;
  message: string;
  button: string;
  buttonEnabled: boolean;
  active: boolean;
  dataStale: boolean;
  mountpoint: string;
  messages: number;
  messageTypes: readonly (readonly [number, number])[];
  bytesReceived: number;
  dataRate: number;
  dataWarning: boolean;
  bytesSent: number;
  sentKBps: number;
  securityWarning: string;
  ggaSource: string;
  browser: NtripBrowser;
};

const pairAt = (pair: JsonArray, index: number): number => {
  const value = pair[index];
  const parsed = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(parsed) ? Math.trunc(parsed) : 0;
};

export const ntripStatus = (view: Json | null): NtripStatus | null =>
  view !== null && optText(view, 'status').trim() !== ''
    ? {
        status: optText(view, 'status'),
        message: optText(view, 'statusMessage'),
        button: optText(view, 'button'),
        buttonEnabled: optBoolean(view, 'buttonEnabled'),
        active: optBoolean(view, 'active'),
        dataStale: optBoolean(view, 'dataStale'),
        mountpoint: optText(view, 'mountpoint'),
        messages: optInt(view, 'messages'),
        messageTypes: (optJSONArray(view, 'messageTypes') ?? []).flatMap((pair) => (Array.isArray(pair) ? [[pairAt(pair, 0), pairAt(pair, 1)] as const] : [])),
        bytesReceived: optInt(view, 'bytesReceived'),
        dataRate: optDouble(view, 'dataRateBytesPerSec', 0),
        dataWarning: optBoolean(view, 'dataWarning'),
        bytesSent: optInt(view, 'bytesSent'),
        sentKBps: optDouble(view, 'sentKBps', 0),
        securityWarning: optText(view, 'securityWarning'),
        ggaSource: optText(view, 'ggaSource'),
        browser: ntripBrowser(optJSONObject(view, 'browser')),
      }
    : null;

export const ntripBrowser = (json: Json | null): NtripBrowser => ({
  status: optText(json, 'status'),
  error: optText(json, 'error'),
  canBrowse: optBoolean(json, 'canBrowse'),
  mountpoints: jsonObjects(optJSONArray(json, 'mountpoints')).map((row) => ({ mountpoint: optText(row, 'mountpoint'), detail: optText(row, 'detail'), selected: optBoolean(row, 'selected') })),
});

export const dataSize = (bytes: number): string =>
  bytes < 1024 ? `${bytes} B` : bytes < 1048576 ? format('%.1f KB', bytes / 1024.0) : format('%.1f MB', bytes / 1048576.0);

export const dataRate = (bytesPerSec: number): string => (bytesPerSec < 1024 ? format('%.0f B/s', bytesPerSec) : format('%.1f KB/s', bytesPerSec / 1024));

const dotColour = (status: string): string => {
  switch (status) {
    case 'connected':
      return NTRIP_GREEN;
    case 'connecting':
    case 'reconnecting':
      return NTRIP_ORANGE;
    case 'error':
      return md('error');
    default:
      return md('outline');
  }
};

export const NtripStatusSection = ({ onWrite }: { onWrite: () => void }) => {
  const [read, setRead] = useState<NtripStatus | null>(null);
  useTicker(NTRIP_POLL_MS, [], async () => setRead(ntripStatus(await Qgc.get(NTRIP_VIEW))));
  if (read === null) return null;
  const status = read;
  const connected = status.status === 'connected';
  return (
    <>
      <SectionHeader text="Connection" />
      <div className="flex w-full flex-col gap-1.5 px-5 py-2">
        <div className="flex items-center gap-2.5">
          <span className="block size-2.5 shrink-0 rounded-full" style={{ background: dotColour(status.status) }} />
          <Text className="flex-1">{status.message.trim() === '' ? 'Disconnected' : status.message}</Text>
          <Button
            enabled={status.buttonEnabled}
            onClick={() =>
              void (async () => {
                await Qgc.set(NTRIP_CONNECT_SETTING, !status.active);
                onWrite();
              })()
            }
          >
            {status.button}
          </Button>
        </div>
        {status.status !== 'disconnected' ? (
          <>
            {status.dataStale && connected ? <Text color={NTRIP_ORANGE}>Connected but no data received recently</Text> : null}
            {connected && status.mountpoint.trim() !== '' ? <StatusLine label="Mountpoint" value={status.mountpoint} /> : null}
            {connected && status.messages > 0 ? <StatusLine label="Messages" value={status.messages.toString()} /> : null}
            {connected && status.messageTypes.length > 0 ? (
              <>
                <Text typography="labelLarge">Message types</Text>
                <div className="flex flex-wrap gap-1.5">
                  {status.messageTypes.map(([id, count]) => (
                    <Text key={id} typography="bodySmall" className="rounded-full bg-surface-variant px-2 py-0.5">
                      {`${id === 0 ? 'unknown' : id.toString()} × ${count}`}
                    </Text>
                  ))}
                </div>
              </>
            ) : null}
            {connected && status.bytesReceived > 0 ? <StatusLine label="Data Received" value={`${dataSize(status.bytesReceived)} (${dataRate(status.dataRate)})`} /> : null}
            {connected && status.dataWarning ? (
              <Text typography="bodySmall" color={NTRIP_ORANGE}>
                {`Warning: Data usage: ${dataSize(status.bytesReceived)} — consider connection costs`}
              </Text>
            ) : null}
            {connected && status.bytesSent > 0 ? <StatusLine label="To Vehicle" value={`${dataSize(status.bytesSent)} (${format('%.1f', status.sentKBps)} KB/s)`} /> : null}
            {connected && status.ggaSource.trim() !== '' ? <StatusLine label="GGA Source" value={status.ggaSource} /> : null}
            {status.securityWarning.trim() !== '' ? (
              <Text typography="bodySmall" color={NTRIP_ORANGE}>
                {status.securityWarning}
              </Text>
            ) : null}
            {status.status === 'error' ? (
              <Text typography="bodySmall" color={md('error')}>
                {status.message}
              </Text>
            ) : null}
          </>
        ) : null}
      </div>
    </>
  );
};

const StatusLine = ({ label, value }: { label: string; value: string }) => (
  <div className="flex w-full">
    <Text typography="bodyMedium" className="flex-1">
      {label}
    </Text>
    <Text typography="bodyMedium">{value}</Text>
  </div>
);

export const NTRIP_FETCH_MOUNTPOINTS = 'ntrip.fetchMountpoints';
export const NTRIP_SELECT_MOUNTPOINT = 'ntrip.selectMountpoint';

export const NtripMountpointBrowser = ({ onWrite }: { onWrite: () => void }) => {
  const [browser, setBrowser] = useState<NtripBrowser | null>(null);
  const [refreshes, setRefreshes] = useState(0);
  const refresh = () => setRefreshes((count) => count + 1);
  useLaunchedEffect([refreshes], async (scope) => {
    const next = ntripStatus(await Qgc.get(NTRIP_VIEW))?.browser ?? null;
    if (!scope.isActive()) return;
    setBrowser(next);
    if (next?.status === 'inProgress') {
      await delay(NTRIP_POLL_MS);
      if (scope.isActive()) refresh();
    }
  });
  if (browser === null) return null;
  return (
    <div className="flex w-full flex-col gap-1.5 px-5 py-2">
      <div>
        <OutlinedButton
          enabled={browser.canBrowse}
          onClick={() =>
            void (async () => {
              await Qgc.invoke(NTRIP_FETCH_MOUNTPOINTS);
              refresh();
            })()
          }
        >
          Browse
        </OutlinedButton>
      </div>
      {browser.status === 'inProgress' ? <Text color={NTRIP_ORANGE}>Fetching mountpoints…</Text> : null}
      {browser.status === 'error' ? <Text color={md('error')}>{browser.error}</Text> : null}
      {browser.mountpoints.map((row) => (
        <div key={row.mountpoint} className="flex w-full items-center py-1">
          <div className="flex flex-1 flex-col">
            <div className="flex gap-1.5">
              <Text typography="bodyLarge" color={row.selected ? md('primary') : md('onSurface')}>
                {row.mountpoint}
              </Text>
              {row.selected ? (
                <Text typography="bodySmall" color={md('primary')}>
                  (selected)
                </Text>
              ) : null}
            </div>
            {row.detail.trim() !== '' ? (
              <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                {row.detail}
              </Text>
            ) : null}
          </div>
          <TextButton
            enabled={!row.selected}
            onClick={() =>
              void (async () => {
                await Qgc.invoke(NTRIP_SELECT_MOUNTPOINT, row.mountpoint);
                refresh();
                onWrite();
              })()
            }
          >
            {row.selected ? 'Selected' : 'Select'}
          </TextButton>
        </div>
      ))}
    </div>
  );
};
