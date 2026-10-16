import { useQgcPath } from '@/bridge/QgcState';
import { Surface, Text } from '@/m3';
import { Color, md } from '@/theme/Colors';
import { obstacleWarning } from './ObstacleDistance';
import { useOsdBackdrop } from './Osd';

const OBSTACLE = 'view.obstacle';

export const ObstacleReadout = ({ className }: { className?: string }) => {
  const view = useQgcPath(OBSTACLE);
  const warning = obstacleWarning(view);
  const close = warning?.close === true;
  const backdrop = useOsdBackdrop(close ? md('errorContainer') : md('surfaceVariant'));
  if (warning === null) return null;
  const content = backdrop === Color.Transparent ? undefined : close ? md('onErrorContainer') : md('onSurfaceVariant');

  return (
    <Surface className={className} color={backdrop} contentColor={content}>
      <Text typography="labelLarge" className="block px-2.5 py-1.5">
        {warning.label}
      </Text>
    </Surface>
  );
};
