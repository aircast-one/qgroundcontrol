import { type ReactNode, useId } from 'react';
import { Checkbox as BaseCheckbox, Slider as BaseSlider, Switch as BaseSwitch } from '@base-ui/react';
import { MaterialIcon } from '@/res/Icon';

export type KeyboardType = 'text' | 'number' | 'decimal' | 'password' | 'uri' | 'email' | 'phone';

const inputMode = (keyboard: KeyboardType) => ({ text: 'text', number: 'numeric', decimal: 'decimal', password: 'text', uri: 'url', email: 'email', phone: 'tel' } as const)[keyboard];

export type TextFieldProps = {
  value: string;
  onValueChange: (value: string) => void;
  label?: ReactNode;
  placeholder?: string;
  supportingText?: ReactNode;
  isError?: boolean;
  singleLine?: boolean;
  minLines?: number;
  enabled?: boolean;
  readOnly?: boolean;
  keyboardType?: KeyboardType;
  leadingIcon?: ReactNode;
  trailingIcon?: ReactNode;
  prefix?: ReactNode;
  suffix?: ReactNode;
  onDone?: () => void;
  autoFocus?: boolean;
  className?: string;
};

const field = (filled: boolean) => ({ value, onValueChange, label, placeholder, supportingText, isError = false, singleLine = true, minLines = 3, enabled = true, readOnly = false, keyboardType = 'text', leadingIcon, trailingIcon, prefix, suffix, onDone, autoFocus, className }: TextFieldProps) => {
  const id = useId();
  const tone = isError ? 'border-error text-error' : 'border-outline focus-within:border-primary focus-within:border-2';
  const control = 'min-w-0 flex-1 bg-transparent text-body-large text-on-surface outline-none placeholder:text-on-surface-variant/60 disabled:opacity-40';
  return (
    <div className={`grid gap-1 ${className ?? ''}`}>
      {label ? <label htmlFor={id} className={`text-body-small ${isError ? 'text-error' : 'text-on-surface-variant'}`}>{label}</label> : null}
      <div className={`flex min-h-14 items-center gap-3 px-4 ${filled ? 'rounded-t-shape-xs border-b bg-surface-container-highest' : 'rounded-shape-xs border'} ${tone} ${enabled ? '' : 'opacity-40'}`}>
        {leadingIcon ? <span className="flex text-on-surface-variant">{leadingIcon}</span> : null}
        {prefix ? <span className="text-body-large text-on-surface-variant">{prefix}</span> : null}
        {singleLine ? (
          <input
            id={id}
            className={control}
            value={value}
            placeholder={placeholder}
            disabled={!enabled}
            readOnly={readOnly}
            autoFocus={autoFocus}
            type={keyboardType === 'password' ? 'password' : 'text'}
            inputMode={inputMode(keyboardType)}
            aria-invalid={isError || undefined}
            onChange={(event) => onValueChange(event.target.value)}
            onKeyDown={(event) => event.key === 'Enter' && onDone?.()}
          />
        ) : (
          <textarea id={id} className={`${control} py-3`} rows={minLines} value={value} placeholder={placeholder} disabled={!enabled} readOnly={readOnly} autoFocus={autoFocus} aria-invalid={isError || undefined} onChange={(event) => onValueChange(event.target.value)} />
        )}
        {suffix ? <span className="text-body-large text-on-surface-variant">{suffix}</span> : null}
        {trailingIcon ? <span className="flex text-on-surface-variant">{trailingIcon}</span> : null}
      </div>
      {supportingText ? <span className={`px-4 text-body-small ${isError ? 'text-error' : 'text-on-surface-variant'}`}>{supportingText}</span> : null}
    </div>
  );
};

export const OutlinedTextField = field(false);
export const TextField = field(true);

export const Checkbox = ({ checked, onCheckedChange, enabled = true, label }: { checked: boolean; onCheckedChange: ((checked: boolean) => void) | null; enabled?: boolean; label?: string }) => (
  <BaseCheckbox.Root
    checked={checked}
    disabled={!enabled}
    readOnly={onCheckedChange === null}
    onCheckedChange={(next) => onCheckedChange?.(next)}
    aria-label={label}
    className="grid size-[18px] shrink-0 place-items-center rounded-[2px] border-2 border-on-surface-variant data-[checked]:border-primary data-[checked]:bg-primary data-[disabled]:opacity-40"
  >
    <BaseCheckbox.Indicator className="flex text-on-primary">
      <MaterialIcon name="check" size={16} />
    </BaseCheckbox.Indicator>
  </BaseCheckbox.Root>
);

export const Switch = ({ checked, onCheckedChange, enabled = true, label }: { checked: boolean; onCheckedChange: ((checked: boolean) => void) | null; enabled?: boolean; label?: string }) => (
  <BaseSwitch.Root
    checked={checked}
    disabled={!enabled}
    readOnly={onCheckedChange === null}
    onCheckedChange={(next) => onCheckedChange?.(next)}
    aria-label={label}
    className="relative inline-flex h-8 w-[52px] shrink-0 items-center rounded-full border-2 border-outline bg-surface-container-highest transition-colors data-[checked]:border-primary data-[checked]:bg-primary data-[disabled]:opacity-40"
  >
    <BaseSwitch.Thumb className="ml-1 block size-4 rounded-full bg-outline transition-all data-[checked]:ml-[22px] data-[checked]:size-6 data-[checked]:bg-on-primary" />
  </BaseSwitch.Root>
);

export const RadioButton = ({ selected, onClick, enabled = true, label }: { selected: boolean; onClick: (() => void) | null; enabled?: boolean; label?: string }) => (
  <button
    type="button"
    role="radio"
    aria-checked={selected}
    aria-label={label}
    disabled={!enabled}
    onClick={() => onClick?.()}
    className={`grid size-5 shrink-0 place-items-center rounded-full border-2 disabled:opacity-40 ${selected ? 'border-primary' : 'border-on-surface-variant'}`}
  >
    {selected ? <span className="size-2.5 rounded-full bg-primary" /> : null}
  </button>
);

export type SliderProps = {
  value: number;
  onValueChange: (value: number) => void;
  valueRange?: [number, number];
  steps?: number;
  onValueChangeFinished?: () => void;
  enabled?: boolean;
  label?: string;
  className?: string;
};

const stepOf = (range: [number, number], steps: number) => (steps > 0 ? (range[1] - range[0]) / (steps + 1) : (range[1] - range[0]) / 1000);

export const Slider = ({ value, onValueChange, valueRange = [0, 1], steps = 0, onValueChangeFinished, enabled = true, label, className }: SliderProps) => (
  <BaseSlider.Root
    value={value}
    min={valueRange[0]}
    max={valueRange[1]}
    step={stepOf(valueRange, steps)}
    disabled={!enabled}
    onValueChange={(next) => onValueChange(Array.isArray(next) ? (next[0] ?? value) : next)}
    onValueCommitted={() => onValueChangeFinished?.()}
    aria-label={label}
    className={`w-full data-[disabled]:opacity-40 ${className ?? ''}`}
  >
    <BaseSlider.Control className="flex h-11 w-full touch-none items-center">
      <BaseSlider.Track className="h-1 w-full rounded-full bg-secondary-container">
        <BaseSlider.Indicator className="rounded-full bg-primary" />
        <BaseSlider.Thumb className="size-5 rounded-full bg-primary shadow outline-none focus-visible:ring-4 focus-visible:ring-primary/30" />
      </BaseSlider.Track>
    </BaseSlider.Control>
  </BaseSlider.Root>
);

export const RangeSlider = ({ value, onValueChange, valueRange = [0, 1], steps = 0, onValueChangeFinished, enabled = true, label }: Omit<SliderProps, 'value' | 'onValueChange'> & { value: [number, number]; onValueChange: (value: [number, number]) => void }) => (
  <BaseSlider.Root
    value={value}
    min={valueRange[0]}
    max={valueRange[1]}
    step={stepOf(valueRange, steps)}
    disabled={!enabled}
    onValueChange={(next) => Array.isArray(next) && onValueChange([next[0] ?? value[0], next[1] ?? value[1]])}
    onValueCommitted={() => onValueChangeFinished?.()}
    aria-label={label}
    className="w-full data-[disabled]:opacity-40"
  >
    <BaseSlider.Control className="flex h-11 w-full touch-none items-center">
      <BaseSlider.Track className="h-1 w-full rounded-full bg-secondary-container">
        <BaseSlider.Indicator className="rounded-full bg-primary" />
        <BaseSlider.Thumb index={0} className="size-5 rounded-full bg-primary shadow outline-none" />
        <BaseSlider.Thumb index={1} className="size-5 rounded-full bg-primary shadow outline-none" />
      </BaseSlider.Track>
    </BaseSlider.Control>
  </BaseSlider.Root>
);
