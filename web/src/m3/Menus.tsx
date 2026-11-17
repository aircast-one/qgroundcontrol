import { type ReactNode, useState } from 'react';
import { Menu } from '@base-ui/react';
import { MaterialIcon } from '@/res/Icon';

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
              <Menu.Popup className={`max-h-[min(70vh,var(--available-height))] min-w-[max(160px,var(--anchor-width))] overflow-auto rounded-shape-xs bg-surface-container py-2 text-on-surface shadow-xl outline-none ${className ?? ''}`}>
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
  checked?: boolean;
  selected?: boolean;
  closeOnClick?: boolean;
};

export const DropdownMenuItem = ({ text, onClick, leadingIcon, trailingIcon, enabled = true, color, checked, selected, closeOnClick }: DropdownMenuItemProps) => {
  const marked = selected === true || checked === true;
  const trailing = trailingIcon ?? (marked ? <MaterialIcon name="check" size={18} /> : null);
  return (
    <Menu.Item
      {...(checked === undefined ? {} : { role: 'menuitemcheckbox', 'aria-checked': checked })}
      {...(selected === true ? { 'aria-current': true } : {})}
      disabled={!enabled}
      onClick={onClick}
      closeOnClick={closeOnClick}
      className={`flex min-h-12 cursor-pointer items-center gap-3 px-3 text-label-large outline-none desktop:min-h-9 desktop:text-body-medium data-[disabled]:cursor-default data-[disabled]:opacity-40 data-[highlighted]:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]`}
      style={{ color }}
    >
      {leadingIcon ? <span className="flex text-on-surface-variant">{leadingIcon}</span> : null}
      <span className="flex-1">{text}</span>
      {trailing !== null ? <span className={`flex ${marked && trailingIcon === undefined ? 'text-primary' : 'text-on-surface-variant'}`}>{trailing}</span> : null}
    </Menu.Item>
  );
};
