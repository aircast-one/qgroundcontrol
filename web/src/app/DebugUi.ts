import { type Tab, TabEntries } from '@/app/MainActivity';
import { type FlyView, FlyViewEntries } from '@/ui/FlyShell';
import { REQUESTABLE_SHEETS } from '@/ui/FlyScreenState';

export const DEBUG_UI_ACTION = 'one.aircast.android.DEBUG_UI';

export type DebugCommand =
  | { kind: 'ShowTab'; tab: Tab }
  | { kind: 'ShowFlyView'; view: FlyView }
  | { kind: 'Orient'; orientation: OrientationLockType }
  | { kind: 'EditLayout'; on: boolean }
  | { kind: 'ResetLayout' }
  | { kind: 'Open'; target: string };

export type DebugResult = { ok: true; command: DebugCommand } | { ok: false; message: string };

const ORIENTATIONS: ReadonlyMap<string, OrientationLockType> = new Map([
  ['landscape', 'landscape-primary'],
  ['reverse-landscape', 'landscape-secondary'],
  ['portrait', 'portrait-primary'],
  ['auto', 'any'],
]);

const SWITCHES: ReadonlyMap<string, boolean> = new Map([
  ['on', true],
  ['off', false],
]);

export const DEBUG_COMMANDS = 'state, tab, fly-view, orientation, layout-edit, layout-reset, open';

const success = (command: DebugCommand): DebugResult => ({ ok: true, command });

const failure = (message: string): DebugResult => ({ ok: false, message });

const named = <T,>(value: string | null, choices: ReadonlyMap<string, T>, what: string, made: (choice: T) => DebugCommand): DebugResult => {
  const found = [...choices].find(([key]) => value !== null && key.toLowerCase() === value.toLowerCase());
  return found ? success(made(found[1])) : failure(`${what} must be one of ${[...choices.keys()].join(', ')}`);
};

const byName = <T extends { name: string }>(entries: readonly T[]): ReadonlyMap<string, T> => new Map(entries.map((it) => [it.name, it]));

export const debugCommand = (cmd: string | null, value: string | null): DebugResult => {
  switch (cmd) {
    case 'tab': return named(value, byName(TabEntries), 'tab', (tab) => ({ kind: 'ShowTab', tab }));
    case 'fly-view': return named(value, byName(FlyViewEntries), 'fly-view', (view) => ({ kind: 'ShowFlyView', view }));
    case 'orientation': return named(value, ORIENTATIONS, 'orientation', (orientation) => ({ kind: 'Orient', orientation }));
    case 'layout-edit': return named(value, SWITCHES, 'layout-edit', (on) => ({ kind: 'EditLayout', on }));
    case 'layout-reset': return success({ kind: 'ResetLayout' });
    case 'open':
      return value !== null && value.trim() !== ''
        ? success({ kind: 'Open', target: value })
        : failure(`open needs a sheet (${[...REQUESTABLE_SHEETS, 'settings'].join(', ')}) or a flight action id, e.g. takeoff, rtl, land`);
    default: return failure(`cmd must be one of ${DEBUG_COMMANDS}`);
  }
};

type Collector = (command: DebugCommand) => void;

const collectors = new Set<Collector>();

export const DebugUi = {
  commands: {
    tryEmit(command: DebugCommand): boolean {
      [...collectors].map((collector) => collector(command));
      return true;
    },
    collect(collector: Collector): () => void {
      collectors.add(collector);
      return () => collectors.delete(collector);
    },
  },
  state: '{}',
};

export class DebugUiReceiver {
  onReceive(cmd: string | null, value: string | null): string {
    if (cmd === 'state') return DebugUi.state;
    const parsed = debugCommand(cmd, value);
    if (!parsed.ok) return `error: ${parsed.message}`;
    return DebugUi.commands.tryEmit(parsed.command) ? 'ok' : 'error: busy, retry';
  }
}
