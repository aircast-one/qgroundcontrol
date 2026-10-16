import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, optString } from '@/bridge/Json';
import { useTicker } from '@/kotlin/Compose';
import { Text } from '@/m3';
import { Color, withAlpha } from '@/theme/Colors';
import { useFlyIsPortrait } from '@/ui/FlyShell';

export const VIDEO_STATS_VIEW = 'view.videoStats';
const VIDEO_STATS_POLL_MS = 1000;

export const videoStatsText = (view: Json | null): string => optString(view, 'text');

export const VideoStatsPill = ({ className }: { className?: string }) => {
  const [text, setText] = useState('');
  useTicker(VIDEO_STATS_POLL_MS, [], async () => setText(videoStatsText(await Qgc.get(VIDEO_STATS_VIEW))));
  const portrait = useFlyIsPortrait();
  if (text.trim() === '' || !portrait) return null;
  return (
    <div className={`pointer-events-none relative ${className ?? ''}`}>
      <Text
        typography="labelSmall"
        color={Color.White}
        fontWeight="bold"
        className="absolute start-2 bottom-2 rounded-full px-2.5 py-1"
        style={{ background: withAlpha(Color.Black, 0.6) }}
      >
        {text}
      </Text>
    </div>
  );
};
