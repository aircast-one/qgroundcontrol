import { type KeyboardEvent, useEffect, useRef, useState } from 'react';
import { useLaunchedEffect, useUpdatedRef } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { Text } from '@/m3';
import { AircastSheet } from '@/map/AircastSheet';
import { md, withAlpha } from '@/theme/Colors';
import type { GuidedOffer } from './GuidedActions';
import { guidedCommand } from './MoreActions';

export const HOLD_TO_CONFIRM_MS = 500;
export const HOLD_HELP_MS = 3000;
export const HOLD_TO_CONFIRM_HELP = 'Hold to Confirm';
const DISABLED_HOLD_ALPHA = 0.38;
const HOLD_FILL_ALPHA = 0.35;

export const GRIPPER_ACTIONS: readonly string[] = ['release', 'grab', 'hold'];

export const gripperOffers = (extras: readonly GuidedOffer[]): readonly GuidedOffer[] => GRIPPER_ACTIONS.flatMap((id) => extras.find((it) => it.id === id) ?? []);

type Hold = { readonly timer: ReturnType<typeof setTimeout>; readonly fired: { value: boolean } };

const HOLD_KEYS: ReadonlySet<string> = new Set([' ', 'Enter']);

export const HoldToConfirmButton = ({ text, enabled, className, onActivated }: { text: string; enabled: boolean; className?: string; onActivated: () => void }) => {
  const [pressed, setPressed] = useState(false);
  const [showHelp, setShowHelp] = useState(false);
  const activate = useUpdatedRef(onActivated);
  const hold = useRef<Hold | null>(null);

  useLaunchedEffect([showHelp], async (scope) => {
    if (!showHelp) return;
    await delay(HOLD_HELP_MS);
    if (scope.isActive()) setShowHelp(false);
  });

  useEffect(
    () => () => {
      if (hold.current !== null) clearTimeout(hold.current.timer);
      hold.current = null;
    },
    [],
  );

  const press = () => {
    if (!enabled || hold.current !== null) return;
    setShowHelp(false);
    const fired = { value: false };
    hold.current = {
      fired,
      timer: setTimeout(() => {
        fired.value = true;
        activate.current();
      }, HOLD_TO_CONFIRM_MS),
    };
    setPressed(true);
  };

  const release = () => {
    const current = hold.current;
    if (current === null) return;
    clearTimeout(current.timer);
    hold.current = null;
    setPressed(false);
    setShowHelp(!current.fired.value);
  };

  const onKey = (event: KeyboardEvent, down: boolean) => {
    if (!HOLD_KEYS.has(event.key)) return;
    event.preventDefault();
    if (down && !event.repeat) press();
    if (!down) release();
  };

  return (
    <div
      role="button"
      tabIndex={enabled ? 0 : -1}
      aria-disabled={!enabled}
      onPointerDown={(event) => {
        if (!enabled) return;
        event.currentTarget.setPointerCapture(event.pointerId);
        press();
      }}
      onPointerUp={release}
      onPointerCancel={release}
      onKeyDown={(event) => onKey(event, true)}
      onKeyUp={(event) => onKey(event, false)}
      onBlur={release}
      className={`relative flex h-14 w-full touch-none select-none items-center justify-center overflow-hidden rounded-shape-md bg-secondary-container outline-none focus-visible:ring-2 focus-visible:ring-primary ${className ?? ''}`}
      style={{ opacity: enabled ? 1 : DISABLED_HOLD_ALPHA }}
    >
      <div
        className="absolute inset-0 origin-left"
        style={{ transform: `scaleX(${pressed ? 1 : 0})`, transition: pressed ? `transform ${HOLD_TO_CONFIRM_MS}ms linear` : 'none', background: withAlpha(md('primary'), HOLD_FILL_ALPHA) }}
      />
      <div className="relative flex flex-col items-center">
        <Text typography="labelLarge" color={md('onSecondaryContainer')}>
          {text}
        </Text>
        {showHelp ? (
          <Text typography="labelSmall" color={md('onSecondaryContainer')}>
            {HOLD_TO_CONFIRM_HELP}
          </Text>
        ) : null}
      </div>
    </div>
  );
};

export const GripperPanel = ({ offers, onDismiss }: { offers: readonly GuidedOffer[]; onDismiss: () => void }) => (
  <AircastSheet onDismissRequest={onDismiss}>
    <div className="flex w-full flex-col gap-2 px-sheet pb-6">
      {offers.map((offer) => (
        <HoldToConfirmButton
          key={offer.id}
          text={offer.title}
          enabled={offer.ready}
          onActivated={() => {
            guidedCommand(offer.id, null)?.();
            onDismiss();
          }}
        />
      ))}
    </div>
  </AircastSheet>
);
