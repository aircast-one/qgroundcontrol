import { useEffect, useMemo, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { type Json, isJson, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { AlertDialog, Checkbox, ExtendedFloatingActionButton, FilledTonalButton, LinearProgressIndicator, OutlinedButton, Surface, Text, TextButton } from '@/m3';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import { md } from '@/theme/Colors';
import { BlocksNavigation, LOG_DOWNLOAD_BLOCK } from '@/ui/AppNavigation';
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
  received: boolean;
  selected: boolean;
  status: string;
  downloading: boolean;
  saved: boolean;
};

export const LogEntry = (fields: Omit<LogEntry, 'downloading' | 'saved'> & { downloading?: boolean; saved?: boolean }): LogEntry => ({ downloading: false, saved: false, ...fields });

export const logSections = (entries: readonly LogEntry[]): (readonly [string, readonly LogEntry[]])[] =>
  (
    [
      ['On the vehicle', entries.filter((it) => !it.saved)],
      ['On this phone', entries.filter((it) => it.saved)],
    ] as const
  ).filter(([, section]) => section.length > 0);

export const downloadCard = (entries: readonly LogEntry[]): readonly [string, string] => {
  const fetching = entries.find((it) => it.downloading);
  return fetching === undefined ? ['Downloading', ''] : [`Downloading log ${fetching.id}`, fetching.status];
};

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
                  received: optBoolean(entry, 'received'),
                  selected: optBoolean(entry, 'selected'),
                  status: optText(entry, 'status'),
                  downloading: optText(entry, 'statusId') === 'downloading',
                  saved: optText(entry, 'statusId') === 'downloaded',
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

const LogRow = ({ entry, enabled, onToggle }: { entry: LogEntry; enabled: boolean; onToggle: (checked: boolean) => void }) => {
  const selectable = enabled && entry.received;
  return (
    <button
      type="button"
      role="checkbox"
      aria-checked={entry.selected}
      disabled={!selectable}
      onClick={() => onToggle(!entry.selected)}
      className="flex min-h-[72px] w-full items-center gap-4 px-4 py-3 text-left"
      style={{ background: entry.selected ? md('secondaryContainer') : 'transparent' }}
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

  const entries = logs?.entries ?? [];
  const selectedCount = entries.filter((it) => it.selected).length;
  const selectable = entries.filter((it) => it.received);
  const busy = logs?.busy === true;
  const connected = logs?.connected === true;

  useEffect(() => {
    if (shouldAutoRefreshLogs(connected, busy)) invoke('refresh');
  }, [connected]);

  const blocks = <BlocksNavigation blocking={logs?.downloading === true} reason={LOG_DOWNLOAD_BLOCK} />;

  if (logs === null || !connected) {
    return (
      <>
        {blocks}
        <EmptyState
          icon="ic_description"
          title="No vehicle"
          text={logs !== null && logs.emptyText.trim() !== '' ? logs.emptyText : 'Connect a vehicle to download its flight logs.'}
          className={className}
        />
      </>
    );
  }

  const [title, progress] = downloadCard(entries);
  const savedTo = savedToText(logs);
  return (
    <div className={`relative h-full w-full ${className ?? ''}`}>
      {blocks}
      {confirmErase !== null ? <EraseConfirmDialog erase={confirmErase} onConfirm={() => invoke(confirmErase.action)} onDismiss={() => setConfirmErase(null)} /> : null}
      <div className="flex h-full w-full flex-col">
        <div className="flex w-full flex-wrap gap-2 px-4 py-2">
          <FilledTonalButton onClick={() => invoke('refresh')} enabled={logs.canRefresh}>
            Refresh
          </FilledTonalButton>
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
                    {title}
                  </Text>
                  {progress.trim() !== '' ? (
                    <Text typography="labelMedium" color={md('onSurfaceVariant')}>
                      {progress}
                    </Text>
                  ) : null}
                  {logs.canCancel ? <TextButton onClick={() => invoke('cancel')}>Cancel</TextButton> : null}
                </div>
                <LinearProgressIndicator />
              </div>
            </Surface>
          </div>
        ) : null}
        {entries.length === 0 ? (
          <EmptyState icon="ic_description" title="No logs yet" text={logs.emptyText.trim() === '' ? 'This vehicle reports no flight logs.' : logs.emptyText} />
        ) : (
          <div className="min-h-0 w-full flex-1 overflow-auto">
            {logSections(entries).map(([sectionTitle, section]) => (
              <div key={sectionTitle}>
                <SectionHeader text={sectionTitle} />
                {section.map((entry) => (
                  <LogRow key={entry.index} entry={entry} enabled={!busy} onToggle={(checked) => void select(entry, checked)} />
                ))}
              </div>
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
              <div className="h-[88px]" />
            </div>
          </div>
        )}
        {savedTo !== null ? (
          <Text typography="bodySmall" className="block w-full p-4">
            {savedTo}
          </Text>
        ) : null}
      </div>
      {logs.canDownload && selectedCount > 0 ? (
        <ExtendedFloatingActionButton
          onClick={() => invoke('download')}
          label={`Download (${selectedCount})`}
          containerColor={md('primaryContainer')}
          contentColor={md('onPrimaryContainer')}
          className="absolute right-4 bottom-4"
        >
          <Drawable name="ic_download" />
          {`Download (${selectedCount})`}
        </ExtendedFloatingActionButton>
      ) : null}
    </div>
  );
};
