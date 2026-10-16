import { md } from '@/theme/Colors';

export const LinearProgressIndicator = ({ progress, color = md('primary'), trackColor = md('secondaryContainer'), className, label }: { progress?: number; color?: string; trackColor?: string; className?: string; label?: string }) => (
  <div
    role="progressbar"
    aria-label={label}
    aria-valuemin={0}
    aria-valuemax={100}
    aria-valuenow={progress === undefined ? undefined : Math.round(progress * 100)}
    className={`relative h-1 w-full overflow-hidden rounded-full ${className ?? ''}`}
    style={{ background: trackColor }}
  >
    <div
      className={`absolute inset-y-0 left-0 rounded-full ${progress === undefined ? 'w-1/3 animate-[m3-indeterminate_1.4s_ease-in-out_infinite]' : 'transition-transform'}`}
      style={progress === undefined ? { background: color } : { background: color, width: '100%', transformOrigin: 'left', transform: `scaleX(${Math.max(0, Math.min(1, progress))})` }}
    />
    <style>{'@keyframes m3-indeterminate { from { transform: translateX(-100%) } to { transform: translateX(300%) } }'}</style>
  </div>
);

export const CircularProgressIndicator = ({ progress, size = 40, strokeWidth = 4, color = md('primary'), trackColor = 'transparent', label }: { progress?: number; size?: number; strokeWidth?: number; color?: string; trackColor?: string; label?: string }) => {
  const radius = (size - strokeWidth) / 2;
  const length = 2 * Math.PI * radius;
  return (
    <svg width={size} height={size} viewBox={`0 0 ${size} ${size}`} role="progressbar" aria-label={label} aria-valuenow={progress === undefined ? undefined : Math.round(progress * 100)} className={progress === undefined ? 'animate-spin' : undefined}>
      <circle cx={size / 2} cy={size / 2} r={radius} fill="none" stroke={trackColor} strokeWidth={strokeWidth} />
      <circle
        cx={size / 2}
        cy={size / 2}
        r={radius}
        fill="none"
        stroke={color}
        strokeWidth={strokeWidth}
        strokeLinecap="round"
        strokeDasharray={`${(progress === undefined ? 0.25 : Math.max(0, Math.min(1, progress))) * length} ${length}`}
        transform={`rotate(-90 ${size / 2} ${size / 2})`}
      />
    </svg>
  );
};
