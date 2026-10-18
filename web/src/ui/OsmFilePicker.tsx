import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { stageFile, unstageAll } from '@/core/files';
import { settingControl, useQgcBool } from '@/bridge/QgcState';
import { openDocument } from '@/kotlin/Platform';
import { OutlinedButton, Text } from '@/m3';
import { md } from '@/theme/Colors';

const OSM_FILE_SETTING = 'settings.viewer3DSettings.osmFilePath';
const OSM_FOLDER = 'osm';
const OSM_ENABLED_SETTING = 'settings.viewer3DSettings.enabled';

export const osmFileAccepted = (name: string): boolean => {
  const lower = name.toLowerCase();
  return lower.endsWith('.osm') || lower.endsWith('.xml');
};

type Staged = { readonly kind: 'Copied'; readonly path: string } | { readonly kind: 'NotOsm' } | { readonly kind: 'Unreadable' };

const stagedPath = (name: string): string => `/${OSM_FOLDER}/${name}`;

const stagedOsm = async (file: File): Promise<Staged> => {
  const shown = file.name.replace(/[/\\]/g, '_');
  if (!osmFileAccepted(shown)) return { kind: 'NotOsm' };
  const copied = await stageFile(stagedPath(shown), file).catch(() => false);
  return copied ? { kind: 'Copied', path: stagedPath(shown) } : { kind: 'Unreadable' };
};

const keepOnly = (path: string): Promise<void> => unstageAll(`/${OSM_FOLDER}/`, path).catch(() => undefined);

const refusalOf = async (staged: Staged): Promise<string | null> => {
  switch (staged.kind) {
    case 'Copied': {
      const refused = await Qgc.writeRefusal(OSM_FILE_SETTING, staged.path);
      if (refused === null) await keepOnly(staged.path);
      return refused;
    }
    case 'NotOsm':
      return 'Choose an OpenStreetMap file (.osm or .xml).';
    case 'Unreadable':
      return 'That file could not be read.';
  }
};

export const OsmFilePicker = ({ onWrite }: { onWrite: () => void }) => {
  const [refusal, setRefusal] = useState<string | null>(null);
  const enabled = useQgcBool(settingControl(OSM_ENABLED_SETTING));
  const pick = async () => {
    const chosen = await openDocument(['*/*']);
    if (chosen === null) return;
    setRefusal(await refusalOf(await stagedOsm(chosen)));
    onWrite();
  };
  return (
    <div className="flex flex-col px-4 py-2">
      <OutlinedButton enabled={enabled} onClick={() => void pick()} className="self-start">
        Choose OpenStreetMap file
      </OutlinedButton>
      {refusal === null ? null : (
        <Text typography="bodySmall" color={md('error')}>
          {refusal}
        </Text>
      )}
    </div>
  );
};
