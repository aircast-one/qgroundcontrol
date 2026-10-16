import { type ReactNode, createContext, useContext, useEffect, useMemo, useState } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { coerceIn, format } from '@/kotlin/Std';
import { FilledTonalIconButton, FilterChip, OutlinedTextField, Surface, Text, TextButton } from '@/m3';
import { typedNumber } from '@/map/AltitudeEntry';
import { VEHICLES_VIEW, vehicleChoices } from '@/map/VehiclePicker';
import { aircast, md } from '@/theme/Colors';
import { useFlyIsPortrait, useScreenSize } from './FlyShell';
import { LocalFlyScreenState } from './FlyScreenState';
import { FLY_STATE, type Readiness, flyState, guidedReadiness } from './FlyState';
import { HoldToConfirm, holdLabel } from './HoldToConfirm';

export const GuidedValuePanel = ({
  title,
  sentence,
  commitLabel,
  commitEnabled,
  onCommit,
  onCancel,
  className,
  children,
}: {
  title: string;
  sentence: string;
  commitLabel: string;
  commitEnabled: boolean;
  onCommit: () => void;
  onCancel: () => void;
  className?: string;
  children?: ReactNode;
}) => {
  const flyScreen = useContext(LocalFlyScreenState);
  const portrait = useFlyIsPortrait();
  const available = useScreenSize().height;
  useEffect(() => {
    flyScreen.guidedPanels.value += 1;
    return () => {
      flyScreen.guidedPanels.value -= 1;
    };
  }, [flyScreen]);
  const flyJson = useQgcPath(FLY_STATE);
  const readiness = useMemo(() => guidedReadiness(flyState(flyJson)), [flyJson]);
  const vehiclesJson = useQgcPath(VEHICLES_VIEW);
  const vehicles = useMemo(() => vehicleChoices(vehiclesJson), [vehiclesJson]);
  const named = guidedVehicle(vehicles.choices.length, vehicles.active?.name ?? null);
  const hold = holdLabel(guidedCommitLabel(commitLabel, readiness));
  const holdEnabled = commitEnabled && readiness?.blocks !== true;
  return (
    <div className={`w-full ${className ?? ''}`}>
      <LocalGuidedCompact.Provider value={guidedCompact(portrait, available)}>
        <Surface className="w-full" color={md('surfaceContainerLow')} shape="extraLarge">
          <div className="flex max-h-[calc(100dvh-32px)] w-full flex-col gap-2 p-4">
            <div className="flex min-h-0 flex-col gap-2 overflow-y-auto px-2">
              <Text typography="titleLarge">{title}</Text>
              {named !== null ? <Text typography="labelLarge" color={md('primary')}>{named}</Text> : null}
              {sentence.trim() !== '' ? <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="whitespace-pre-line">{sentence}</Text> : null}
              {readiness !== null ? (
                <Surface
                  color={readiness.blocks ? md('errorContainer') : aircast('warningContainer')}
                  contentColor={readiness.blocks ? md('onErrorContainer') : md('onSurface')}
                  shape="medium"
                >
                  <Text typography="bodyMedium" className="block p-3">{readiness.text}</Text>
                </Surface>
              ) : null}
              {children}
            </div>
            {portrait ? (
              <>
                <HoldToConfirm label={hold} enabled={holdEnabled} onConfirm={onCommit} />
                <TextButton onClick={onCancel} className="self-center">Cancel</TextButton>
              </>
            ) : (
              <div className="flex items-center gap-2">
                <HoldToConfirm label={hold} enabled={holdEnabled} className="flex-1" onConfirm={onCommit} />
                <TextButton onClick={onCancel}>Cancel</TextButton>
              </div>
            )}
          </div>
        </Surface>
      </LocalGuidedCompact.Provider>
    </div>
  );
};

export const LocalGuidedCompact = createContext(false);

const COMPACT_PANEL_HEIGHT = 440;

export const guidedCompact = (portrait: boolean, available: number): boolean => !portrait && available < COMPACT_PANEL_HEIGHT;

export const guidedCommitLabel = (label: string, readiness: Readiness | null): string => {
  const at = label.indexOf(' · ');
  const action = at < 0 ? label : label.slice(0, at);
  const target = at < 0 ? null : label.slice(at + 3);
  const anyway = readiness !== null && !readiness.blocks ? `${action} anyway` : action;
  return [anyway, target].filter((it): it is string => it !== null).join(' · ');
};

export const guidedPresets = (unit: string, minimum: number, maximum: number): readonly number[] =>
  (unit === 'ft' ? [15.0, 30.0, 60.0, 150.0] : [5.0, 10.0, 20.0, 50.0]).filter((it) => it >= minimum && it <= maximum);

const withoutPointZero = (text: string) => (text.endsWith('.0') ? text.slice(0, -2) : text);

export const GuidedPresets = ({ value, minimum, maximum, unit, onValue }: { value: number; minimum: number; maximum: number; unit: string; onValue: (value: number) => void }) => (
  <div className="flex flex-wrap gap-2">
    {guidedPresets(unit, minimum, maximum).map((preset) => (
      <FilterChip key={preset} selected={preset === value} onClick={() => onValue(preset)} label={`${withoutPointZero(guidedValueText(preset, unit))} ${unit}`} />
    ))}
  </div>
);

export const guidedVehicle = (vehicleCount: number, activeName: string | null): string | null =>
  activeName !== null && vehicleCount >= 2 && activeName.trim() !== '' ? activeName : null;

const METRIC_UNITS: ReadonlySet<string> = new Set(['m', 'm/s', 'km/h']);

export const guidedBounds = (minimum: number | null, maximum: number | null): readonly [number, number] | null =>
  minimum !== null && maximum !== null ? [minimum, maximum] : null;

export const guidedDecimals = (unit: string): number => (METRIC_UNITS.has(unit) ? 1 : 0);

export const guidedRounded = (value: number, unit: string): number => Number(value.toFixed(guidedDecimals(unit))) + 0;

export const guidedStepped = (value: number, delta: number, minimum: number, maximum: number, unit: string): number =>
  guidedRounded(coerceIn(value + delta, minimum, maximum), unit);

export const guidedTyped = (text: string, minimum: number, maximum: number, unit: string): number | null => {
  const typed = typedNumber(text);
  return typed === null ? null : guidedRounded(coerceIn(typed, minimum, maximum), unit);
};

export const guidedValueText = (value: number, unit: string): string => format(`%.${guidedDecimals(unit)}f`, value);

export const guidedQuickPicks = (value: number, minimum: number, maximum: number, unit: string): readonly (readonly [string, number])[] => [
  ...[10, 20].map((step) => [`+${step} ${unit}`.trim(), guidedStepped(value, step, minimum, maximum, unit)] as const),
  [`Max ${withoutPointZero(guidedValueText(maximum, unit))} ${unit}`.trim(), guidedRounded(maximum, unit)] as const,
];

export const GuidedQuickPicks = ({ value, minimum, maximum, unit, onValue }: { value: number; minimum: number; maximum: number; unit: string; onValue: (value: number) => void }) => (
  <div className="flex flex-wrap gap-2">
    {guidedQuickPicks(value, minimum, maximum, unit).map(([label, target]) => (
      <FilterChip key={label} selected={false} enabled={target !== value} onClick={() => onValue(target)} label={label} />
    ))}
  </div>
);

export const GuidedStepper = ({
  value,
  label,
  unit,
  minimum,
  maximum,
  onValue,
}: {
  value: number;
  label: string;
  unit: string;
  minimum: number;
  maximum: number;
  onValue: (value: number) => void;
}) => {
  const [typing, setTyping] = useState<string | null>(null);
  const compact = useContext(LocalGuidedCompact);
  const size = compact ? 'text-headline-medium' : 'text-display-small @max-[220px]:text-headline-medium';
  return (
    <div className="@container w-full">
      <div className="flex w-full items-center @max-[220px]:flex-col @max-[220px]:items-start @max-[220px]:gap-2">
        <div className="flex min-w-0 flex-1 flex-col @max-[220px]:flex-none">
          {typing === null ? (
            <button type="button" className={`${size} whitespace-nowrap text-left tabular-nums`} onClick={() => setTyping(guidedValueText(value, unit))}>
              {guidedReading(value, unit)}
            </button>
          ) : (
            <OutlinedTextField
              value={typing}
              onValueChange={(text) => {
                setTyping(text);
                const typed = guidedTyped(text, minimum, maximum, unit);
                if (typed !== null) onValue(typed);
              }}
              singleLine
              keyboardType="decimal"
              autoFocus
              onDone={() => {
                if (document.activeElement instanceof HTMLElement) document.activeElement.blur();
                setTyping(null);
              }}
              className="w-40"
            />
          )}
          {label.trim() !== '' ? <Text typography="labelLarge" color={md('onSurfaceVariant')}>{label}</Text> : null}
        </div>
        <div className="flex gap-2">
          {([[-1, '−'], [1, '+']] as const).map(([delta, sign]) => (
            <FilledTonalIconButton key={delta} onClick={() => onValue(guidedStepped(value, delta, minimum, maximum, unit))} containerColor={md('surfaceContainerHighest')} contentColor={md('onSurface')}>
              <Text typography="titleLarge">{sign}</Text>
            </FilledTonalIconButton>
          ))}
        </div>
      </div>
    </div>
  );
};

export const guidedReading = (value: number, unit: string): string =>
  [guidedValueText(value, unit), unit].filter((it) => it.trim() !== '').join(' ');
