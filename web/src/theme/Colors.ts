export type MdRole =
  | 'primary' | 'onPrimary' | 'primaryContainer' | 'onPrimaryContainer'
  | 'secondary' | 'onSecondary' | 'secondaryContainer' | 'onSecondaryContainer'
  | 'tertiary' | 'onTertiary' | 'tertiaryContainer'
  | 'error' | 'onError' | 'errorContainer' | 'onErrorContainer'
  | 'background' | 'onBackground' | 'surface' | 'onSurface' | 'surfaceVariant' | 'onSurfaceVariant'
  | 'outline' | 'outlineVariant' | 'surfaceDim' | 'surfaceBright'
  | 'surfaceContainerLowest' | 'surfaceContainerLow' | 'surfaceContainer' | 'surfaceContainerHigh' | 'surfaceContainerHighest'
  | 'inverseSurface' | 'inverseOnSurface' | 'scrim';

export type AircastRole =
  | 'success' | 'onSuccess' | 'successContainer' | 'warning' | 'onWarning' | 'warningContainer'
  | 'mission' | 'alert' | 'mapLand' | 'mapWater' | 'mapRoad'
  | 'outdoorBackground' | 'outdoorForeground' | 'outdoorAccent' | 'onOutdoorAccent';

const kebab = (name: string) => name.replace(/[A-Z]/g, (letter) => `-${letter.toLowerCase()}`);

const STATUS: ReadonlySet<string> = new Set(['success', 'onSuccess', 'successContainer', 'warning', 'onWarning', 'warningContainer']);

export const md = (role: MdRole): string => `var(--md-${kebab(role)})`;

export const aircast = (role: AircastRole): string => (STATUS.has(role) ? `var(--md-${kebab(role)})` : `var(--aircast-${kebab(role)})`);

export const withAlpha = (color: string, alpha: number): string => `color-mix(in srgb, ${color} ${Math.round(alpha * 100)}%, transparent)`;

export const Color = {
  Black: '#000000',
  White: '#ffffff',
  Transparent: 'transparent',
  Red: '#ff0000',
  Green: '#00ff00',
  Yellow: '#ffff00',
  Gray: '#888888',
  LightGray: '#cccccc',
  DarkGray: '#444444',
  Unspecified: 'currentColor',
} as const;

export const argb = (value: number): string => {
  const alpha = ((value >>> 24) & 0xff) / 255;
  const rgb = (value & 0xffffff).toString(16).padStart(6, '0');
  return alpha >= 1 ? `#${rgb}` : withAlpha(`#${rgb}`, alpha);
};

export const AircastSpace = { s1: 4, s2: 8, s3: 12, s4: 16, s5: 20, s6: 24, s8: 32 } as const;

export const AircastShapes = { extraSmall: 4, small: 8, medium: 12, large: 16, extraLarge: 28 } as const;
