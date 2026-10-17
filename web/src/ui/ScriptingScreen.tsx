import { useState } from 'react';
import { type Json, jsonStrings, optBoolean, optDouble, optJSONArray, optJSONObject, optText } from '@/bridge/Json';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { createDocument, openDocument } from '@/kotlin/Platform';
import { delay } from '@/kotlin/Std';
import { AlertDialog, LinearProgressIndicator, OutlinedButton, Text, TextButton } from '@/m3';
import { factFromControl } from '@/ui/ParameterForm';
import { FactRow } from '@/ui/SettingsScreen';

export const SCRIPTING_VIEW = 'view.scripting';
export const SCRIPTING_SCREEN = 'scripting';
export const SCRIPTING_OPEN = 'scripting.open';
export const SCRIPTING_UPLOAD = 'scripting.upload';
export const SCRIPTING_DOWNLOAD = 'scripting.download';
export const SCRIPTING_DELETE = 'scripting.delete';
export const SCRIPTING_CANCEL = 'scripting.cancel';
const BUSY_POLL_MS = 300;
const IDLE_POLL_MS = 2000;
const STAGE_FOLDER = 'cache';
const LUA_MIME = 'text/x-lua';

export type Scripting = {
  available: boolean;
  enabled: boolean;
  enable: Fact | null;
  scripts: readonly string[];
  busy: boolean;
  progress: number;
  status: string;
  unsupportedText: string;
};

export type ScriptRefusal = { title: string; text: string };

export const scriptRefusal = (title: string, reason: string | null, fallback: string): ScriptRefusal | null =>
  reason === null ? null : { title, text: reason.trim() === '' ? fallback : reason };

export const scripting = (view: Json | null): Scripting | null => {
  if (view === null) return null;
  const enable = optJSONObject(view, 'enable');
  return {
    available: optBoolean(view, 'available'),
    enabled: optBoolean(view, 'enabled'),
    enable: enable === null ? null : factFromControl(enable),
    scripts: jsonStrings(optJSONArray(view, 'scripts')),
    busy: optBoolean(view, 'busy'),
    progress: optDouble(view, 'progress', 0),
    status: optText(view, 'status'),
    unsupportedText: optText(view, 'unsupportedText'),
  };
};

const stagePath = (name: string): string => `/${STAGE_FOLDER}/${name}`;

const stageFolder = async (): Promise<FileSystemDirectoryHandle> => (await navigator.storage.getDirectory()).getDirectoryHandle(STAGE_FOLDER, { create: true });

const stagedCopy = async (source: File, name: string): Promise<boolean> => {
  try {
    const writable = await (await (await stageFolder()).getFileHandle(name, { create: true })).createWritable();
    await writable.write(source);
    await writable.close();
    return true;
  } catch {
    return false;
  }
};

const staged = async (name: string): Promise<File | null> => {
  try {
    return await (await (await stageFolder()).getFileHandle(name)).getFile();
  } catch {
    return null;
  }
};

const discard = async (name: string): Promise<void> => {
  try {
    await (await stageFolder()).removeEntry(name);
  } catch {
    return;
  }
};

const whileBusy = async (): Promise<void> => {
  if (scripting(await Qgc.get(SCRIPTING_VIEW))?.busy !== true) return;
  await delay(BUSY_POLL_MS);
  return whileBusy();
};

export const ScriptingScreen = ({ className }: { className?: string }) => {
  const [revision, setRevision] = useState(0);
  const [read, setRead] = useState<Scripting | null>(null);
  const [refusal, setRefusal] = useState<ScriptRefusal | null>(null);
  const [confirmDelete, setConfirmDelete] = useState<string | null>(null);
  const bump = () => setRevision((it) => it + 1);

  useLaunchedEffect([], async (scope) => {
    await Qgc.invoke(SCRIPTING_OPEN);
    if (scope.isActive()) bump();
  });

  useLaunchedEffect([revision], async (scope) => {
    const next = scripting(await Qgc.get(SCRIPTING_VIEW));
    if (!scope.isActive()) return;
    setRead(next);
    await delay(next?.busy === true ? BUSY_POLL_MS : IDLE_POLL_MS);
    if (scope.isActive()) bump();
  });

  const act = async (path: string, ...args: unknown[]) => {
    await Qgc.refusalOf(path, ...args);
    bump();
  };

  const upload = async () => {
    const chosen = await openDocument(['*/*']);
    if (chosen === null) return;
    const name = chosen.name.replaceAll('/', '_');
    const stage = `upload-${name}`;
    const copied = await stagedCopy(chosen, stage);
    setRefusal(scriptRefusal('Lua Upload', copied ? await Qgc.refusalOf(SCRIPTING_UPLOAD, stagePath(stage), name) : `File ${name} does not exist`, 'Upload failed'));
    bump();
  };

  const download = async (name: string) => {
    const stage = `download-${name}`;
    await discard(stage);
    const refused = scriptRefusal('Lua Download', await Qgc.refusalOf(SCRIPTING_DOWNLOAD, name, stagePath(stage), name), 'Download failed');
    setRefusal(refused);
    if (refused === null) await whileBusy();
    const saved = refused === null ? await staged(stage) : null;
    if (saved !== null) createDocument(name, LUA_MIME, saved);
    await discard(stage);
    bump();
  };

  const remove = async (name: string) => {
    setRefusal(scriptRefusal('Lua Delete', await Qgc.refusalOf(SCRIPTING_DELETE, name), 'Delete failed'));
    bump();
  };

  const page = read;
  if (page === null) return null;
  if (!page.available) return <Text className={`p-4 ${className ?? ''}`}>{page.unsupportedText}</Text>;
  return (
    <>
      <div className={`flex w-full flex-col gap-1 overflow-y-auto ${className ?? ''}`}>
        {page.enable !== null ? <FactRow fact={page.enable} title="Enable scripting" onWrite={bump} /> : null}
        <div className="flex flex-col items-start gap-1.5 px-5">
          {page.status.trim() !== '' ? <Text typography="bodyMedium">{page.status}</Text> : null}
          <OutlinedButton enabled={page.enabled && !page.busy} onClick={() => void upload()}>
            Upload
          </OutlinedButton>
          {page.scripts.map((name) => (
            <div key={name} className="flex w-full items-center">
              <Text typography="bodyLarge" className="flex-1">
                {name}
              </Text>
              <TextButton enabled={page.enabled && !page.busy} onClick={() => void download(name)}>
                Download
              </TextButton>
              <TextButton enabled={page.enabled && !page.busy} onClick={() => setConfirmDelete(name)}>
                Delete
              </TextButton>
            </div>
          ))}
          {page.busy ? (
            <>
              <div className="flex items-center gap-3">
                <TextButton onClick={() => void act(SCRIPTING_CANCEL)}>Cancel operation</TextButton>
                <Text>{`Transferring... ${Math.round(page.progress * 100)}%`}</Text>
              </div>
              <LinearProgressIndicator progress={page.progress} className="w-full" />
            </>
          ) : null}
        </div>
      </div>
      {confirmDelete !== null ? (
        <AlertDialog
          onDismissRequest={() => setConfirmDelete(null)}
          title="Delete Lua script"
          text={`Are you sure you want to delete the script "${confirmDelete}"? This action cannot be undone.`}
          confirmButton={
            <TextButton
              onClick={() => {
                const name = confirmDelete;
                setConfirmDelete(null);
                void remove(name);
              }}
            >
              OK
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirmDelete(null)}>Cancel</TextButton>}
        />
      ) : null}
      {refusal !== null ? (
        <AlertDialog onDismissRequest={() => setRefusal(null)} title={refusal.title} text={refusal.text} confirmButton={<TextButton onClick={() => setRefusal(null)}>OK</TextButton>} />
      ) : null}
    </>
  );
};
