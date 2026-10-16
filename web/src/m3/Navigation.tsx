import type { ReactNode } from 'react';

type ItemProps = { selected: boolean; onClick: () => void; icon: ReactNode; label?: ReactNode; enabled?: boolean };

export const NavigationBar = ({ children, className }: { children: ReactNode; className?: string }) => (
  <nav className={`flex h-20 items-stretch bg-surface-container pb-[env(safe-area-inset-bottom)] ${className ?? ''}`}>{children}</nav>
);

export const NavigationBarItem = ({ selected, onClick, icon, label, enabled = true }: ItemProps) => (
  <button type="button" aria-current={selected ? 'page' : undefined} disabled={!enabled} onClick={onClick} className="flex flex-1 flex-col items-center justify-center gap-1 text-label-medium text-on-surface-variant disabled:opacity-40">
    <span className={`flex h-8 w-16 items-center justify-center rounded-full ${selected ? 'bg-secondary-container text-on-secondary-container' : ''}`}>{icon}</span>
    {label ? <span className={selected ? 'text-on-surface' : undefined}>{label}</span> : null}
  </button>
);

export const NavigationRail = ({ children, header, className }: { children: ReactNode; header?: ReactNode; className?: string }) => (
  <nav className={`flex w-20 flex-col items-center gap-3 bg-surface py-3 ${className ?? ''}`}>
    {header}
    {children}
  </nav>
);

export const NavigationRailItem = ({ selected, onClick, icon, label, enabled = true }: ItemProps) => (
  <button type="button" aria-current={selected ? 'page' : undefined} disabled={!enabled} onClick={onClick} className="flex w-full flex-col items-center gap-1 text-label-medium text-on-surface-variant disabled:opacity-40">
    <span className={`flex h-8 w-14 items-center justify-center rounded-full ${selected ? 'bg-secondary-container text-on-secondary-container' : ''}`}>{icon}</span>
    {label ? <span className={selected ? 'text-on-surface' : undefined}>{label}</span> : null}
  </button>
);
