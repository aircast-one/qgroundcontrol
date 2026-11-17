import type { CSSProperties, ReactElement, ReactNode, Ref } from 'react';
import { Tooltipped } from './Feedback';

type ButtonProps = {
  children?: ReactNode;
  onClick?: () => void;
  enabled?: boolean;
  className?: string;
  style?: CSSProperties;
  label?: string;
  title?: string;
  type?: 'button' | 'submit';
  containerColor?: string;
  contentColor?: string;
  ref?: Ref<HTMLButtonElement>;
};

const base = 'inline-flex select-none items-center justify-center gap-2 text-label-large transition-[background,opacity] disabled:cursor-default disabled:opacity-[0.38] focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--md-primary)]';
const pill = 'h-10 rounded-full px-6';

const button = (look: string, overlay: string, { children, onClick, enabled = true, className, style, label, title, type = 'button', containerColor, contentColor, ref }: ButtonProps) => (
  <button
    ref={ref}
    type={type}
    className={`${base} ${look} ${enabled ? overlay : ''} ${className ?? ''}`}
    style={{ background: containerColor, color: contentColor, ...style }}
    onClick={onClick}
    disabled={!enabled}
    aria-label={label}
    title={title}
  >
    {children}
  </button>
);

const make = (look: string, overlay: string) => (props: ButtonProps) => button(look, overlay, props);

const withTooltip = (tip: string | undefined, element: ReactElement) => (tip === undefined || tip === '' ? element : <Tooltipped tooltip={tip}>{element}</Tooltipped>);

const makeIcon = (look: string, overlay: string) => ({ title, ...props }: ButtonProps) => withTooltip(title ?? props.label, button(look, overlay, { ...props, label: props.label ?? title }));

export const Button = make(`${pill} bg-primary text-on-primary`, 'hover:brightness-110 active:brightness-125');
export const FilledTonalButton = make(`${pill} bg-secondary-container text-on-secondary-container`, 'hover:brightness-110 active:brightness-125');
export const ElevatedButton = make(`${pill} bg-surface-container-low text-primary shadow`, 'hover:bg-surface-container');
export const OutlinedButton = make(`${pill} border border-outline text-primary`, 'hover:bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]');
export const TextButton = make('h-10 rounded-full px-3 text-primary', 'hover:bg-[color-mix(in_srgb,var(--md-primary)_8%,transparent)]');

export const IconButton = makeIcon('size-10 rounded-full text-on-surface-variant', 'hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)]');
export const FilledIconButton = makeIcon('size-10 rounded-full bg-primary text-on-primary', 'hover:brightness-110');
export const FilledTonalIconButton = makeIcon('size-10 rounded-full bg-secondary-container text-on-secondary-container', 'hover:brightness-110');
export const OutlinedIconButton = makeIcon('size-10 rounded-full border border-outline text-on-surface-variant', 'hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)]');

export const FloatingActionButton = make('size-14 rounded-shape-lg bg-primary-container text-on-primary-container shadow-lg', 'hover:brightness-110');
export const SmallFloatingActionButton = make('size-10 rounded-shape-md bg-primary-container text-on-primary-container shadow-md', 'hover:brightness-110');
export const ExtendedFloatingActionButton = make('h-14 rounded-shape-lg bg-primary-container px-4 text-on-primary-container shadow-lg', 'hover:brightness-110');

export const IconToggleButton = ({ checked, onCheckedChange, enabled = true, children, label, className }: { checked: boolean; onCheckedChange: (checked: boolean) => void; enabled?: boolean; children: ReactNode; label?: string; className?: string }) =>
  withTooltip(
    label,
    <button
      type="button"
      aria-pressed={checked}
      aria-label={label}
      disabled={!enabled}
      onClick={() => onCheckedChange(!checked)}
      className={`${base} size-10 rounded-full ${checked ? 'text-primary' : 'text-on-surface-variant'} hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)] ${className ?? ''}`}
    >
      {children}
    </button>,
  );
