import { useEffect, useMemo, useRef, useState } from 'react';
import { AppCommands } from '@/bridge/BridgeCommands';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { type Json, isJson, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { savedFile } from '@/core/files';
import { createDocument } from '@/kotlin/Platform';
import { inTurn } from '@/kotlin/Std';
import { AlertDialog, Button, Checkbox, FilledTonalButton, LinearProgressIndicator, OutlinedButton, Surface, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { BlocksNavigation, LOG_DOWNLOAD_BLOCK } from '@/ui/AppNavigation';
import { RecordedFlights } from '@/ui/RecordedFlights';
import { EmptyState, SectionHeader } from '@/ui/SetupRow';

const LOG_ROOT = 'logDownload';
const LOG_MODEL = 'logDownload.model';
const LOGS_VIEW = 'view.logs';

export const TIME_UNRECEIVED = 'unreceived';
export const TIME_UNKNOWN = 'unknown';

const CLOCK = new Intl.DateTimeFormat(undefined, { timeStyle: 'short' });
const MONTH = new Intl.DateTimeFormat(undefined, { month: 'short' });

const sameDay = (a: Date, b: Date): boolean => a.getFullYear() === b.getFullYear() && a.getMonth() === b.getMonth() && a.getDate() === b.getDate();

export const compactLogTime = (time: Date, today: Date): string => {
  if (sameDay(time, today)) return CLOCK.format(time);
  if (sameDay(time, new Date(today.getFullYear(), today.getMonth(), today.getDate() - 1))) return 'Yesterday';
  const day = `${MONTH.format(time)} ${time.getDate()}`;
  return time.getFullYear() === today.getFullYear() ? day : `${day}, ${time.getFullYear()}`;
};

const LOCAL_TIME = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2}(\.\d{1,9})?)?$/;
const OFFSET_TIME = /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}(:\d{2}(\.\d{1,9})?)?(Z|[+-]\d{2}:\d{2})$/;

export const logLocalTime = (raw: string): Date | null => {
  if (!OFFSET_TIME.test(raw) && !LOCAL_TIME.test(raw)) return null;
  const parsed = new Date(raw);
  return Number.isNaN(parsed.getTime()) ? null : parsed;
};

export const logTimeText = (raw: string, state: string, format: (time: Date) => string): string => {
  if (state === TIME_UNRECEIVED) return '';
  if (state === TIME_UNKNOWN) return 'Date unknown';
  const time = logLocalTime(raw);
  return time === null ? 'Date unknown' : format(time);
};

export type LogEntry = {
  index: number;
  id: number;
  time: string;
  sizeStr: string;
  sizeBytes: number;
  downloadedBytes: number;
  received: boolean;
  selected: boolean;
  status: string;
  downloading: boolean;
  saved: boolean;
  savedPath: string;
};

export const LogEntry = (
  fields: Omit<LogEntry, 'downloading' | 'saved' | 'savedPath' | 'sizeBytes' | 'downloadedBytes'> & { downloading?: boolean; saved?: boolean; savedPath?: string; sizeBytes?: number; downloadedBytes?: number },
): LogEntry => ({ downloading: false, saved: false, savedPath: '', sizeBytes: 0, downloadedBytes: 0, ...fields });

export const ON_THE_VEHICLE = 'On the vehicle';

export type DownloadCard = { title: string; progress: string; fraction: number | undefined };

export const downloadCard = (entries: readonly LogEntry[]): DownloadCard => {
  const fetching = entries.find((it) => it.downloading);
  if (fetching === undefined) return { title: 'Downloading', progress: '', fraction: undefined };
  return { title: `Downloading log ${fetching.id}`, progress: fetching.status, fraction: fetching.sizeBytes > 0 ? Math.min(fetching.downloadedBytes / fetching.sizeBytes, 1) : undefined };
};

export const downloadLabel = (selected: number): string => (selected > 0 ? `Download (${selected})` : 'Download');

export const newlySaved = (entries: readonly LogEntry[], awaited: ReadonlySet<number>): readonly LogEntry[] => entries.filter((it) => awaited.has(it.id) && it.saved && it.savedPath !== '');

export type LogsView = {
  connected: boolean;
  entries: readonly LogEntry[];
  emptyText: string;
  eraseWarning: string;
  canRefresh: boolean;
  canDownload: boolean;
  canCancel: boolean;
  canErase: boolean;
  eraseSelectedShown: boolean;
  canEraseSelected: boolean;
  canSort: boolean;
  sortText: string;
  busy: boolean;
  downloading: boolean;
  anyDownloaded: boolean;
  savePath: string;
  savePathReason: string;
};

export const logsView = (view: Json | null): LogsView | null =>
  view === null
    ? null
    : {
        connected: optBoolean(view, 'connected'),
        entries: (optJSONArray(view, 'entries') ?? []).flatMap((entry, index) =>
          !isJson(entry)
            ? []
            : [
                LogEntry({
                  index: optInt(entry, 'index', index),
                  id: optInt(entry, 'id'),
                  time: logTimeText(optText(entry, 'time'), optText(entry, 'timeState'), (it) => compactLogTime(it, new Date())),
                  sizeStr: optText(entry, 'sizeText'),
                  sizeBytes: optInt(entry, 'sizeBytes'),
                  downloadedBytes: optInt(entry, 'downloadedBytes'),
                  received: optBoolean(entry, 'received'),
                  selected: optBoolean(entry, 'selected'),
                  status: optText(entry, 'status'),
                  downloading: optText(entry, 'statusId') === 'downloading',
                  saved: optText(entry, 'statusId') === 'downloaded',
                  savedPath: optText(entry, 'savedPath'),
                }),
              ],
        ),
        emptyText: optText(view, 'emptyText'),
        eraseWarning: optText(view, 'eraseWarning'),
        canRefresh: optBoolean(view, 'canRefresh'),
        canDownload: optBoolean(view, 'canDownload'),
        canCancel: optBoolean(view, 'canCancel'),
        canErase: optBoolean(view, 'canErase'),
        eraseSelectedShown: optBoolean(view, 'eraseSelectedShown'),
        canEraseSelected: optBoolean(view, 'canEraseSelected'),
        canSort: optBoolean(view, 'canSort'),
        sortText: optText(view, 'sortText'),
        busy: optBoolean(view, 'busy'),
        downloading: optBoolean(view, 'downloading'),
        anyDownloaded: optBoolean(view, 'anyDownloaded'),
        savePath: optText(view, 'savePath'),
        savePathReason: optText(view, 'savePathReason'),
      };

export type EraseKind = { readonly name: 'All' | 'Selected'; readonly ordinal: number; readonly action: string; readonly title: string; readonly text: string; readonly confirm: string };

export const EraseKind = {
  All: {
    name: 'All',
    ordinal: 0,
    action: 'eraseAll',
    title: 'Delete All Onboard Log Files',
    text: 'All onboard log files will be erased permanently. Is this really what you want?',
    confirm: 'Erase All',
  },
  Selected: {
    name: 'Selected',
    ordinal: 1,
    action: 'eraseSelected',
    title: 'Delete Selected Onboard Log Files',
    text: 'The selected onboard log files will be erased permanently. Is this really what you want?',
    confirm: 'Erase Selected',
  },
} as const satisfies Record<string, EraseKind>;

export const EraseKindEntries: readonly EraseKind[] = [EraseKind.All, EraseKind.Selected];

const EraseConfirmDialog = ({ erase, onConfirm, onDismiss }: { erase: EraseKind; onConfirm: () => void; onDismiss: () => void }) => (
  <AlertDialog
    onDismissRequest={onDismiss}
    title={erase.title}
    text={erase.text}
    confirmButton={
      <TextButton
        contentColor={md('error')}
        onClick={() => {
          onConfirm();
          onDismiss();
        }}
      >
        {erase.confirm}
      </TextButton>
    }
    dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
  />
);

const LOG_NOT_SAVED = 'The log could not be saved';

const saveToDevice = (path: string): Promise<void> =>
  savedFile(path).then(
    (file) => (file === null ? undefined : createDocument(file.name, 'application/octet-stream', file)),
    (error: unknown) => void AppCommands.notice(LOG_NOT_SAVED, String(error)),
  );

const LogRow = ({ entry, enabled, onToggle }: { entry: LogEntry; enabled: boolean; onToggle: (checked: boolean) => void }) => {
  const selectable = enabled && entry.received;
  return (
    <div className="flex w-full items-center" style={{ background: entry.selected ? md('secondaryContainer') : 'transparent' }}>
      <button
        type="button"
        role="checkbox"
        aria-checked={entry.selected}
        disabled={!selectable}
        onClick={() => onToggle(!entry.selected)}
        className="flex min-h-[72px] min-w-0 flex-1 items-center gap-4 px-4 py-3 text-left"
      >
        <span className="grid size-10 shrink-0 place-items-center rounded-full" style={{ background: entry.selected ? md('primary') : md('secondaryContainer') }}>
          <Drawable name={entry.selected ? 'ic_check_circle' : 'ic_description'} size={24} style={{ color: entry.selected ? md('onPrimary') : md('onSecondaryContainer') }} />
        </span>
        <span className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge">{`Log ${entry.id}`}</Text>
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {[entry.time, entry.sizeStr].filter((it) => it.trim() !== '').join(' · ')}
          </Text>
        </span>
        <Text typography="labelMedium" color={md('primary')}>
          {entry.status}
        </Text>
      </button>
      {entry.saved && entry.savedPath !== '' && !__DESKTOP__ ? (
        <TextButton label={`Save log ${entry.id} again`} onClick={() => void saveToDevice(entry.savedPath)} className="mr-2">
          Save again
        </TextButton>
      ) : null}
    </div>
  );
};

export const savedToText = (logs: LogsView | null): string | null => {
  if (logs === null || !logs.anyDownloaded) return null;
  if (logs.savePath.trim() !== '') return `Saved to ${logs.savePath}`;
  if (logs.savePathReason.trim() !== '') return logs.savePathReason;
  return null;
};

export const shouldAutoRefreshLogs = (hasVehicle: boolean, busy: boolean): boolean => hasVehicle && !busy;

const invoke = (action: string): void => void Qgc.invoke(`${LOG_ROOT}.${action}`);

const select = (entry: LogEntry, selected: boolean): Promise<boolean> => Qgc.set(`${LOG_MODEL}.${entry.index}.selected`, selected);

export const LogDownloadScreen = ({ className }: { className?: string }) => {
  const json = useQgcPath(LOGS_VIEW);
  const logs = useMemo(() => logsView(json), [json]);
  const [confirmErase, setConfirmErase] = useState<EraseKind | null>(null);
  const awaited = useRef<ReadonlySet<number>>(new Set());

  const entries = logs?.entries ?? [];
  const selectedCount = entries.filter((it) => it.selected).length;
  const selectable = entries.filter((it) => it.received);
  const busy = logs?.busy === true;
  const connected = logs?.connected === true;

  useEffect(() => {
    if (shouldAutoRefreshLogs(connected, busy)) invoke('refresh');
  }, [connected]);

  useEffect(() => {
    const saved = newlySaved(entries, awaited.current);
    if (saved.length === 0) return;
    awaited.current = new Set([...awaited.current].filter((id) => !saved.some((it) => it.id === id)));
    void inTurn(saved, (it) => saveToDevice(it.savedPath));
  }, [entries]);

  const download = () => {
    awaited.current = __DESKTOP__ ? new Set() : new Set(entries.filter((it) => it.selected).map((it) => it.id));
    invoke('download');
  };

  const blocks = <BlocksNavigation blocking={logs?.downloading === true} reason={LOG_DOWNLOAD_BLOCK} />;

  if (logs === null || !connected) {
    return (
      <div className={`flex h-full w-full flex-col overflow-auto ${className ?? ''}`}>
        {blocks}
        <RecordedFlights />
        <EmptyState
          icon="ic_description"
          title="No vehicle"
          text={logs !== null && logs.emptyText.trim() !== '' ? logs.emptyText : 'Connect a vehicle to download its flight logs.'}
        />
      </div>
    );
  }

  const card = downloadCard(entries);
  const savedTo = savedToText(logs);
  return (
    <div className={`relative h-full w-full ${className ?? ''}`}>
      {blocks}
      {confirmErase !== null ? <EraseConfirmDialog erase={confirmErase} onConfirm={() => invoke(confirmErase.action)} onDismiss={() => setConfirmErase(null)} /> : null}
      <div className="flex h-full w-full flex-col">
        <RecordedFlights />
        <div className="flex w-full flex-wrap gap-2 px-4 py-2">
          <FilledTonalButton onClick={() => invoke('refresh')} enabled={logs.canRefresh}>
            Refresh
          </FilledTonalButton>
          <Button onClick={download} enabled={logs.canDownload && selectedCount > 0} title={selectedCount > 0 ? undefined : 'Select the logs to download'}>
            <Drawable name="ic_download" />
            {downloadLabel(selectedCount)}
          </Button>
          {logs.sortText.trim() !== '' ? (
            <FilledTonalButton onClick={() => invoke('toggleSortByDate')} enabled={logs.canSort}>
              {sentenceCase(logs.sortText)}
            </FilledTonalButton>
          ) : null}
        </div>
        {selectable.length > 0 && !busy ? (
          <div
            className="flex w-full cursor-pointer items-center px-4 py-1"
            onClick={() => {
              const selectAll = selectedCount < selectable.length;
              void Promise.all(selectable.map((entry) => select(entry, selectAll)));
            }}
          >
            <Checkbox checked={selectedCount === selectable.length} onCheckedChange={null} label={selectedCount < selectable.length ? 'Select all' : 'Deselect all'} />
            <Text typography="labelLarge" className="pl-2">
              {selectedCount < selectable.length ? 'Select all' : 'Deselect all'}
            </Text>
          </div>
        ) : null}
        {busy ? (
          <div className="w-full px-4 py-2">
            <Surface shape="large" color={md('surfaceContainerHigh')}>
              <div className="flex flex-col gap-2.5 p-4">
                <div className="flex items-center">
                  <Text typography="titleSmall" className="flex-1">
                    {card.title}
                  </Text>
                  {card.progress.trim() !== '' ? (
                    <Text typography="labelMedium" color={md('onSurfaceVariant')}>
                      {card.progress}
                    </Text>
                  ) : null}
                  {logs.canCancel ? <TextButton onClick={() => invoke('cancel')}>Cancel</TextButton> : null}
                </div>
                <LinearProgressIndicator progress={card.fraction} label={card.title} />
              </div>
            </Surface>
          </div>
        ) : null}
        {entries.length === 0 ? (
          <EmptyState icon="ic_description" title="No logs yet" text={logs.emptyText.trim() === '' ? 'This vehicle reports no flight logs.' : logs.emptyText} />
        ) : (
          <div className="min-h-0 w-full flex-1 overflow-auto">
            <SectionHeader text={ON_THE_VEHICLE} />
            {entries.map((entry) => (
              <LogRow key={entry.index} entry={entry} enabled={!busy} onToggle={(checked) => void select(entry, checked)} />
            ))}
            <div>
              {!busy ? (
                <div className="flex gap-2 px-4 py-2">
                  {logs.eraseSelectedShown ? (
                    <OutlinedButton enabled={logs.canEraseSelected} contentColor={md('error')} onClick={() => setConfirmErase(EraseKind.Selected)}>
                      Erase selected
                    </OutlinedButton>
                  ) : null}
                  <OutlinedButton enabled={logs.canErase} contentColor={md('error')} onClick={() => setConfirmErase(EraseKind.All)}>
                    Erase all
                  </OutlinedButton>
                </div>
              ) : null}
            </div>
          </div>
        )}
        {savedTo !== null ? (
          <Text typography="bodySmall" className="block w-full p-4">
            {savedTo}
          </Text>
        ) : null}
      </div>
    </div>
  );
};
