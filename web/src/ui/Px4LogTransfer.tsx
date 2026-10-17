import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, jsonObjects, optBoolean, optDouble, optJSONArray, optJSONObject, optLong, optText } from '@/bridge/Json';
import { useKeyedState, useLaunchedEffect } from '@/kotlin/Compose';
import { coerceIn, delay } from '@/kotlin/Std';
import { AlertDialog, Checkbox, DropdownMenu, DropdownMenuItem, LinearProgressIndicator, OutlinedButton, OutlinedTextField, Text, TextButton } from '@/m3';
import { aircast, md } from '@/theme/Colors';
import { FootNote, SectionHeader } from '@/ui/SetupRow';

const MAVLINK_LOG_VIEW = 'view.mavlinkLog';
const MAVLINK_LOG_POLL_MS = 1000;
const TEXT_SAVE_DELAY_MS = 600;
const MAVLINK_LOG_SET = 'mavlinkLog.set';

export const WIND_SPEEDS: readonly (readonly [string, string])[] = [
  ['Not Set', '-1'],
  ['Calm', '0'],
  ['Breeze', '5'],
  ['Gale', '8'],
  ['Storm', '10'],
];

export const FLIGHT_RATINGS: readonly (readonly [string, string])[] = [
  ['Not Set', 'notset'],
  ['Crashed (Pilot Error)', 'crash_pilot'],
  ['Crashed (Software or Hardware Issue)', 'crash_sw_hw'],
  ['Unsatisfactory', 'unsatisfactory'],
  ['Good', 'good'],
  ['Great', 'great'],
];

export type LogFile = { name: string; size: number; uploaded: boolean; writing: boolean };

export const LogFile = (fields: { name: string; size: number; uploaded: boolean; writing?: boolean }): LogFile => ({ writing: false, ...fields });

export type MavlinkLog = {
  px4: boolean;
  running: boolean;
  canStart: boolean;
  persistence: boolean;
  settings: Json;
  files: readonly LogFile[];
  uploading: boolean;
  uploadingFile: string;
  uploadProgress: number;
  message: string;
};

export const mavlinkLog = (view: Json | null): MavlinkLog | null =>
  view === null
    ? null
    : {
        px4: optBoolean(view, 'vehiclePx4'),
        running: optBoolean(view, 'logRunning'),
        canStart: optBoolean(view, 'canStartLog'),
        persistence: optBoolean(view, 'persistence', true),
        settings: optJSONObject(view, 'settings') ?? {},
        files: jsonObjects(optJSONArray(view, 'files')).map((file) =>
          LogFile({ name: optText(file, 'name'), size: optLong(file, 'size'), uploaded: optBoolean(file, 'uploaded'), writing: optBoolean(file, 'writing') }),
        ),
        uploading: optBoolean(view, 'uploading'),
        uploadingFile: optText(view, 'uploadingFile'),
        uploadProgress: coerceIn(optDouble(view, 'uploadProgress', 0), 0, 1),
        message: optText(view, 'message'),
      };

const SIZE_FORMAT = new Intl.NumberFormat(undefined, { maximumFractionDigits: 0 });

export const logSizeText = (size: number): string => SIZE_FORMAT.format(size);

export const logListIdle = (log: MavlinkLog): boolean => !log.uploading && !log.running;

export const uploadEmail = (settings: Json, drafts: ReadonlyMap<string, string>): string => drafts.get('emailAddress') ?? optText(settings, 'emailAddress');

export const unsavedTexts = (settings: Json, drafts: ReadonlyMap<string, string>): (readonly [string, string])[] => [...drafts].filter(([field, typed]) => typed !== optText(settings, field));

export type LogConfirm = { title: string; question: string; run: () => void };

export const MAVLINK_LOGGING_TITLE = 'MAVLink Logging';
export const NO_PX4_VEHICLE = 'Connect a PX4 vehicle to manage logs';

type Call = readonly [string, readonly unknown[]];

const queue: { tail: Promise<unknown> } = { tail: Promise.resolve() };

const offMainInOrder = (block: () => Promise<unknown>): void => {
  queue.tail = queue.tail.then(block).catch(() => undefined);
};

const firstRefusal = (calls: readonly Call[]): Promise<string | null> =>
  calls.reduce<Promise<string | null>>(async (earlier, [path, args]) => {
    const before = await earlier;
    const refused = await Qgc.refusalOf(path, ...args);
    return before ?? refused;
  }, Promise.resolve(null));

const settingCalls = (unsaved: readonly (readonly [string, string])[]): Call[] => unsaved.map(([field, typed]) => [MAVLINK_LOG_SET, [field, typed]] as const);

export const Px4LogTransferPage = ({ className }: { className?: string }) => {
  const [read, setRead] = useState<MavlinkLog | null>(null);
  const [polls, setPolls] = useState(0);
  const [selected, setSelected] = useState<ReadonlySet<string>>(new Set());
  const [confirming, setConfirming] = useState<LogConfirm | null>(null);
  const [refusal, setRefusal] = useState<string | null>(null);
  const [drafts, setDrafts] = useState<ReadonlyMap<string, string>>(new Map());

  useLaunchedEffect([polls], async (scope) => {
    const poll = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const next = mavlinkLog(await Qgc.get(MAVLINK_LOG_VIEW));
      if (!scope.isActive()) return;
      setRead(next);
      await delay(MAVLINK_LOG_POLL_MS);
      return poll();
    };
    await poll();
  });

  const actAll = (calls: readonly Call[]) =>
    offMainInOrder(async () => {
      const refused = await firstRefusal(calls);
      if (refused !== null) setRefusal(refused);
      setPolls((it) => it + 1);
    });

  const act = (path: string, ...args: unknown[]) => actAll([[path, args]]);

  const draft = (field: string) => (typed: string) => setDrafts((previous) => new Map(previous).set(field, typed));
  const save = (field: string) => (typed: string) => act(MAVLINK_LOG_SET, field, typed);

  const log = read;
  if (log === null) return null;
  if (!log.px4) {
    return (
      <Text typography="bodyMedium" color={md('onSurfaceVariant')} className={`block px-4 ${className ?? ''}`}>
        {NO_PX4_VEHICLE}
      </Text>
    );
  }
  const editable = log.persistence;
  const saveItemsThen = (path: string, ...args: unknown[]) => actAll([...settingCalls(unsavedTexts(log.settings, drafts)), [path, args]]);
  const idle = logListIdle(log);
  const uploadedSelected = log.files.some((it) => selected.has(it.name) && it.uploaded);
  const text = (field: string) => optText(log.settings, field);
  const flag = (field: string) => optBoolean(log.settings, field);

  return (
    <>
      <div className={`flex w-full flex-col gap-2 overflow-auto px-4 ${className ?? ''}`}>
        <SectionHeader text="MAVLink 2.0 Logging" />
        <FootNote text="PX4 Pro only" />
        <div className="flex items-center gap-2">
          <Text className="flex-1">Logging</Text>
          <OutlinedButton enabled={!log.running && log.canStart && editable} onClick={() => act('mavlinkLog.start')}>
            Start
          </OutlinedButton>
          <OutlinedButton enabled={log.running && editable} onClick={() => act('mavlinkLog.stop')}>
            Stop
          </OutlinedButton>
        </div>
        <LogFlag label="Start logging automatically" checked={flag('enableAutoStart')} enabled={editable} onChange={(it) => act(MAVLINK_LOG_SET, 'enableAutoStart', it)} />

        <SectionHeader text="Log Upload" />
        <LogText label="Email Address" value={text('emailAddress')} enabled={editable} onType={draft('emailAddress')} onSave={save('emailAddress')} />
        <LogText label="Default Description" value={text('description')} enabled={editable} onType={draft('description')} onSave={save('description')} />
        <LogText label="Upload URL" value={text('uploadURL')} enabled={editable} onType={draft('uploadURL')} onSave={save('uploadURL')} />
        <LogText label="Video URL" value={text('videoURL')} enabled={editable} onType={draft('videoURL')} onSave={save('videoURL')} />
        <LogChoice label="Wind Speed" choices={WIND_SPEEDS} value={text('windSpeed')} enabled={editable} onPick={(it) => saveItemsThen(MAVLINK_LOG_SET, 'windSpeed', it)} />
        <LogChoice label="Flight Rating" choices={FLIGHT_RATINGS} value={text('rating')} enabled={editable} onPick={(it) => saveItemsThen(MAVLINK_LOG_SET, 'rating', it)} />
        <LogText label="Additional Feedback" value={text('feedback')} enabled={editable} onType={draft('feedback')} onSave={save('feedback')} />
        <LogFlag label="Make logs public" checked={flag('publicLog')} enabled={editable} onChange={(it) => act(MAVLINK_LOG_SET, 'publicLog', it)} />
        <LogFlag label="Upload logs automatically" checked={flag('enableAutoUpload')} enabled={editable} onChange={(it) => saveItemsThen(MAVLINK_LOG_SET, 'enableAutoUpload', it)} />
        <LogFlag label="Delete logs after upload" checked={flag('deleteAfterUpload')} enabled={editable && flag('enableAutoUpload')} onChange={(it) => act(MAVLINK_LOG_SET, 'deleteAfterUpload', it)} />

        <SectionHeader text="Saved Log Files" />
        {log.files.length === 0 ? <Text typography="bodySmall">No log files</Text> : null}
        {log.files.map((file) => {
          const uploadingThis = log.uploading && log.uploadingFile === file.name;
          return (
            <div key={file.name} className="flex items-center gap-3">
              <Checkbox
                checked={selected.has(file.name)}
                enabled={!uploadingThis && !file.writing}
                label={file.name}
                onCheckedChange={(it) => setSelected(it ? new Set([...selected, file.name]) : new Set([...selected].filter((name) => name !== file.name)))}
              />
              <Text typography="bodySmall" color={file.writing ? aircast('warning') : undefined} className="flex-1">
                {file.name}
              </Text>
              {uploadingThis && !file.uploaded ? (
                <LinearProgressIndicator progress={log.uploadProgress} className="w-24" />
              ) : (
                <Text typography="bodySmall" color={md('onSurfaceVariant')}>
                  {file.uploaded ? 'Uploaded' : logSizeText(file.size)}
                </Text>
              )}
            </div>
          );
        })}
        <div className="flex flex-wrap gap-1">
          <TextButton enabled={idle} onClick={() => setSelected(new Set(log.files.filter((it) => !it.writing).map((it) => it.name)))}>
            Select All
          </TextButton>
          <TextButton enabled={idle} onClick={() => setSelected(new Set())}>
            Select None
          </TextButton>
          <TextButton
            enabled={selected.size > 0 && idle}
            onClick={() =>
              setConfirming({
                title: 'Delete Selected Log Files',
                question: 'Delete the selected log files?',
                run: () => {
                  act('mavlinkLog.delete', ...selected);
                  setSelected(new Set());
                },
              })
            }
          >
            Delete…
          </TextButton>
          {log.uploading ? (
            <TextButton onClick={() => setConfirming({ title: 'Cancel Upload', question: 'Cancel the upload in progress?', run: () => act('mavlinkLog.cancelUpload') })}>Cancel Upload…</TextButton>
          ) : (
            <TextButton
              enabled={selected.size > 0 && idle && !uploadedSelected}
              onClick={() => {
                actAll(settingCalls(unsavedTexts(log.settings, drafts)));
                if (uploadEmail(log.settings, drafts).trim() === '') {
                  setRefusal('Please enter an email address before uploading MAVLink log files.');
                  return;
                }
                setConfirming({
                  title: 'Upload Selected Log Files',
                  question: 'Upload the selected log files?',
                  run: () => {
                    act('mavlinkLog.upload', ...selected);
                    setSelected(new Set());
                  },
                });
              }}
            >
              Upload…
            </TextButton>
          )}
        </div>
        {log.message.trim() !== '' ? (
          <Text typography="bodySmall" color={md('error')}>
            {log.message}
          </Text>
        ) : null}
      </div>
      {confirming !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(null)}
          title={confirming.title}
          text={confirming.question}
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(null);
                confirming.run();
              }}
            >
              Ok
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(null)}>Cancel</TextButton>}
        />
      ) : null}
      {refusal !== null ? (
        <AlertDialog onDismissRequest={() => setRefusal(null)} title={MAVLINK_LOGGING_TITLE} text={refusal} confirmButton={<TextButton onClick={() => setRefusal(null)}>Close</TextButton>} />
      ) : null}
    </>
  );
};

const LogFlag = ({ label, checked, enabled, onChange }: { label: string; checked: boolean; enabled: boolean; onChange: (checked: boolean) => void }) => (
  <div className="flex items-center gap-3">
    <Checkbox checked={checked} enabled={enabled} onCheckedChange={onChange} label={label} />
    <Text>{label}</Text>
  </div>
);

const LogText = ({ label, value, enabled, onType, onSave }: { label: string; value: string; enabled: boolean; onType: (typed: string) => void; onSave: (typed: string) => void }) => {
  const [typed, setTyped] = useKeyedState([value], () => value);
  useLaunchedEffect([typed], async (scope) => {
    onType(typed);
    if (typed === value) return;
    await delay(TEXT_SAVE_DELAY_MS);
    if (scope.isActive()) onSave(typed);
  });
  return <OutlinedTextField value={typed} onValueChange={setTyped} enabled={enabled} label={label} singleLine className="w-full" />;
};

const LogChoice = ({ label, choices, value, enabled, onPick }: { label: string; choices: readonly (readonly [string, string])[]; value: string; enabled: boolean; onPick: (raw: string) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="flex items-center">
      <Text className="flex-1">{label}</Text>
      <div className="relative">
        <OutlinedButton enabled={enabled} onClick={() => setOpen(true)}>
          {choices.find(([, raw]) => raw === value)?.[0] ?? choices[0]?.[0]}
        </OutlinedButton>
        <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
          {choices.map(([shown, raw]) => (
            <DropdownMenuItem
              key={raw}
              text={shown}
              onClick={() => {
                setOpen(false);
                onPick(raw);
              }}
            />
          ))}
        </DropdownMenu>
      </div>
    </div>
  );
};
