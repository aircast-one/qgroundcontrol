import { type ReactNode, useState } from 'react';
import { Menu } from '@base-ui/react';

export type DropdownMenuProps = {
  expanded: boolean;
  onDismissRequest: () => void;
  children: ReactNode;
  side?: 'bottom' | 'top' | 'left' | 'right';
  align?: 'start' | 'center' | 'end';
  className?: string;
};

export const DropdownMenu = ({ expanded, onDismissRequest, children, side = 'bottom', align = 'start', className }: DropdownMenuProps) => {
  const [anchor, setAnchor] = useState<HTMLElement | null>(null);
  return (
    <>
      <span ref={(node) => setAnchor(node?.parentElement ?? null)} hidden />
      {anchor ? (
        <Menu.Root open={expanded} onOpenChange={(open) => !open && onDismissRequest()} modal={false}>
          <Menu.Portal>
            <Menu.Positioner anchor={anchor} side={side} align={align} sideOffset={4} className="z-50">
              <Menu.Popup className={`max-h-[min(70vh,var(--available-height))] min-w-[160px] overflow-auto rounded-shape-xs bg-surface-container py-2 text-on-surface shadow-xl outline-none ${className ?? ''}`}>
                {children}
              </Menu.Popup>
            </Menu.Positioner>
          </Menu.Portal>
        </Menu.Root>
      ) : null}
    </>
  );
};

export type DropdownMenuItemProps = {
  text: ReactNode;
  onClick: () => void;
  leadingIcon?: ReactNode;
  trailingIcon?: ReactNode;
  enabled?: boolean;
  color?: string;
};

export const DropdownMenuItem = ({ text, onClick, leadingIcon, trailingIcon, enabled = true, color }: DropdownMenuItemProps) => (
  <Menu.Item
    disabled={!enabled}
    onClick={onClick}
    className="flex min-h-12 cursor-pointer items-center gap-3 px-3 text-label-large outline-none data-[disabled]:cursor-default data-[disabled]:opacity-40 data-[highlighted]:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]"
    style={{ color }}
  >
    {leadingIcon ? <span className="flex text-on-surface-variant">{leadingIcon}</span> : null}
    <span className="flex-1">{text}</span>
    {trailingIcon ? <span className="flex text-on-surface-variant">{trailingIcon}</span> : null}
  </Menu.Item>
);
