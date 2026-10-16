import { type HTMLAttributes, type KeyboardEvent, type PointerEvent, type ReactNode, useEffect, useRef, useState } from 'react';
import { useUpdatedRef } from '@/kotlin/Compose';
import { vibrate } from '@/kotlin/Platform';
import { coerceAtLeast, coerceIn } from '@/kotlin/Std';
import { Surface, Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { sentenceCase } from '@/map/SentenceCase';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { aircast, md, withAlpha } from '@/theme/Colors';
import type { Readiness } from '@/ui/FlyState';
import type { GuidedOffer } from '@/ui/GuidedActions';
import { type GuidedTakeoff, takeoffRangeUsable } from '@/ui/GuidedTakeoff';
import { PAUSE } from '@/ui/MoreActions';
import { osdShadow } from '@/ui/Osd';

export const CHECKLIST = 'checklist';

export type FlyDeckLayout = { readonly name: 'Bottom' | 'Rail'; readonly ordinal: number };
export const FlyDeckLayout = {
  Bottom: { name: 'Bottom', ordinal: 0 },
  Rail: { name: 'Rail', ordinal: 1 },
} as const satisfies Record<string, FlyDeckLayout>;
export const FlyDeckLayoutEntries: readonly FlyDeckLayout[] = [FlyDeckLayout.Bottom, FlyDeckLayout.Rail];

const DISABLED_ALPHA = 0.38;
const DECK_BUTTON_HEIGHT = 80;
const DECK_ICON_SIZE = 28;

export type DeckEntry = {
  id: string;
  label: string;
  icon: DrawableName;
  enabled: boolean;
  warning: boolean;
  onHold: (() => void) | null;
  onClick: () => void;
};

export const DeckEntry = (fields: Omit<DeckEntry, 'warning' | 'onHold'> & Partial<Pick<DeckEntry, 'warning' | 'onHold'>>): DeckEntry => ({
  warning: false,
  onHold: null,
  ...fields,
});

export const HOLD_TO_TAKE_OFF = 'Slide to take off';
export const TAKE_OFF = 'Take off';
export const NO_LINK = 'No link';
export const NO_SIGNAL_CODE = 'noSignal';

export const takeoffLabel = (offer: GuidedOffer | null, readiness: Readiness | null): string => {
  if (offer?.reasonCode === NO_SIGNAL_CODE) return NO_LINK;
  return readiness === null ? HOLD_TO_TAKE_OFF : TAKE_OFF;
};

export const DECK_HOLD_MS = 1500;
const SLIDE_SPAN = 0.6;

export const slideProgress = (slidPx: number, widthPx: number): number => coerceIn(slidPx / (coerceAtLeast(widthPx, 1) * SLIDE_SPAN), 0, 1);

const DECK_HOLD_FILL_ALPHA = 0.3;
const TOUCH_SLOP = 8;
const LONG_PRESS_TIMEOUT_MS = 400;
const LONG_PRESS_HAPTIC_MS = 30;

export const holdTakeoffHeight = (takeoff: GuidedTakeoff | null): number | null => (takeoff !== null && takeoffRangeUsable(takeoff) ? takeoff.initial : null);

type Press = { pointer: number | null; startX: number; pressedAt: number; fired: boolean; sliding: boolean; frame: number | null };

const isActivation = (event: KeyboardEvent) => event.key === 'Enter' || event.key === ' ';

export const useRememberHold = (key: unknown, enabled: boolean, onTap: () => void, onHold: (() => void) | null, label: string): [number, HTMLAttributes<HTMLElement>] => {
  const [progress, setProgress] = useState(0);
  const tap = useUpdatedRef(onTap);
  const hold = useUpdatedRef(onHold);
  const press = useRef<Press | null>(null);
  const stopFill = (current: Press) => {
    if (current.frame !== null) cancelAnimationFrame(current.frame);
    current.frame = null;
  };
  const end = () => {
    const current = press.current;
    if (current !== null) stopFill(current);
    press.current = null;
    setProgress(0);
  };
  useEffect(() => end, [key]);
  const fire = (current: Press) => {
    if (current.fired) return;
    current.fired = true;
    vibrate(LONG_PRESS_HAPTIC_MS);
    hold.current?.();
    end();
  };
  const fill = (current: Press) => {
    const step = () => {
      if (press.current !== current || current.sliding) return;
      const share = Math.min(1, (performance.now() - current.pressedAt) / DECK_HOLD_MS);
      setProgress(share);
      if (share >= 1) return fire(current);
      current.frame = requestAnimationFrame(step);
      return undefined;
    };
    current.frame = requestAnimationFrame(step);
  };
  const start = (pointer: number | null, startX: number) => {
    if (press.current !== null) return;
    const current: Press = { pointer, startX, pressedAt: performance.now(), fired: false, sliding: false, frame: null };
    press.current = current;
    fill(current);
  };
  const release = () => {
    const current = press.current;
    if (current === null) return;
    const quick = performance.now() - current.pressedAt < LONG_PRESS_TIMEOUT_MS;
    end();
    if (!current.fired && !current.sliding && quick) tap.current();
  };
  if (onHold === null || !enabled) {
    return [
      progress,
      {
        role: 'button',
        tabIndex: enabled ? 0 : -1,
        'aria-disabled': !enabled || undefined,
        onClick: enabled ? () => tap.current() : undefined,
        onKeyDown: (event) => {
          if (!enabled || !isActivation(event)) return;
          event.preventDefault();
          tap.current();
        },
      },
    ];
  }
  return [
    progress,
    {
      role: 'button',
      tabIndex: 0,
      'aria-description': label,
      style: { touchAction: 'none' },
      onContextMenu: (event) => event.preventDefault(),
      onClick: (event) => {
        if (event.detail === 0) tap.current();
      },
      onPointerDown: (event: PointerEvent<HTMLElement>) => {
        if (event.button !== 0) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        start(event.pointerId, event.clientX);
      },
      onPointerMove: (event: PointerEvent<HTMLElement>) => {
        const current = press.current;
        if (current === null || current.pointer !== event.pointerId || current.fired) return;
        const slid = event.clientX - current.startX;
        if (!current.sliding && slid > TOUCH_SLOP) {
          current.sliding = true;
          stopFill(current);
        }
        if (!current.sliding) return;
        event.preventDefault();
        const reached = slideProgress(slid, Math.round(event.currentTarget.getBoundingClientRect().width));
        setProgress(reached);
        if (reached >= 1) fire(current);
      },
      onPointerUp: (event: PointerEvent<HTMLElement>) => {
        if (press.current?.pointer === event.pointerId) release();
      },
      onPointerCancel: (event: PointerEvent<HTMLElement>) => {
        if (press.current?.pointer === event.pointerId) end();
      },
      onKeyDown: (event) => {
        if (!isActivation(event) || event.repeat) return;
        event.preventDefault();
        start(null, 0);
      },
      onKeyUp: (event) => {
        if (!isActivation(event) || press.current?.pointer !== null) return;
        event.preventDefault();
        release();
      },
      onBlur: () => {
        if (press.current?.pointer === null) end();
      },
    },
  ];
};

export const HoldFill = ({ progress, color, className }: { progress: number; color: string; className?: string }) =>
  progress > 0 ? <div className={`h-full ${className ?? ''}`} style={{ width: `${progress * 100}%`, background: color }} /> : null;

const DeckPress = ({ entry, children }: { entry: DeckEntry; children: ReactNode }) => {
  const [progress, gesture] = useRememberHold(entry.id, entry.enabled, entry.onClick, entry.onHold, entry.label);
  return (
    <div {...gesture} className="relative flex size-full cursor-pointer items-center justify-center outline-none focus-visible:ring-2 focus-visible:ring-primary">
      <HoldFill progress={progress} color={withAlpha('currentColor', DECK_HOLD_FILL_ALPHA)} className="absolute inset-y-0 start-0" />
      {children}
    </div>
  );
};

export const deckIds = (shown: ReadonlySet<string>, armed: boolean): readonly (readonly [string, boolean])[] => {
  const flying = armed && (shown.has('rtl') || shown.has('land'));
  const ground = [CHECKLIST, shown.has('takeoff') ? 'takeoff' : 'arm'];
  const chosen = (flying ? [PAUSE, 'rtl', 'land'] : ground).filter((it) => shown.has(it));
  const primary = flying ? (chosen.includes('rtl') ? 'rtl' : null) : (chosen[chosen.length - 1] ?? null);
  return chosen.map((it) => [it, it === primary] as const);
};

export const DeckButton = ({ entry, primary, className }: { entry: DeckEntry; primary: boolean; className?: string }) => (
  <Surface
    shape="large"
    color={primary && entry.enabled ? md('primaryContainer') : md('surfaceContainerHigh')}
    contentColor={!entry.enabled ? withAlpha(md('onSurface'), DISABLED_ALPHA) : primary ? md('onPrimaryContainer') : md('onSurface')}
    className={`overflow-hidden ${className ?? ''}`}
    style={{ height: DECK_BUTTON_HEIGHT }}
  >
    <DeckPress entry={entry}>
      <div className="flex flex-col items-center justify-center gap-1">
        <Drawable name={entry.icon} size={DECK_ICON_SIZE} />
        <Text typography="labelLarge" maxLines={1}>
          {entry.label}
        </Text>
      </div>
    </DeckPress>
  </Surface>
);

const RAIL_BUTTON_SIZE = 48;
const RAIL_ICON_SIZE = 24;
const RAIL_PRIMARY_ICON_SIZE = 32;

export const railLabel = (label: string): string => {
  const rest = label.startsWith('Slide to ') ? label.slice('Slide to '.length) : label;
  return rest.charAt(0).toUpperCase() + rest.slice(1);
};

export const RailDeckButton = ({ entry, primary, labelled = false }: { entry: DeckEntry; primary: boolean; labelled?: boolean }) => (
  <div className="flex flex-col items-center">
    <Surface
      shape="full"
      color="transparent"
      contentColor={aircast('outdoorForeground')}
      className="overflow-hidden"
      style={{ width: RAIL_BUTTON_SIZE, height: RAIL_BUTTON_SIZE, opacity: entry.enabled ? 1 : DISABLED_ALPHA }}
    >
      <DeckPress entry={entry}>
        <Drawable name={entry.icon} label={entry.label} size={primary ? RAIL_PRIMARY_ICON_SIZE : RAIL_ICON_SIZE} className={osdShadow()} />
      </DeckPress>
    </Surface>
    {labelled ? (
      <Text typography="labelSmall" color={aircast('outdoorForeground')} maxLines={1} className={osdShadow()}>
        {railLabel(entry.label)}
      </Text>
    ) : null}
  </div>
);

const MORE_COLUMNS = 4;
const MORE_TILE_HEIGHT = 80;

export type MoreTile = { label: string; icon: DrawableName; enabled: boolean; warning: boolean; onClick: () => void };

export const MoreTile = (fields: Omit<MoreTile, 'warning'> & Partial<Pick<MoreTile, 'warning'>>): MoreTile => ({ warning: false, ...fields });

export const guidedIcon = (id: string): DrawableName => {
  switch (id) {
    case 'startMission':
    case 'continueMission':
    case 'resumeMission':
      return 'ic_route';
    case 'cancelRoi':
      return 'ic_close';
    case PAUSE:
      return 'ic_pause';
    case 'landAbort':
      return 'ic_flight_takeoff';
    case 'release':
      return 'ic_download';
    case 'grab':
      return 'ic_upload';
    case 'hold':
      return 'ic_stop_circle';
    case 'vtolTransitionToFixedWing':
    case 'vtolTransitionToMultiRotor':
      return 'ic_swap_horiz';
    case 'forceArm':
      return 'ic_bolt';
    default:
      return 'ic_send';
  }
};

const chunked = <T,>(items: readonly T[], size: number): readonly (readonly T[])[] =>
  Array.from({ length: Math.ceil(items.length / size) }, (_, row) => items.slice(row * size, row * size + size));

export const MoreActionsSheet = ({ tiles, onDismiss, footer }: { tiles: readonly MoreTile[]; onDismiss: () => void; footer: ReactNode }) => (
  <AircastSheet onDismissRequest={onDismiss}>
    <div className="flex flex-col gap-3 overflow-y-auto px-4 pb-6">
      <div className="flex flex-col px-2">
        <Text typography="titleLarge">More actions</Text>
        <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
          Everything that changes what the drone does
        </Text>
      </div>
      {chunked(tiles, MORE_COLUMNS).map((row, at) => (
        <div key={at} className="flex gap-3">
          {row.map((tile) => (
            <Surface
              key={tile.label}
              onClick={() => {
                onDismiss();
                tile.onClick();
              }}
              enabled={tile.enabled}
              shape="large"
              color={md('surfaceContainerHigh')}
              contentColor={tile.warning ? md('error') : md('onSurface')}
              className="flex flex-1 basis-0 flex-col items-center justify-center gap-2 px-1"
              style={{ height: MORE_TILE_HEIGHT, opacity: tile.enabled ? 1 : DISABLED_ALPHA }}
            >
              <Drawable name={tile.icon} size={24} />
              <Text typography="labelMedium" maxLines={2} textAlign="center">
                {sentenceCase(tile.label)}
              </Text>
            </Surface>
          ))}
          {Array.from({ length: MORE_COLUMNS - row.length }, (_, gap) => (
            <div key={`gap-${gap}`} className="flex-1 basis-0" />
          ))}
        </div>
      ))}
      {footer}
    </div>
  </AircastSheet>
);
