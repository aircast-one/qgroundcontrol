import type { CSSProperties, ReactNode } from 'react';

export type Typography =
  | 'displayLarge' | 'displayMedium' | 'displaySmall'
  | 'headlineLarge' | 'headlineMedium' | 'headlineSmall'
  | 'titleLarge' | 'titleMedium' | 'titleSmall'
  | 'bodyLarge' | 'bodyMedium' | 'bodySmall'
  | 'labelLarge' | 'labelMedium' | 'labelSmall'
  | 'telemetry' | 'telemetryMedium';

const kebab = (name: string) => name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

export const typographyClass = (typography: Typography): string => `text-${kebab(typography)}`;

export type TextProps = {
  children?: ReactNode;
  typography?: Typography;
  color?: string;
  fontWeight?: CSSProperties['fontWeight'];
  fontSize?: number;
  textAlign?: CSSProperties['textAlign'];
  maxLines?: number;
  ellipsis?: boolean;
  italic?: boolean;
  monospace?: boolean;
  className?: string;
  style?: CSSProperties;
  id?: string;
};

export const Text = ({ children, typography = 'bodyLarge', color, fontWeight, fontSize, textAlign, maxLines, ellipsis = false, italic = false, monospace = false, className, style, id }: TextProps) => {
  const clamp: CSSProperties = maxLines === undefined
    ? {}
    : maxLines === 1
      ? { whiteSpace: 'nowrap', overflow: 'hidden', textOverflow: ellipsis ? 'ellipsis' : 'clip' }
      : { display: '-webkit-box', WebkitLineClamp: maxLines, WebkitBoxOrient: 'vertical', overflow: 'hidden' };
  return (
    <span
      id={id}
      className={`${typographyClass(typography)} ${className ?? ''}`}
      style={{ color, fontWeight, fontSize, textAlign, fontStyle: italic ? 'italic' : undefined, fontFamily: monospace ? 'ui-monospace, monospace' : undefined, display: textAlign ? 'block' : undefined, ...clamp, ...style }}
    >
      {children}
    </span>
  );
};
