import { describe, expect, it } from 'vitest';
import { Tab } from '@/app/MainActivity';
import { FlyView } from '@/ui/FlyShell';
import { DEBUG_COMMANDS, type DebugResult, debugCommand } from './DebugUi';

const command = (result: DebugResult) => (result.ok ? result.command : null);
const message = (result: DebugResult) => (result.ok ? null : result.message);

describe('DebugUiTest', () => {
  it('each command parses to what the app applies', () => {
    expect(command(debugCommand('tab', 'plan'))).toEqual({ kind: 'ShowTab', tab: Tab.Plan });
    expect(command(debugCommand('fly-view', 'Video'))).toEqual({ kind: 'ShowFlyView', view: FlyView.Video });
    expect(command(debugCommand('orientation', 'landscape'))).toEqual({ kind: 'Orient', orientation: 'landscape-primary' });
    expect(command(debugCommand('layout-edit', 'off'))).toEqual({ kind: 'EditLayout', on: false });
    expect(command(debugCommand('layout-reset', null))).toEqual({ kind: 'ResetLayout' });
    expect(command(debugCommand('open', 'takeoff'))).toEqual({ kind: 'Open', target: 'takeoff' });
  });

  it('a bad command names what it accepts instead of guessing', () => {
    expect(message(debugCommand('rotate', null))).toBe(`cmd must be one of ${DEBUG_COMMANDS}`);
    expect(message(debugCommand('orientation', 'sideways'))).toBe('orientation must be one of landscape, reverse-landscape, portrait, auto');
    expect(message(debugCommand('tab', 'Home'))).toBe('tab must be one of Fly, Plan, Analyze');
    expect(message(debugCommand('open', ''))).toBe('open needs a sheet (more, readings, camera, gimbal, status, status-all, modes, traffic, settings) or a flight action id, e.g. takeoff, rtl, land');
  });
});
