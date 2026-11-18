import type { CSSProperties, ReactNode } from 'react';
import { HorizontalDivider, Text, TooltipBox, VerticalDivider } from '@/m3';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { md, withAlpha } from '@/theme/Colors';

export const RAIL_WIDTH = 52;
const RAIL_ITEM = 44;
const RAIL_LABELLED_ITEM = 52;
export const DOCKED_RAIL_HEIGHT = RAIL_LABELLED_ITEM + 8;
const RAIL_LABEL_SIZE = 10;
const RAIL_ICON = 22;
const RAIL_ALPHA = 0.94;
const DISABLED_ALPHA = 0.38;

export const PlanRail = ({ className, style, children, docked = false }: { className?: string; style?: CSSProperties; children?: ReactNode; docked?: boolean }) => (
  <div className={`flex overflow-hidden rounded-[20px] ${docked ? 'flex-row' : 'flex-col'} ${className ?? ''}`} style={{ background: withAlpha(md('surfaceContainer'), RAIL_ALPHA), ...style }}>
    {docked ? (
      <div role="toolbar" className="flex min-w-0 flex-row items-center gap-1 overflow-x-auto px-1.5" style={{ height: DOCKED_RAIL_HEIGHT }}>
        {children}
      </div>
    ) : (
      <div className="flex min-h-0 flex-col items-center overflow-y-auto py-0.5" style={{ width: RAIL_WIDTH }}>
        {children}
      </div>
    )}
  </div>
);

export const RailButton = ({
  icon,
  label,
  onClick,
  enabled = true,
  chosen = false,
  labelled = false,
}: {
  icon: DrawableName;
  label: string;
  onClick: () => void;
  enabled?: boolean;
  chosen?: boolean;
  labelled?: boolean;
}) => (
  <TooltipBox tooltip={label}>
    <button
      type="button"
      onClick={onClick}
      disabled={!enabled}
      aria-label={label}
      aria-pressed={chosen}
      className="my-0.5 flex shrink-0 flex-col items-center justify-center rounded-[14px] focus-visible:outline-2 focus-visible:outline-primary"
      style={{
        width: RAIL_ITEM,
        height: labelled ? RAIL_LABELLED_ITEM : RAIL_ITEM,
        opacity: enabled ? 1 : DISABLED_ALPHA,
        background: chosen ? md('primaryContainer') : 'transparent',
        color: chosen ? md('onPrimaryContainer') : md('onSurface'),
      }}
    >
      <Drawable name={icon} size={RAIL_ICON} />
      {labelled ? (
        <Text typography="labelSmall" fontSize={RAIL_LABEL_SIZE} maxLines={1}>
          {label}
        </Text>
      ) : null}
    </button>
  </TooltipBox>
);

export const RailDivider = ({ docked = false }: { docked?: boolean }) =>
  docked ? (
    <div className="flex h-7 shrink-0 px-0.5">
      <VerticalDivider />
    </div>
  ) : (
    <div className="w-7 shrink-0 py-0.5">
      <HorizontalDivider />
    </div>
  );
