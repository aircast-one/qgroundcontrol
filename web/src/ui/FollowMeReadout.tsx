import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Surface, Text } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { FOLLOW_ME_VIEW, followMeLabel, followMeReading } from './FollowMe';
import { useOsdBackdrop } from './Osd';

export const FollowMeReadout = ({ className }: { className?: string }) => {
  const view = useQgcPath(FOLLOW_ME_VIEW);
  const reading = useMemo(() => followMeReading(view), [view]);
  const label = useMemo(() => followMeLabel(reading), [reading]);
  const sending = reading?.wouldSend === true;
  const backdrop = useOsdBackdrop(sending ? withAlpha(md('secondaryContainer'), 0.92) : withAlpha(md('surfaceVariant'), 0.92));
  if (label === null) return null;

  return (
    <Surface className={className} color={backdrop}>
      <Text typography="labelLarge" maxLines={2} className="block max-w-[300px] px-2.5 py-1.5">
        {label}
      </Text>
    </Surface>
  );
};
