import { describe, expect, it } from 'vitest';
import type { Json } from '@/bridge/Json';
import { NOT_SUPPORTED_SCREEN } from '@/ui/SetupPages';
import {
  PARAMETERS_STOPPED,
  SetupComponent,
  disabledWhile,
  firmwareLine,
  parameterWait,
  parametersIncomplete,
  prerequisiteText,
  remainingSetup,
  reportsOpening,
  setupComponents,
  setupMatches,
  firstToOpen,
} from '@/ui/SetupScreen';
import { SetupPage, parametersReady } from '@/ui/SetupView';

describe('SetupScreenTest', () => {
  it('the firmware line is the one the core spelled', () => {
    const line = firmwareLine({ kind: 'object', available: true, summary: 'PX4 Pro 1.15.0 beta', vehicleType: 'Quadrotor' });
    expect(line).toEqual({ summary: 'PX4 Pro 1.15.0 beta', vehicleType: 'Quadrotor' });
  });

  it('the screen reports opening unless a parameter form does it, and always while a prerequisite blocks the page', () => {
    expect(reportsOpening(null, SetupPage({ name: 'Sensors', parameterSections: false }))).toBe(true);
    expect(reportsOpening(null, SetupPage({ name: 'Heli', parameterSections: true }))).toBe(false);
    expect(reportsOpening('Radio', SetupPage({ name: 'Flight Modes', parameterSections: false }))).toBe(true);
    expect(reportsOpening('Airframe', SetupPage({ name: 'Power', parameterSections: true }))).toBe(true);
    expect(reportsOpening(null, SetupPage({ name: 'Flight Safety', parameterSections: false, screen: NOT_SUPPORTED_SCREEN }))).toBe(false);
  });

  it('parameters read ready only on a connected vehicle that says so', () => {
    expect(parametersReady({ connected: true, parametersReady: true })).toBe(true);
    expect(parametersReady({ connected: false, parametersReady: true })).toBe(false);
    expect(parametersReady({ connected: true, parametersReady: false })).toBe(false);
    expect(parametersReady(null)).toBe(false);
  });

  it('no vehicle reads as empty rather than stray separators', () => {
    expect(firmwareLine(null)).toEqual({ summary: '', vehicleType: '' });
    expect(firmwareLine({ kind: 'object', available: false, summary: '', vehicleType: '' })).toEqual({ summary: '', vehicleType: '' });
  });

  it('a component is read with the verdicts the core reached', () => {
    const read = setupComponents({
      components: [
        { name: 'Radio', needsAttention: true, blockedReason: 'armed' },
        { name: 'Camera', needsAttention: false, blockedReason: null },
      ],
    });

    expect(read.map((it) => it.name)).toEqual(['Radio', 'Camera']);
    expect(read.map((it) => it.needsAttention)).toEqual([true, false]);
    expect(read.map((it) => it.blockedReason)).toEqual(['armed', null]);
  });

  it('a blocked page says why like SetupPage', () => {
    expect(disabledWhile('flying')).toBe('Disabled while the vehicle is flying');
  });

  it('a component with no name is not offered, and no view is no components', () => {
    expect(setupComponents({ components: [{ name: 'Radio' }, {}] })).toHaveLength(1);
    expect(setupComponents(null).map((it) => it.name)).toEqual([]);
  });

  it('a component promoted for attention is not listed a second time', () => {
    const components = [
      SetupComponent({ index: 0, name: 'Frame', needsAttention: false }),
      SetupComponent({ index: 1, name: 'Sensors', needsAttention: true }),
      SetupComponent({ index: 2, name: 'Power', needsAttention: false }),
    ];

    expect(remainingSetup(components).map((it) => it.name)).toEqual(['Frame', 'Power']);
  });

  it('every component needing attention leaves nothing for the full list', () => {
    const components = [SetupComponent({ index: 0, name: 'Sensors', needsAttention: true })];

    expect(remainingSetup(components).map((it) => it.name)).toEqual([]);
  });

  it('a parameter load that has stopped is not drawn as one still running', () => {
    const view = (reason: string, ready: boolean): Json => ({
      parametersReady: ready,
      parametersReason: reason,
      parametersText: 'This vehicle has not answered the request for its parameters, and the retries are finished.',
    });

    expect(parameterWait(view('loading', false))?.title).toBe('Loading parameters from the vehicle.');
    expect(parameterWait(view('loading', false))?.body).toBe('');

    const stopped = parameterWait(view('unanswered', false));
    expect(stopped?.title, 'the core states what the vehicle did').toBe('This vehicle has not answered the request for its parameters, and the retries are finished.');
    expect(stopped?.body, 'and this head owns the only thing an operator can act on from here').toBe('Setup needs them. Disconnect and connect the link to ask again.');

    expect(parameterWait(view('', true)), 'a ready vehicle waits for nothing').toBeNull();
    expect(parameterWait(view('noVehicle', false))).toBeNull();
    expect(parameterWait(null), 'the screen already says to connect a vehicle').toBeNull();
  });

  it('a reason this head has never heard of reads as stopped, not as ready', () => {
    const waiting = parameterWait({ parametersReady: false, parametersReason: 'refused', parametersText: 'The vehicle refused the request.' });
    expect(waiting?.title).toBe('The vehicle refused the request.');
    expect(
      waiting?.body,
      'ready would hide a dead load behind a normal screen and loading is the defect being fixed, so an unfamiliar state is closer to stopped than to either',
    ).toBe(PARAMETERS_STOPPED);
  });

  it('a reason with no sentence beside it still says something true', () => {
    expect(parameterWait({ parametersReady: false, parametersReason: 'refused', parametersText: '' })?.title).toBe('This vehicle has not sent its parameters (refused).');
  });

  it('a download skipped in flight offers to fetch the parameters', () => {
    const wait = parameterWait({ parametersReady: false, parametersReason: 'skipped', parametersText: 'Parameter download was skipped because the vehicle is flying.' });
    expect(wait?.downloadOffered).toBe(true);
    expect(wait?.title).toBe('Parameter download was skipped because the vehicle is flying.');
  });

  it('a component with an unfinished prerequisite says which one', () => {
    const view = { components: [{ name: 'Flight Modes', needsAttention: true, blockedReason: null, prerequisite: 'Frame' }] };
    expect(setupComponents(view)[0]?.prerequisite).toBe('Frame');
    expect(prerequisiteText('Frame', 'Flight Modes')).toBe('Frame has to be set up before Flight modes.');
  });

  it('the setup search narrows pages by name, ignoring case and spaces', () => {
    expect(setupMatches('Flight Modes', '  flight ')).toBe(true);
    expect(setupMatches('Radio', '')).toBe(true);
    expect(setupMatches('Radio', 'sensor')).toBe(false);
  });

  it("a vehicle that withheld parameters reads as SetupView's Parameters Incomplete", () => {
    const view = { parametersReady: true, parametersReason: 'incomplete', parametersText: "The vehicle didn't return its full parameter list, so some setup options are unavailable." };
    expect(parametersIncomplete(view)).toBe("Parameters Incomplete. The vehicle didn't return its full parameter list, so some setup options are unavailable.");
    expect(parametersIncomplete({ parametersReady: true, parametersReason: '' })).toBeNull();
    expect(parameterWait(view), 'ready parameters never block setup').toBeNull();
  });

  it('setup opens on the first component that still needs setting up, not the first in the list', () => {
    const components = [
      SetupComponent({ index: 0, name: 'ESC', needsAttention: false }),
      SetupComponent({ index: 1, name: 'Pre-Arm Check', needsAttention: true }),
      SetupComponent({ index: 2, name: 'Sensors', needsAttention: true }),
    ];
    expect(firstToOpen(components, () => true)?.name).toBe('Pre-Arm Check');
    expect(firstToOpen(components, (it) => it.name !== 'Pre-Arm Check')?.name).toBe('Sensors');
    expect(firstToOpen(components.slice(0, 1), () => true)?.name).toBe('ESC');
    expect(firstToOpen([], () => true)).toBeUndefined();
  });
});
