import type { ReactNode } from 'react';
import type { Scope } from '@/kotlin/Compose';

export const VIRTUAL_JOYSTICK_PATH = 'view.virtualJoystick';

export const VirtualStickSender = {
  start: (_scope: Scope): void => undefined,
};

export const VirtualJoystick = (_props: { pad: number; className?: string; trailing?: ReactNode }) => null;

export const useShownStickPad = (_shortSide: number, _room: number = Number.POSITIVE_INFINITY): number | null => null;
