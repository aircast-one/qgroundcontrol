import type { CSSProperties, ReactNode } from 'react';
import { md } from '@/theme/Colors';

export type ListItemProps = {
  headlineContent: ReactNode;
  supportingContent?: ReactNode;
  overlineContent?: ReactNode;
  leadingContent?: ReactNode;
  trailingContent?: ReactNode;
  onClick?: () => void;
  enabled?: boolean;
  containerColor?: string;
  className?: string;
  style?: CSSProperties;
};

export const ListItem = ({ headlineContent, supportingContent, overlineContent, leadingContent, trailingContent, onClick, enabled = true, containerColor, className, style }: ListItemProps) => {
  const body = (
    <>
      {leadingContent ? <span className="flex shrink-0 items-center text-on-surface-variant">{leadingContent}</span> : null}
      <span className="grid min-w-0 flex-1 gap-0.5">
        {overlineContent ? <span className="text-label-small text-on-surface-variant">{overlineContent}</span> : null}
        <span className="text-body-large text-on-surface">{headlineContent}</span>
        {supportingContent ? <span className="text-body-medium text-on-surface-variant">{supportingContent}</span> : null}
      </span>
      {trailingContent ? <span className="flex shrink-0 items-center text-label-small text-on-surface-variant">{trailingContent}</span> : null}
    </>
  );
  const look = `flex min-h-14 w-full items-center gap-4 px-4 py-2 text-left ${className ?? ''}`;
  return onClick ? (
    <button type="button" disabled={!enabled} onClick={onClick} className={`${look} hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)] disabled:opacity-40`} style={{ background: containerColor, ...style }}>
      {body}
    </button>
  ) : (
    <div className={look} style={{ background: containerColor, ...style }}>
      {body}
    </div>
  );
};

export const HorizontalDivider = ({ thickness = 1, color = md('outlineVariant'), className }: { thickness?: number; color?: string; className?: string }) => (
  <hr className={`m-0 w-full shrink-0 border-0 ${className ?? ''}`} style={{ height: thickness, background: color }} />
);

export const VerticalDivider = ({ thickness = 1, color = md('outlineVariant'), className }: { thickness?: number; color?: string; className?: string }) => (
  <span role="separator" aria-orientation="vertical" className={`block self-stretch ${className ?? ''}`} style={{ width: thickness, background: color }} />
);
