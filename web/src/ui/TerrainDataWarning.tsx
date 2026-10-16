import { useState } from 'react';
import { type Json, optLong } from '@/bridge/Json';
import { useQgcDerived } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { IconButton, Surface, Text } from '@/m3';
import { Drawable } from '@/res/Icon';
import { aircast, md } from '@/theme/Colors';

export const TERRAIN_DOWNLOAD = 'view.terrainDownload';
export const TERRAIN_STALL_MS = 30_000;
export const TERRAIN_STALLED_TITLE = "Terrain data isn't loading";
export const TERRAIN_STALLED_DETAIL = 'Terrain following may not work. It needs internet and terrain coverage for this area.';
const TERRAIN_DATA_WARNING_MAX_WIDTH = 420;
const TERRAIN_DATA_WARNING_CORNER = 12;

export type TerrainLoad = { readonly loaded: number; readonly pending: number; readonly waiting: boolean };

export const TerrainLoad = (loaded: number, pending: number): TerrainLoad => ({ loaded, pending, waiting: pending > 0 });

export const terrainLoad = (view: Json | null): TerrainLoad => TerrainLoad(optLong(view, 'loaded'), optLong(view, 'pending'));

export const TerrainDataWarning = ({ className }: { className?: string }) => {
  const load = useQgcDerived(TERRAIN_DOWNLOAD, terrainLoad);
  const [stalled, setStalled] = useState(false);
  const [dismissed, setDismissed] = useState(false);
  useLaunchedEffect([load.waiting, load.loaded], async (scope) => {
    setStalled(false);
    if (!load.waiting) setDismissed(false);
    if (load.waiting) {
      await delay(TERRAIN_STALL_MS);
      if (scope.isActive()) setStalled(true);
    }
  });
  if (!stalled || dismissed) return null;
  return (
    <Surface className={className} style={{ maxWidth: TERRAIN_DATA_WARNING_MAX_WIDTH }} shape={TERRAIN_DATA_WARNING_CORNER} color={aircast('warningContainer')} contentColor={md('onSurface')}>
      <div className="flex items-center gap-3 py-2 pr-1 pl-3">
        <Drawable name="ic_warning" size={24} style={{ color: aircast('warning') }} className="shrink-0" />
        <div className="flex min-w-0 flex-col">
          <Text typography="titleSmall">{TERRAIN_STALLED_TITLE}</Text>
          <Text typography="bodySmall">{TERRAIN_STALLED_DETAIL}</Text>
        </div>
        <IconButton onClick={() => setDismissed(true)} label="Dismiss" className="shrink-0">
          <Drawable name="ic_close" />
        </IconButton>
      </div>
    </Surface>
  );
};
