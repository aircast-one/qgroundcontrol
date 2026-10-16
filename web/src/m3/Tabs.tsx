import type { ReactNode } from 'react';
import { MaterialIcon } from '@/res/Icon';

export const TabRow = ({ children, scrollable = false, className }: { selectedTabIndex?: number; children: ReactNode; scrollable?: boolean; className?: string }) => (
  <div role="tablist" className={`flex border-b border-surface-variant ${scrollable ? 'overflow-x-auto' : ''} ${className ?? ''}`}>
    {children}
  </div>
);

export const ScrollableTabRow = (props: { selectedTabIndex?: number; children: ReactNode; className?: string }) => <TabRow {...props} scrollable />;

export const Tab = ({ selected, onClick, text, icon, enabled = true }: { selected: boolean; onClick: () => void; text?: ReactNode; icon?: ReactNode; enabled?: boolean }) => (
  <button
    type="button"
    role="tab"
    aria-selected={selected}
    disabled={!enabled}
    onClick={onClick}
    className={`relative flex min-h-12 flex-1 shrink-0 flex-col items-center justify-center gap-1 px-4 text-title-small disabled:opacity-40 ${selected ? 'text-primary' : 'text-on-surface-variant'} hover:bg-[color-mix(in_srgb,currentColor_8%,transparent)]`}
  >
    {icon}
    {text}
    {selected ? <span className="absolute inset-x-4 bottom-0 h-[3px] rounded-t-full bg-primary" /> : null}
  </button>
);

export const SingleChoiceSegmentedButtonRow = ({ children, className }: { children: ReactNode; className?: string }) => (
  <div role="radiogroup" className={`inline-flex ${className ?? ''}`}>
    {children}
  </div>
);

export const MultiChoiceSegmentedButtonRow = ({ children, className }: { children: ReactNode; className?: string }) => (
  <div role="group" className={`inline-flex ${className ?? ''}`}>
    {children}
  </div>
);

export const SegmentedButton = ({ selected, onClick, label, icon, index, count, enabled = true, multi = false }: { selected: boolean; onClick: () => void; label: ReactNode; icon?: ReactNode; index: number; count: number; enabled?: boolean; multi?: boolean }) => (
  <button
    type="button"
    role={multi ? 'checkbox' : 'radio'}
    aria-checked={selected}
    disabled={!enabled}
    onClick={onClick}
    className={`-ml-px flex h-10 flex-1 items-center justify-center gap-2 border border-outline px-3 text-label-large first:ml-0 disabled:opacity-40 ${index === 0 ? 'rounded-l-full' : ''} ${index === count - 1 ? 'rounded-r-full' : ''} ${selected ? 'bg-secondary-container text-on-secondary-container' : 'text-on-surface'}`}
  >
    {selected ? <MaterialIcon name="check" size={18} /> : icon}
    {label}
  </button>
);
