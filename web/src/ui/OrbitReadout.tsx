import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Surface, Text } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { ORBIT_VIEW, orbitLabel, orbitReading } from './Orbit';
import { useOsdBackdrop } from './Osd';

export const OrbitReadout = ({ className }: { className?: string }) => {
  const view = useQgcPath(ORBIT_VIEW);
  const label = useMemo(() => orbitLabel(orbitReading(view)), [view]);
  const backdrop = useOsdBackdrop(withAlpha(md('secondaryContainer'), 0.92));
  if (label === null) return null;

  return (
    <Surface className={className} color={backdrop}>
      <Text typography="labelLarge" className="block px-2.5 py-1.5">
        {label}
      </Text>
    </Surface>
  );
};
