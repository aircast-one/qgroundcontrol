import type { CSSProperties } from 'react';
import { type DrawableName, drawables } from './drawables';

type IconProps = { size?: number; label?: string | null; className?: string; style?: CSSProperties };

export const Drawable = ({ name, size = 24, label = null, className, style }: IconProps & { name: DrawableName }) => {
  const data = drawables[name];
  return (
    <svg viewBox={data.viewBox} width={size} height={size} className={className} style={style} role={label ? 'img' : undefined} aria-label={label ?? undefined} aria-hidden={label ? undefined : true}>
      {data.paths.map((path, index) => (
        <path key={index} {...path} />
      ))}
    </svg>
  );
};

export const MaterialIcon = ({ name, size = 24, label = null, filled = false, className, style }: IconProps & { name: string; filled?: boolean }) => (
  <span
    className={`material-symbols-rounded select-none leading-none ${className ?? ''}`}
    style={{ fontSize: size, width: size, height: size, fontVariationSettings: `'FILL' ${filled ? 1 : 0}`, ...style }}
    role={label ? 'img' : undefined}
    aria-label={label ?? undefined}
    aria-hidden={label ? undefined : true}
  >
    {name}
  </span>
);
