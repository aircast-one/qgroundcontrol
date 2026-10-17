import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { createDocument, openDocument } from '@/kotlin/Platform';
import { AlertDialog, Checkbox, LinearProgressIndicator, OutlinedButton, RadioButton, Text, TextButton } from '@/m3';
import type { OfflineSet } from '@/ui/OfflineMaps';

export const OFFLINE_EXPORT = 'offlineMaps.export';
export const OFFLINE_IMPORT = 'offlineMaps.import';
const TILESET_EXTENSION = 'qgctiledb';
const TILESET_MIME = 'application/octet-stream';
const EXPORT_STAGE = `tile-export.${TILESET_EXTENSION}`;
const IMPORT_STAGE = `tile-import.${TILESET_EXTENSION}`;
const STAGE_FOLDER = 'cache';
const UNWRITABLE = 'That file could not be written.';
const UNREADABLE = 'That file could not be read.';

type Transfer = { readonly name: 'Exporting' | 'Importing'; readonly ordinal: number; readonly label: string };

const Transfer = {
  Exporting: { name: 'Exporting', ordinal: 0, label: 'Exporting' },
  Importing: { name: 'Importing', ordinal: 1, label: 'Importing' },
} as const satisfies Record<string, Transfer>;

export const exportArguments = (path: string, chosen: ReadonlySet<number>): unknown[] => [path, ...[...chosen].toSorted((a, b) => a - b)];

const stagePath = (name: string): string => `/${STAGE_FOLDER}/${name}`;

const stageFolder = async (): Promise<FileSystemDirectoryHandle> => (await navigator.storage.getDirectory()).getDirectoryHandle(STAGE_FOLDER, { create: true });

const discard = async (name: string): Promise<void> => {
  try {
    await (await stageFolder()).removeEntry(name);
  } catch {
    return;
  }
};

const savedExport = async (): Promise<string | null> => {
  try {
    const staged = await (await (await stageFolder()).getFileHandle(EXPORT_STAGE)).getFile();
    createDocument(`export.${TILESET_EXTENSION}`, TILESET_MIME, await staged.arrayBuffer());
    return null;
  } catch {
    return UNWRITABLE;
  }
};

const stagedImport = async (source: File): Promise<boolean> => {
  try {
    const writable = await (await (await stageFolder()).getFileHandle(IMPORT_STAGE, { create: true })).createWritable();
    await writable.write(source);
    await writable.close();
    return true;
  } catch {
    return false;
  }
};

const ActionLine = ({ label, button, enabled, onClick }: { label: string; button: string; enabled: boolean; onClick: () => void }) => (
  <div className="flex w-full items-center">
    <Text className="flex-1">{label}</Text>
    <OutlinedButton enabled={enabled} onClick={onClick}>
      {button}
    </OutlinedButton>
  </div>
);

export const TileSetTransfer = ({ sets, onRefusal }: { sets: readonly OfflineSet[]; onRefusal: (refusal: string | null) => void }) => {
  const [running, setRunning] = useState<Transfer | null>(null);
  const [choosingSets, setChoosingSets] = useState(false);
  const [choosingMode, setChoosingMode] = useState(false);
  const [chosen, setChosen] = useState<ReadonlySet<number>>(() => new Set());
  const [replace, setReplace] = useState(false);

  const finish = (refusal: string | null) => {
    setRunning(null);
    onRefusal(refusal);
  };

  const exportSets = async () => {
    setRunning(Transfer.Exporting);
    const refused = await Qgc.refusalOf(OFFLINE_EXPORT, ...exportArguments(stagePath(EXPORT_STAGE), chosen));
    const refusal = refused ?? (await savedExport());
    await discard(EXPORT_STAGE);
    finish(refusal);
  };

  const importSets = async () => {
    const source = await openDocument(['*/*']);
    if (source === null) return;
    setRunning(Transfer.Importing);
    const copied = await stagedImport(source);
    const refusal = copied ? await Qgc.refusalOf(OFFLINE_IMPORT, stagePath(IMPORT_STAGE), replace) : UNREADABLE;
    await discard(IMPORT_STAGE);
    finish(refusal);
  };

  return (
    <>
      <div className="flex flex-col gap-1">
        <ActionLine label="Import map tiles" button="Import…" enabled={running === null} onClick={() => setChoosingMode(true)} />
        <ActionLine
          label="Export map tiles"
          button="Export…"
          enabled={running === null}
          onClick={() => {
            setChosen(new Set());
            setChoosingSets(true);
          }}
        />
        {running === null ? null : (
          <>
            <Text>{running.label}</Text>
            <LinearProgressIndicator className="w-full" label={running.label} />
          </>
        )}
      </div>

      {choosingSets ? (
        <AlertDialog
          onDismissRequest={() => setChoosingSets(false)}
          title="Export selected tile sets"
          text={
            <div className="flex flex-col">
              {sets.map((set) => (
                <div key={set.id} className="flex min-h-12 items-center gap-3">
                  <Checkbox
                    checked={chosen.has(set.id)}
                    onCheckedChange={(it) => setChosen((held) => new Set(it ? [...held, set.id] : [...held].filter((id) => id !== set.id)))}
                    label={set.name}
                  />
                  <Text>{set.name}</Text>
                </div>
              ))}
            </div>
          }
          confirmButton={
            <TextButton
              enabled={chosen.size > 0}
              onClick={() => {
                setChoosingSets(false);
                void exportSets();
              }}
            >
              Ok
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setChoosingSets(false)}>Cancel</TextButton>}
        />
      ) : null}

      {choosingMode ? (
        <AlertDialog
          onDismissRequest={() => setChoosingMode(false)}
          title="Import tile sets"
          text={
            <div className="flex flex-col" role="radiogroup">
              {(
                [
                  [false, 'Append to existing sets'],
                  [true, 'Replace existing sets'],
                ] as const
              ).map(([mode, label]) => (
                <div key={label} className="flex min-h-12 items-center gap-3">
                  <RadioButton selected={replace === mode} onClick={() => setReplace(mode)} label={label} />
                  <Text>{label}</Text>
                </div>
              ))}
            </div>
          }
          confirmButton={
            <TextButton
              onClick={() => {
                setChoosingMode(false);
                void importSets();
              }}
            >
              Ok
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setChoosingMode(false)}>Cancel</TextButton>}
        />
      ) : null}
    </>
  );
};
