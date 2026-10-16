import type { ReactNode } from 'react';
import { MaterialIcon } from '@/res/Icon';

type ChipProps = { onClick: () => void; label: ReactNode; leadingIcon?: ReactNode; trailingIcon?: ReactNode; enabled?: boolean; className?: string };

const chip = 'inline-flex h-8 select-none items-center gap-2 rounded-shape-sm px-3 text-label-large disabled:opacity-40 focus-visible:outline-2 focus-visible:outline-[var(--md-primary)]';

export const FilterChip = ({ selected, onClick, label, leadingIcon, trailingIcon, enabled = true, className }: ChipProps & { selected: boolean }) => (
  <button
    type="button"
    aria-pressed={selected}
    disabled={!enabled}
    onClick={onClick}
    className={`${chip} ${selected ? 'bg-secondary-container text-on-secondary-container' : 'border border-outline text-on-surface-variant hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)]'} ${className ?? ''}`}
  >
    {leadingIcon ?? (selected ? <MaterialIcon name="check" size={18} /> : null)}
    {label}
    {trailingIcon}
  </button>
);

export const AssistChip = ({ onClick, label, leadingIcon, trailingIcon, enabled = true, className }: ChipProps) => (
  <button type="button" disabled={!enabled} onClick={onClick} className={`${chip} border border-outline text-on-surface hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)] ${className ?? ''}`}>
    {leadingIcon ? <span className="flex text-primary">{leadingIcon}</span> : null}
    {label}
    {trailingIcon}
  </button>
);

export const SuggestionChip = AssistChip;

export const InputChip = ({ selected, onClick, label, leadingIcon, trailingIcon, enabled = true, className }: ChipProps & { selected: boolean }) => (
  <button type="button" aria-pressed={selected} disabled={!enabled} onClick={onClick} className={`${chip} ${selected ? 'bg-secondary-container text-on-secondary-container' : 'border border-outline text-on-surface-variant'} ${className ?? ''}`}>
    {leadingIcon}
    {label}
    {trailingIcon}
  </button>
);
