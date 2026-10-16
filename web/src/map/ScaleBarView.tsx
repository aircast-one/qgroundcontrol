import type { CSSProperties } from 'react';
import type { Json } from '@/bridge/Json';
import { useProduceState } from '@/kotlin/Compose';
import { Text } from '@/m3';
import { Color, withAlpha } from '@/theme/Colors';
import { MapBridge } from './MapBridge';
import { mapScaleBar, metresAcross } from './MapScale';

const MAX_BAR_DP = 100;

export const ScaleBarView = ({ latitude, zoom, className, style }: { latitude: number; zoom: number; className?: string; style?: CSSProperties }) => {
  const maxPixels = MAX_BAR_DP;
  const across = metresAcross(latitude, zoom, MAX_BAR_DP);
  const view = useProduceState<Json | null>(null, [across], async (set) => {
    set(across === null ? null : await MapBridge.read(`view.mapScale(${across})`));
  });
  const scale = mapScaleBar(view, maxPixels);
  if (scale === null) return null;
  return (
    <div className={`flex flex-col ${className ?? ''}`} style={style}>
      <Text typography="labelSmall" color={Color.White} className="pb-0.5">
        {scale.text}
      </Text>
      <div className="py-0.5" style={{ width: scale.pixels, background: withAlpha(Color.White, 0.9) }} />
    </div>
  );
};
