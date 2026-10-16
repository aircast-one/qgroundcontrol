import type { CSSProperties, ReactNode } from 'react';
import { md } from '@/theme/Colors';

export type Shape = 'none' | 'extraSmall' | 'small' | 'medium' | 'large' | 'extraLarge' | 'full' | number;

export const radius = (shape: Shape): number | string => {
  if (typeof shape === 'number') return shape;
  return { none: 0, extraSmall: 4, small: 8, medium: 12, large: 16, extraLarge: 28, full: 9999 }[shape];
};

export type SurfaceProps = {
  children?: ReactNode;
  color?: string;
  contentColor?: string;
  shape?: Shape;
  border?: string;
  shadow?: boolean;
  onClick?: () => void;
  enabled?: boolean;
  className?: string;
  style?: CSSProperties;
  label?: string;
  title?: string;
};

export const Surface = ({ children, color = md('surface'), contentColor, shape = 'none', border, shadow = false, onClick, enabled = true, className, style, label, title }: SurfaceProps) => {
  const look: CSSProperties = {
    background: color,
    color: contentColor,
    borderRadius: radius(shape),
    border,
    boxShadow: shadow ? '0 2px 6px rgb(0 0 0 / 0.3)' : undefined,
    ...style,
  };
  return onClick ? (
    <button type="button" className={`text-left disabled:opacity-40 ${className ?? ''}`} style={look} onClick={onClick} disabled={!enabled} aria-label={label} title={title}>
      {children}
    </button>
  ) : (
    <div className={className} style={look} aria-label={label} title={title}>
      {children}
    </div>
  );
};

export const Card = (props: SurfaceProps) => <Surface color={md('surfaceContainerHighest')} contentColor={md('onSurface')} shape="medium" {...props} />;

export const ElevatedCard = (props: SurfaceProps) => <Surface color={md('surfaceContainerLow')} contentColor={md('onSurface')} shape="medium" shadow {...props} />;

export const OutlinedCard = (props: SurfaceProps) => <Surface color={md('surface')} contentColor={md('onSurface')} shape="medium" border={`1px solid ${md('outlineVariant')}`} {...props} />;
