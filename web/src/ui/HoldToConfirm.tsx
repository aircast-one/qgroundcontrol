import { Text } from '@/m3';
import { md, withAlpha } from '@/theme/Colors';
import { HoldFill, useRememberHold } from '@/ui/FlyDeck';

const DISABLED_HOLD_ALPHA = 0.38;

const TRACK_HEIGHT = 64;

const HOLD_TRACK_FILL_ALPHA = 0.45;

export const holdLabel = (name: string): string => (name.trim() === '' ? 'Slide to confirm' : `Slide to ${name.toLowerCase()}`);

export const HoldToConfirm = ({
  label,
  className,
  destructive = false,
  enabled = true,
  onConfirm,
}: {
  label: string;
  className?: string;
  destructive?: boolean;
  enabled?: boolean;
  onConfirm: () => void;
}) => {
  const accent = destructive ? md('error') : md('primary');
  const track = destructive ? md('errorContainer') : md('primaryContainer');
  const onTrack = destructive ? md('onErrorContainer') : md('onPrimaryContainer');
  const [progress, gesture] = useRememberHold(label, enabled, () => {}, onConfirm, label);
  return (
    <div
      {...gesture}
      className={`relative flex w-full touch-none select-none items-center justify-center overflow-hidden rounded-full ${className ?? ''}`}
      style={{ ...gesture.style, height: TRACK_HEIGHT, background: track, opacity: enabled ? 1 : DISABLED_HOLD_ALPHA }}
    >
      <HoldFill progress={progress} color={withAlpha(accent, HOLD_TRACK_FILL_ALPHA)} className="absolute inset-y-0 left-0" />
      <Text typography="labelLarge" color={onTrack} className="relative">
        {label}
      </Text>
    </div>
  );
};
