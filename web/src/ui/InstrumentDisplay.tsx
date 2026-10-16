import type { ReactNode } from 'react';

export type RangeType = { readonly name: 'None' | 'Color' | 'Opacity' | 'Icon'; readonly ordinal: number; readonly label: string };

export const RangeType = {
  None: { name: 'None', ordinal: 0, label: 'None' },
  Color: { name: 'Color', ordinal: 1, label: 'Color' },
  Opacity: { name: 'Opacity', ordinal: 2, label: 'Opacity' },
  Icon: { name: 'Icon', ordinal: 3, label: 'Icon' },
} as const satisfies Record<string, RangeType>;

export type ValueDisplay = {
  text: string;
  showUnits: boolean;
  showIcon: boolean;
  icon: string;
  rangeType: RangeType;
  values: readonly number[];
  colours: readonly number[];
  opacities: readonly number[];
  icons: readonly string[];
};

export const ValueDisplay = (fields: Partial<ValueDisplay> = {}): ValueDisplay => ({
  text: '',
  showUnits: true,
  showIcon: false,
  icon: '',
  rangeType: RangeType.None,
  values: [],
  colours: [],
  opacities: [],
  icons: [],
  ...fields,
});

export const displayColour = (_display: ValueDisplay, _raw: number | null): number | null => null;

export const readDisplays = (_vehicleClass: string): ReadonlyMap<string, ValueDisplay> => new Map();

export const writeDisplay = (_vehicleClass: string, _id: string, _display: ValueDisplay): void => undefined;

export const ValueDisplayDialog = (_props: { label: string; initial: ValueDisplay; onDismiss: () => void; extra?: ReactNode; onDone: (display: ValueDisplay) => void }) => null;

export const ValueLabel = (_props: { display: ValueDisplay; raw: number | null; label: string; fallback: string }) => null;
