import { type CSSProperties, type ReactNode, useMemo, useState } from 'react';
import { type Json, optArrayText, optBoolean, optJSONArray, optString, parseJson } from '@/bridge/Json';
import { getSharedPreferences } from '@/kotlin/Platform';
import { coerceIn } from '@/kotlin/Std';
import { AlertDialog, FilterChip, OutlinedTextField, Switch, Text, TextButton } from '@/m3';
import { toDoubleOrNull } from '@/map/AltitudeEntry';
import { argb, md } from '@/theme/Colors';

const DISPLAY_STORE = 'fly-instrument-display';
const DEFAULT_RANGE_LOW = 0.0;
const DEFAULT_RANGE_HIGH = 100.0;
const GREEN = 0xff008000;
export const NO_COLOUR = 0;
export const RANGE_COLOURS: readonly number[] = [GREEN, 0xffffff00, 0xffffa500, 0xffff0000, 0xff0000ff, 0xffffffff, NO_COLOUR];

const ICON_URLS = import.meta.glob<string>('../../../resources/InstrumentValueIcons/*.svg', { query: '?url', import: 'default', eager: true });

export type RangeType = { readonly name: 'None' | 'Color' | 'Opacity' | 'Icon'; readonly ordinal: number; readonly label: string };

export const RangeType = {
  None: { name: 'None', ordinal: 0, label: 'None' },
  Color: { name: 'Color', ordinal: 1, label: 'Color' },
  Opacity: { name: 'Opacity', ordinal: 2, label: 'Opacity' },
  Icon: { name: 'Icon', ordinal: 3, label: 'Icon' },
} as const satisfies Record<string, RangeType>;

export const RangeTypeEntries: readonly RangeType[] = [RangeType.None, RangeType.Color, RangeType.Opacity, RangeType.Icon];

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

export const withRangeType = (display: ValueDisplay, type: RangeType, firstIcon: string): ValueDisplay => {
  const values = type === RangeType.None ? [] : [DEFAULT_RANGE_LOW, DEFAULT_RANGE_HIGH];
  const slots = type === RangeType.None ? 0 : values.length + 1;
  return {
    ...display,
    rangeType: type,
    values,
    colours: type === RangeType.Color ? Array.from({ length: slots }, () => GREEN) : [],
    opacities: type === RangeType.Opacity ? Array.from({ length: slots }, () => 1.0) : [],
    icons: type === RangeType.Icon ? Array.from({ length: slots }, () => firstIcon) : [],
  };
};

export const withRow = (display: ValueDisplay, firstIcon: string): ValueDisplay => ({
  ...display,
  values: [...display.values, (display.values.at(-1) ?? DEFAULT_RANGE_LOW) + 1],
  colours: display.rangeType === RangeType.Color ? [...display.colours, GREEN] : display.colours,
  opacities: display.rangeType === RangeType.Opacity ? [...display.opacities, 1.0] : display.opacities,
  icons: display.rangeType === RangeType.Icon ? [...display.icons, firstIcon] : display.icons,
});

export const withoutRow = (display: ValueDisplay, index: number): ValueDisplay => ({
  ...display,
  values: display.values.filter((_, i) => i !== index),
  colours: display.colours.filter((_, i) => i !== index + 1),
  opacities: display.opacities.filter((_, i) => i !== index + 1),
  icons: display.icons.filter((_, i) => i !== index + 1),
});

export const rangeIndex = (raw: number | null, values: readonly number[]): number => {
  if (raw === null || Number.isNaN(raw)) return 0;
  const at = values.findIndex((it) => raw <= it);
  return at >= 0 ? at : values.length;
};

export const displayColour = (display: ValueDisplay, raw: number | null): number | null => {
  if (display.rangeType !== RangeType.Color) return null;
  const colour = display.colours[rangeIndex(raw, display.values)] ?? null;
  return colour === null || colour === NO_COLOUR ? null : colour;
};

export const displayOpacity = (display: ValueDisplay, raw: number | null): number => {
  if (display.rangeType !== RangeType.Opacity) return 1;
  const opacity = display.opacities[rangeIndex(raw, display.values)] ?? null;
  return opacity === null ? 1 : coerceIn(opacity, 0, 1);
};

export const displayIcon = (display: ValueDisplay, raw: number | null): string | null => {
  const icon = display.rangeType === RangeType.Icon ? (display.icons[rangeIndex(raw, display.values)] ?? null) : display.showIcon ? display.icon : null;
  return icon !== null && icon.trim() !== '' ? icon : null;
};

export const iconShown = (display: ValueDisplay): boolean => display.rangeType === RangeType.Icon || display.showIcon;

export const displayReading = (display: ValueDisplay, value: string, units: string): string => (display.showUnits && units.trim() !== '' ? `${value} ${units}` : value);

export const displayJson = (display: ValueDisplay): string =>
  JSON.stringify({
    text: display.text,
    showUnits: display.showUnits,
    showIcon: display.showIcon,
    icon: display.icon,
    rangeType: display.rangeType.name,
    values: display.values,
    colours: display.colours,
    opacities: display.opacities,
    icons: display.icons,
  });

const numberAt = (value: unknown): number => (typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN);

const list = <T,>(json: Json, key: string, read: (value: unknown, index: number, array: unknown[]) => T): readonly T[] => (optJSONArray(json, key) ?? []).map(read);

export const displayFrom = (json: string | null): ValueDisplay => {
  const o = parseJson(json);
  if (o === null) return ValueDisplay();
  return ValueDisplay({
    text: optString(o, 'text'),
    showUnits: optBoolean(o, 'showUnits', true),
    showIcon: optBoolean(o, 'showIcon'),
    icon: optString(o, 'icon'),
    rangeType: RangeTypeEntries.find((it) => it.name === optString(o, 'rangeType')) ?? RangeType.None,
    values: list(o, 'values', numberAt),
    colours: list(o, 'colours', (value) => {
      const colour = numberAt(value);
      return Number.isFinite(colour) ? Math.trunc(colour) : 0;
    }),
    opacities: list(o, 'opacities', numberAt),
    icons: list(o, 'icons', (_, index, array) => optArrayText(array, index)),
  });
};

export const readDisplays = (vehicleClass: string): ReadonlyMap<string, ValueDisplay> => {
  const prefix = `${vehicleClass}/`;
  return new Map(
    Object.entries(getSharedPreferences(DISPLAY_STORE).getAll())
      .filter(([key]) => key.startsWith(prefix))
      .map(([key, text]) => [key.slice(prefix.length), displayFrom(text)] as const),
  );
};

export const writeDisplay = (vehicleClass: string, id: string, display: ValueDisplay): void => {
  getSharedPreferences(DISPLAY_STORE).putString(`${vehicleClass}/${id}`, displayJson(display));
};

const iconFile = (path: string): string => path.slice(path.lastIndexOf('/') + 1);

export const iconNames = (): readonly string[] =>
  Object.keys(ICON_URLS)
    .map(iconFile)
    .filter((it) => it.endsWith('.svg'))
    .toSorted();

const iconUrl = (name: string): string => Object.entries(ICON_URLS).find(([path]) => iconFile(path) === name)?.[1] ?? '';

export const ValueIcon = ({ name, tint, size, className, style }: { name: string; tint: string; size: number; className?: string; style?: CSSProperties }) => {
  const url = iconUrl(name);
  return (
    <span
      role="img"
      aria-label={name.endsWith('.svg') ? name.slice(0, -'.svg'.length) : name}
      className={`inline-block shrink-0 ${className ?? ''}`}
      style={{
        width: size,
        height: size,
        backgroundColor: tint,
        maskImage: url === '' ? undefined : `url("${url}")`,
        maskSize: 'contain',
        maskRepeat: 'no-repeat',
        maskPosition: 'center',
        ...style,
      }}
    />
  );
};

const Swatch = ({ colour, chosen, onClick }: { colour: number; chosen: boolean; onClick: () => void }) => (
  <button
    type="button"
    aria-pressed={chosen}
    aria-label={colour === NO_COLOUR ? 'No color' : `#${(colour & 0xffffff).toString(16).padStart(6, '0')}`}
    onClick={onClick}
    className="size-6 shrink-0 rounded-full"
    style={{ background: colour === NO_COLOUR ? 'transparent' : argb(colour), border: `2px solid ${chosen ? md('primary') : md('outline')}` }}
  />
);

const IconPickerDialog = ({ names, chosen, onDismiss, onPick }: { names: readonly string[]; chosen: string; onDismiss: () => void; onPick: (name: string) => void }) => (
  <AlertDialog
    onDismissRequest={onDismiss}
    title="Select icon"
    text={
      <div className="grid h-[360px] overflow-auto" style={{ gridTemplateColumns: 'repeat(auto-fill, minmax(44px, 1fr))', gridAutoRows: 'max-content' }}>
        {names.map((name) => (
          <button key={name} type="button" onClick={() => onPick(name)} className="m-1 flex items-center justify-center p-1.5" style={{ border: `2px solid ${name === chosen ? md('primary') : 'transparent'}` }}>
            <ValueIcon name={name} tint={md('onSurface')} size={24} />
          </button>
        ))}
      </div>
    }
    confirmButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
  />
);

const IconButtonFor = ({ name, onClick }: { name: string; onClick: () => void }) => (
  <button type="button" onClick={onClick} className="flex shrink-0 items-center justify-center p-1.5" style={{ border: `1px solid ${md('outline')}` }}>
    <ValueIcon name={name} tint={md('onSurface')} size={24} />
  </button>
);

const doubleText = (value: number): string => (Number.isInteger(value) ? value.toFixed(1) : String(value));

export const ValueDisplayDialog = ({
  label,
  initial,
  onDismiss,
  extra = null,
  onDone,
}: {
  label: string;
  initial: ValueDisplay;
  onDismiss: () => void;
  extra?: ReactNode;
  onDone: (display: ValueDisplay) => void;
}) => {
  const names = useMemo(() => iconNames(), []);
  const firstIcon = names[0] ?? '';
  const [display, setDisplay] = useState(initial);
  const [picking, setPicking] = useState<((name: string) => void) | null>(null);
  const [pickingCurrent, setPickingCurrent] = useState('');
  const pick = (current: string, chosen: (name: string) => void) => {
    setPickingCurrent(current);
    setPicking(() => chosen);
  };

  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Telemetry display"
      text={
        <div className="flex flex-col gap-2">
          {picking !== null ? (
            <IconPickerDialog
              names={names}
              chosen={pickingCurrent}
              onDismiss={() => setPicking(null)}
              onPick={(name) => {
                picking(name);
                setPicking(null);
              }}
            />
          ) : null}
          <Text typography="labelMedium">{label}</Text>
          {extra}
          <div className="flex flex-wrap items-center gap-2">
            <FilterChip selected={display.showIcon} onClick={() => setDisplay({ ...display, showIcon: true, icon: display.icon.trim() === '' ? firstIcon : display.icon })} label="Icon" />
            <FilterChip selected={!display.showIcon} onClick={() => setDisplay({ ...display, showIcon: false })} label="Text" />
            {display.showIcon ? <IconButtonFor name={display.icon} onClick={() => pick(display.icon, (it) => setDisplay((shown) => ({ ...shown, icon: it })))} /> : null}
          </div>
          {!display.showIcon ? <OutlinedTextField value={display.text} onValueChange={(it) => setDisplay({ ...display, text: it })} label="Text" singleLine /> : null}
          <div className="flex items-center">
            <Text typography="bodyMedium" className="flex-1">
              Show units
            </Text>
            <Switch checked={display.showUnits} onCheckedChange={(it) => setDisplay({ ...display, showUnits: it })} label="Show units" />
          </div>
          <Text typography="titleSmall">Value range</Text>
          <Text typography="bodySmall">Change the color, opacity or icon when the value crosses a threshold</Text>
          <div className="flex flex-wrap gap-1.5">
            {RangeTypeEntries.map((type) => (
              <FilterChip key={type.name} selected={display.rangeType === type} onClick={() => display.rangeType !== type && setDisplay(withRangeType(display, type, firstIcon))} label={type.label} />
            ))}
          </div>
          {display.rangeType !== RangeType.None ? (
            <>
              <Text typography="bodySmall">{rangeHelp(display.rangeType)}</Text>
              {Array.from({ length: display.values.length + 1 }, (_, row) => {
                const limit = display.values[row] ?? null;
                return (
                  <div key={row} className="flex flex-wrap items-center gap-1.5">
                    {limit !== null ? (
                      <OutlinedTextField
                        className="w-24"
                        value={doubleText(limit)}
                        onValueChange={(typed) => {
                          const v = toDoubleOrNull(typed);
                          if (v !== null) setDisplay({ ...display, values: display.values.map((old, i) => (i === row ? v : old)) });
                        }}
                        label="≤"
                        singleLine
                        keyboardType="decimal"
                      />
                    ) : (
                      <Text typography="bodySmall" className="w-24">
                        above
                      </Text>
                    )}
                    <RangeCell display={display} row={row} onChange={setDisplay} onPick={pick} />
                    {row < display.values.length && display.values.length > 1 ? <TextButton onClick={() => setDisplay(withoutRow(display, row))}>✕</TextButton> : null}
                  </div>
                );
              })}
              <TextButton onClick={() => setDisplay(withRow(display, firstIcon))}>Add row</TextButton>
            </>
          ) : null}
        </div>
      }
      confirmButton={<TextButton onClick={() => onDone(display)}>Done</TextButton>}
      dismissButton={<TextButton onClick={onDismiss}>Cancel</TextButton>}
    />
  );
};

export const rangeHelp = (type: RangeType): string => {
  switch (type.name) {
    case 'Color':
      return 'Specify the color you want to apply based on value ranges. The color will be applied to the icon if available, otherwise to the value itself.';
    case 'Opacity':
      return 'Specify the icon opacity you want based on value ranges.';
    case 'Icon':
      return 'Specify the icon you want to display based on value ranges.';
    case 'None':
      return '';
  }
};

const RangeCell = ({ display, row, onChange, onPick }: { display: ValueDisplay; row: number; onChange: (display: ValueDisplay) => void; onPick: (current: string, chosen: (name: string) => void) => void }) => {
  switch (display.rangeType.name) {
    case 'Color':
      return (
        <>
          {RANGE_COLOURS.map((colour) => (
            <Swatch key={colour} colour={colour} chosen={display.colours[row] === colour} onClick={() => onChange({ ...display, colours: display.colours.map((old, i) => (i === row ? colour : old)) })} />
          ))}
        </>
      );
    case 'Opacity': {
      const opacity = display.opacities[row];
      return (
        <OutlinedTextField
          className="w-24"
          value={opacity === undefined ? '' : doubleText(opacity)}
          onValueChange={(typed) => {
            const v = toDoubleOrNull(typed);
            if (v !== null) onChange({ ...display, opacities: display.opacities.map((old, i) => (i === row ? v : old)) });
          }}
          label="Opacity"
          singleLine
          keyboardType="decimal"
        />
      );
    }
    case 'Icon': {
      const current = display.icons[row];
      return current === undefined ? null : (
        <IconButtonFor name={current} onClick={() => onPick(current, (name) => onChange({ ...display, icons: display.icons.map((old, i) => (i === row ? name : old)) }))} />
      );
    }
    case 'None':
      return null;
  }
};

export const ValueLabel = ({ display, raw, label, fallback }: { display: ValueDisplay; raw: number | null; label: string; fallback: string }) => {
  const colour = displayColour(display, raw);
  const tint = colour === null ? fallback : argb(colour);
  const opacity = displayOpacity(display, raw);
  const icon = displayIcon(display, raw);
  if (icon !== null) return <ValueIcon name={icon} tint={tint} size={16} style={{ opacity }} />;
  return iconShown(display) ? null : (
    <Text typography="labelSmall" color={tint} style={{ opacity }}>
      {display.text.trim() === '' ? label : display.text}
    </Text>
  );
};
