import { useEffect, useLayoutEffect, useRef, useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, isNull, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optLong, optString, optText } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { createDocument } from '@/kotlin/Platform';
import { coerceAtLeast, delay } from '@/kotlin/Std';
import { DropdownMenu, DropdownMenuItem, FilledTonalButton, FilterChip, HorizontalDivider, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { Color, aircast, md } from '@/theme/Colors';
import { LoggingCategoriesDialog } from '@/ui/LoggingCategories';
import { sentenceCase } from '@/ui/SettingsScreen';

export const APP_LOG_VIEW = 'view.appLog';
export const APP_LOG_CLEAR = 'appLog.clear';
export const APP_LOG_SAVE = 'appLog.save';
export const APP_LOG_SAVE_FORMAT = 'settings.logManagerSettings.saveFormat.rawValue';
const SAVE_FORMAT_CSV = 1;
const APP_LOG_POLL_MS = 500;
const FILTER_DEBOUNCE_MS = 200;
const LEVEL_WARNING = 2;
const LEVEL_CRITICAL = 3;
const MONOSPACE = 'ui-monospace, monospace';

export type AppLogFilter = { levelIndex: number; category: string; text: string; regex: boolean };
export const AppLogFilter = (fields: Partial<AppLogFilter> = {}): AppLogFilter => ({ levelIndex: 0, category: '', text: '', regex: false, ...fields });

export type AppLogEntry = { sequence: number; level: number; message: string; category: string; timestamp: string; source: string };

export type AppLogRead = {
  levels: readonly string[];
  categories: readonly string[];
  regexValid: boolean;
  first: number | null;
  entries: readonly AppLogEntry[];
};

const encoded = (text: string): string =>
  encodeURIComponent(text)
    .replaceAll('%20', '+')
    .replace(/[!'()~]/g, (character) => `%${character.charCodeAt(0).toString(16).toUpperCase()}`);

export const appLogPath = (filter: AppLogFilter, after: number | null): string =>
  `${APP_LOG_VIEW}(${coerceAtLeast(filter.levelIndex - 1, 0)},${encoded(filter.category)},${encoded(filter.text)},${filter.regex ? 1 : 0},${after ?? ''})`;

export const appLogRead = (view: Json | null): AppLogRead | null =>
  view === null || optString(view, 'class') !== 'AppLog'
    ? null
    : {
        levels: jsonStrings(optJSONArray(view, 'levels')),
        categories: jsonStrings(optJSONArray(view, 'categories')),
        regexValid: optBoolean(view, 'regexValid', true),
        first: isNull(view, 'first') ? null : optLong(view, 'first'),
        entries: jsonObjects(optJSONArray(view, 'entries')).map((entry) => ({
          sequence: optLong(entry, 'sequence'),
          level: optInt(entry, 'level'),
          message: optText(entry, 'message'),
          category: optText(entry, 'category'),
          timestamp: optText(entry, 'timestamp'),
          source: optText(entry, 'source'),
        })),
      };

export const mergedEntries = (held: readonly AppLogEntry[], read: AppLogRead): readonly AppLogEntry[] => [
  ...held.filter((entry) => read.first !== null && entry.sequence >= read.first),
  ...read.entries,
];

export const appLogFileName = (saveFormat: Json | null): string => (saveFormat !== null && optInt(saveFormat, 'value') === SAVE_FORMAT_CSV ? 'QGCConsole.csv' : 'QGCConsole.txt');

export const appLogMime = (fileName: string): string => (fileName.toLowerCase().endsWith('.csv') ? 'text/csv' : 'text/plain');

const useLevelColor = (level: number): string => {
  if (level === 0) return md('onSurfaceVariant');
  if (level === LEVEL_WARNING) return aircast('warning');
  if (level >= LEVEL_CRITICAL) return aircast('alert');
  return md('onSurface');
};

const atEnd = (list: HTMLElement): boolean => list.scrollHeight - list.scrollTop - list.clientHeight <= 1;

export const AppLogPage = ({ className }: { className?: string }) => {
  const [filter, setFilter] = useState(AppLogFilter());
  const [entries, setEntries] = useState<readonly AppLogEntry[]>([]);
  const [read, setRead] = useState<AppLogRead | null>(null);
  const [cleared, setCleared] = useState(0);
  const [notice, setNotice] = useState<string | null>(null);
  const [following, setFollowing] = useState(true);
  const [showCategories, setShowCategories] = useState(false);
  const [atBottom, setAtBottom] = useState(true);
  const list = useRef<HTMLDivElement>(null);

  useLaunchedEffect([filter.levelIndex, filter.category, filter.text, filter.regex, cleared], async (scope) => {
    await delay(FILTER_DEBOUNCE_MS);
    if (!scope.isActive()) return;
    setEntries([]);
    const poll = async (held: readonly AppLogEntry[]): Promise<void> => {
      if (!scope.isActive()) return;
      const next = appLogRead(await Qgc.get(appLogPath(filter, held.at(-1)?.sequence ?? null)));
      if (!scope.isActive()) return;
      const merged = next === null ? held : mergedEntries(held, next);
      if (next !== null) {
        setRead(next);
        setEntries(merged);
      }
      await delay(APP_LOG_POLL_MS);
      void poll(merged);
    };
    void poll([]);
  });

  useEffect(() => {
    if (!atBottom) setFollowing(false);
  }, [atBottom]);

  useLayoutEffect(() => {
    const element = list.current;
    if (element === null) return;
    if (following && entries.length > 0) element.scrollTop = element.scrollHeight;
    setAtBottom(atEnd(element));
  }, [entries.length, following]);

  const save = () =>
    void (async () => {
      const saveName = appLogFileName(await Qgc.get(APP_LOG_SAVE_FORMAT));
      const saved = await Qgc.invokeResult(APP_LOG_SAVE, saveName);
      if (typeof saved !== 'string') {
        setNotice('The log could not be read.');
        return;
      }
      try {
        createDocument(saveName, appLogMime(saveName), saved);
        setNotice(null);
      } catch {
        setNotice('The file could not be written.');
      }
    })();

  return (
    <div className={`flex h-full flex-col ${className ?? ''}`}>
      <div className="relative w-full flex-1 overflow-hidden">
        {entries.length === 0 ? (
          <Text color={md('onSurfaceVariant')} className="absolute top-1/2 left-1/2 -translate-x-1/2 -translate-y-1/2">
            No log entries
          </Text>
        ) : null}
        <div ref={list} className="h-full w-full overflow-y-auto" onScroll={(event) => setAtBottom(atEnd(event.currentTarget))}>
          {entries.map((entry, index) => (
            <AppLogRow key={entry.sequence} entry={entry} index={index} />
          ))}
        </div>
        {!atBottom && entries.length > 0 ? (
          <FilledTonalButton onClick={() => setFollowing(true)} className="absolute top-2 left-1/2 -translate-x-1/2">
            Show latest
          </FilledTonalButton>
        ) : null}
      </div>
      <HorizontalDivider />
      <AppLogFilterBar
        filter={filter}
        read={read}
        onFilter={setFilter}
        onCategories={() => setShowCategories(true)}
        onSave={save}
        onClear={() =>
          void (async () => {
            await Qgc.invoke(APP_LOG_CLEAR);
            setCleared((it) => it + 1);
          })()
        }
      />
      {notice !== null ? (
        <Text color={aircast('alert')} className="block px-3 py-1">
          {notice}
        </Text>
      ) : null}
      {showCategories ? <LoggingCategoriesDialog onDismiss={() => setShowCategories(false)} /> : null}
    </div>
  );
};

const AppLogRow = ({ entry, index }: { entry: AppLogEntry; index: number }) => {
  const colour = useLevelColor(entry.level);
  return (
    <div
      className="flex w-full items-stretch gap-2 pr-2 [contain-intrinsic-size:auto_44px] [content-visibility:auto]"
      style={{ background: index % 2 === 0 ? md('surface') : md('surfaceContainer') }}
    >
      <div className="w-[3px] shrink-0" style={{ background: entry.level >= LEVEL_WARNING ? colour : Color.Transparent }} />
      <div className="min-w-0 flex-1 py-1">
        <Text typography="bodySmall" color={colour} monospace maxLines={2} ellipsis className="block">
          {entry.message}
        </Text>
        <Text typography="labelSmall" color={md('onSurfaceVariant')} monospace maxLines={1} ellipsis className="block">
          {[entry.timestamp, entry.category, entry.source].filter((it) => it !== '').join('  ')}
        </Text>
      </div>
    </div>
  );
};

const AppLogFilterBar = ({
  filter,
  read,
  onFilter,
  onCategories,
  onSave,
  onClear,
}: {
  filter: AppLogFilter;
  read: AppLogRead | null;
  onFilter: (filter: AppLogFilter) => void;
  onCategories: () => void;
  onSave: () => void;
  onClear: () => void;
}) => {
  const levels = read?.levels ?? [];
  const categories = read?.categories ?? [];
  return (
    <div className="flex w-full flex-col gap-2 bg-surface-container p-2">
      <div className="flex items-center gap-2 overflow-x-auto">
        <Picker label={sentenceCase(levels[filter.levelIndex] ?? 'All Levels')} options={levels.map(sentenceCase)} onPick={(index) => onFilter({ ...filter, levelIndex: index })} />
        <Picker
          label={sentenceCase(filter.category === '' ? 'All Categories' : filter.category)}
          options={categories.map(sentenceCase)}
          onPick={(index) => onFilter({ ...filter, category: index === 0 ? '' : (categories[index] ?? '') })}
        />
        <OutlinedButton onClick={onCategories} className="shrink-0">
          Categories
        </OutlinedButton>
        <OutlinedButton onClick={onSave} className="shrink-0">
          Save
        </OutlinedButton>
        <OutlinedButton onClick={onClear} className="shrink-0">
          Clear
        </OutlinedButton>
      </div>
      <div className="flex items-center gap-2">
        <OutlinedTextField value={filter.text} onValueChange={(text) => onFilter({ ...filter, text })} placeholder="Search…" isError={read?.regexValid === false} className="flex-1" />
        <FilterChip selected={filter.regex} onClick={() => onFilter({ ...filter, regex: !filter.regex })} label={<span style={{ fontFamily: MONOSPACE }}>.*</span>} />
      </div>
    </div>
  );
};

const Picker = ({ label, options, onPick }: { label: string; options: readonly string[]; onPick: (index: number) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative shrink-0">
      <TextButton onClick={() => setOpen(true)} enabled={options.length > 0}>
        {label}
      </TextButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {options.map((option, index) => (
          <DropdownMenuItem
            key={index}
            text={option}
            onClick={() => {
              setOpen(false);
              onPick(index);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};
