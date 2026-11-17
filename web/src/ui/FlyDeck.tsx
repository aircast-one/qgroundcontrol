import { type CSSProperties, type HTMLAttributes, type KeyboardEvent, type PointerEvent, type ReactNode, createContext, useContext, useEffect, useRef, useState } from 'react';
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
import { PAUSE, RELEASE } from '@/ui/MoreActions';
import { OSD_CONTROL_SIZE, OSD_SCRIM, osdShadow } from '@/ui/Osd';

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
  short: string | null;
};

export const DeckEntry = (fields: Omit<DeckEntry, 'warning' | 'onHold' | 'short'> & Partial<Pick<DeckEntry, 'warning' | 'onHold' | 'short'>>): DeckEntry => ({
  warning: false,
  onHold: null,
  short: null,
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
        'aria-label': label,
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
      'aria-label': label,
      style: { touchAction: 'none' },
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
    <div
      {...gesture}
      className="relative flex size-full cursor-pointer items-center justify-center rounded-[inherit] outline-none focus-visible:outline-2 focus-visible:outline-solid focus-visible:outline-offset-2 focus-visible:outline-[var(--md-primary)]"
    >
      <div className="pointer-events-none absolute inset-0 overflow-hidden rounded-[inherit]">
        <HoldFill progress={progress} color={withAlpha('currentColor', DECK_HOLD_FILL_ALPHA)} className="absolute inset-y-0 start-0" />
      </div>
      {children}
    </div>
  );
};

export type DeckIds = readonly (readonly [string, boolean])[];

export const deckIds = (shown: ReadonlySet<string>, armed: boolean, payloadStrip: boolean): DeckIds => {
  const flying = armed && (shown.has('rtl') || shown.has('land'));
  const ground = [CHECKLIST, shown.has('takeoff') ? 'takeoff' : 'arm'];
  const flown = (flying ? [PAUSE, 'rtl', 'land'] : ground).filter((it) => shown.has(it));
  const primary = flying ? (flown.includes('rtl') ? 'rtl' : null) : (flown[flown.length - 1] ?? null);
  const chosen = flying && shown.has(RELEASE) && !payloadStrip ? [...flown, RELEASE] : flown;
  return chosen.map((it) => [it, it === primary] as const);
};

const DECK_GAP = 10;
const DECK_MORE_WIDTH = 64;
const DECK_MIN_WIDTH = 56;
const RAIL_GAP = 8;
const RAIL_LABEL_HEIGHT = 20;
export const RAIL_STOP_GAP = 12;
const DECK_PRIORITY: readonly string[] = ['rtl', 'land', 'takeoff', 'arm', RELEASE, PAUSE, CHECKLIST];

const priority = (id: string): number => (DECK_PRIORITY.includes(id) ? DECK_PRIORITY.indexOf(id) : DECK_PRIORITY.length);

const fitting = (deck: DeckIds, room: number, slot: number, fixed: number): DeckIds => {
  const kept = new Set(
    deck
      .map(([id]) => id)
      .toSorted((a, b) => priority(a) - priority(b))
      .slice(0, Math.max(0, Math.floor((room - fixed) / slot))),
  );
  return room <= 0 ? deck : deck.filter(([id]) => kept.has(id));
};

export const bottomDeck = (deck: DeckIds, width: number, centre: number): { deck: DeckIds; centreInline: boolean } => {
  const centreInline = width <= 0 || centre <= 0 || deck.length * (DECK_MIN_WIDTH + DECK_GAP) + DECK_MORE_WIDTH + centre + DECK_GAP <= width;
  return { deck: fitting(deck, width, DECK_MIN_WIDTH + DECK_GAP, DECK_MORE_WIDTH + (centreInline && centre > 0 ? centre + DECK_GAP : 0)), centreInline };
};

const railLabelRoom = (labelled: boolean): number => (labelled ? RAIL_LABEL_HEIGHT : 0);

export const railDeck = (deck: DeckIds, height: number): { deck: DeckIds; labelled: boolean } => {
  const slot = (labelled: boolean) => OSD_CONTROL_SIZE + railLabelRoom(labelled) + RAIL_GAP;
  const stop = (labelled: boolean) => OSD_CONTROL_SIZE + railLabelRoom(labelled) + RAIL_STOP_GAP;
  const labelled = height <= 0 || deck.length * slot(true) + stop(true) <= height;
  return { deck: fitting(deck, height, slot(labelled), stop(labelled)), labelled };
};

export const CROWDED_DECK = 5;

export const deckLabel = (label: string, buttons: number): string => (buttons >= CROWDED_DECK ? railLabel(label) : label);

export const deckColours = (entry: DeckEntry, primary: boolean): readonly [string, string] => {
  if (!entry.enabled) return [md('surfaceContainerHigh'), withAlpha(md('onSurface'), DISABLED_ALPHA)];
  if (entry.warning) return [md('error'), md('onError')];
  return primary ? [md('primaryContainer'), md('onPrimaryContainer')] : [md('surfaceContainerHigh'), md('onSurface')];
};

export const DeckButton = ({ entry, primary, className, style, buttons = 0 }: { entry: DeckEntry; primary: boolean; className?: string; style?: CSSProperties; buttons?: number }) => (
  <Surface shape="large" color={deckColours(entry, primary)[0]} contentColor={deckColours(entry, primary)[1]} className={className} style={{ ...style, height: DECK_BUTTON_HEIGHT }}>
    <DeckPress entry={entry}>
      <div className="flex min-w-0 flex-col items-center justify-center gap-1 px-1">
        <Drawable name={entry.icon} size={DECK_ICON_SIZE} />
        <Text typography="labelLarge" maxLines={2} textAlign="center">
          {deckLabel(entry.label, buttons)}
        </Text>
      </div>
    </DeckPress>
  </Surface>
);

const RAIL_ICON_SIZE = 24;
const RAIL_PRIMARY_ICON_SIZE = 28;
const RAIL_INSET_ICON_SIZE = 20;

export const railLabel = (label: string): string => {
  const rest = label.startsWith('Slide to ') ? label.slice('Slide to '.length) : label;
  return rest.charAt(0).toUpperCase() + rest.slice(1);
};

export const RailDeckButton = ({ entry, primary, labelled = false }: { entry: DeckEntry; primary: boolean; labelled?: boolean }) => {
  const label = entry.short ?? railLabel(entry.label);
  return (
    <div className="flex flex-col items-center gap-1" style={{ width: OSD_CONTROL_SIZE }}>
      <Surface shape="full" color="transparent" contentColor={entry.warning ? md('error') : aircast('outdoorForeground')} className="shrink-0" style={{ minWidth: OSD_CONTROL_SIZE, height: OSD_CONTROL_SIZE, opacity: entry.enabled ? 1 : DISABLED_ALPHA }}>
        <DeckPress entry={entry}>
          {labelled ? (
            <Drawable name={entry.icon} label={entry.label} size={primary ? RAIL_PRIMARY_ICON_SIZE : RAIL_ICON_SIZE} className={osdShadow()} />
          ) : (
            <span className={`flex flex-col items-center px-2 ${osdShadow()}`}>
              <Drawable name={entry.icon} label={entry.label} size={RAIL_INSET_ICON_SIZE} />
              <span aria-hidden>
                <Text typography="labelSmall" maxLines={1}>
                  {label}
                </Text>
              </span>
            </span>
          )}
        </DeckPress>
      </Surface>
      {labelled ? (
        <span className="w-max max-w-[72px] shrink-0 rounded-shape-sm px-1.5 text-center" style={{ background: OSD_SCRIM }}>
          <Text typography="labelMedium" color={aircast('outdoorForeground')}>
            {label}
          </Text>
        </span>
      ) : null}
    </div>
  );
};

const MENU_COLUMNS = 4;
const MENU_TILE_HEIGHT = 80;

export type MenuTile = { label: string; icon: DrawableName; enabled: boolean; warning: boolean; onClick: () => void };

export const MenuTile = (fields: Omit<MenuTile, 'warning'> & Partial<Pick<MenuTile, 'warning'>>): MenuTile => ({ warning: false, ...fields });

export type MenuSection = { title: string; tiles: readonly MenuTile[] };

export const LocalFlyDestinations = createContext<readonly MenuTile[] | null>(null);

export const useFlyDestinations = (): readonly MenuTile[] => {
  const destinations = useContext(LocalFlyDestinations);
  if (destinations === null) throw new Error('the Fly menu is mounted outside LocalFlyDestinations');
  return destinations;
};

export const FLY_MENU = 'Menu';

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
    case RELEASE:
      return 'ic_drop';
    case 'grab':
      return 'ic_payload';
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

const TileGrid = ({ tiles, onDismiss }: { tiles: readonly MenuTile[]; onDismiss: () => void }) => (
  <div className="flex flex-col gap-3">
    {chunked(tiles, MENU_COLUMNS).map((row, at) => (
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
            style={{ height: MENU_TILE_HEIGHT, opacity: tile.enabled ? 1 : DISABLED_ALPHA }}
          >
            <Drawable name={tile.icon} size={24} />
            <Text typography="labelMedium" maxLines={2} textAlign="center">
              {sentenceCase(tile.label)}
            </Text>
          </Surface>
        ))}
        {Array.from({ length: MENU_COLUMNS - row.length }, (_, gap) => (
          <div key={`gap-${gap}`} className="flex-1 basis-0" />
        ))}
      </div>
    ))}
  </div>
);

export const FlyMenuSheet = ({ sections, onDismiss, footer }: { sections: readonly MenuSection[]; onDismiss: () => void; footer: ReactNode }) => (
  <AircastSheet onDismissRequest={onDismiss} label={FLY_MENU}>
    <div className="flex flex-col gap-4 overflow-y-auto px-4 pb-6">
      <Text typography="titleLarge" className="px-2">
        {FLY_MENU}
      </Text>
      {sections
        .filter((section) => section.tiles.length > 0)
        .map((section) => (
          <section key={section.title} aria-label={section.title} className="flex flex-col gap-2">
            <Text typography="labelLarge" color={md('onSurfaceVariant')} className="px-2">
              {section.title}
            </Text>
            <TileGrid tiles={section.tiles} onDismiss={onDismiss} />
          </section>
        ))}
      {footer}
    </div>
  </AircastSheet>
);
