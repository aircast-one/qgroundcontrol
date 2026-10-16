import { describe, expect, it } from 'vitest';
import { APM_AIRFRAME_SCREEN } from '@/ui/ApmAirframeScreen';
import { MOTORS, NOT_SUPPORTED_SCREEN, RADIO, REMOTE_SUPPORT, SENSORS, headCanOpen } from '@/ui/SetupPages';
import { attentionAction, parameterCountText, readinessNote, setupSubtitle } from '@/ui/SetupScreen';
import { SYSLINK_SCREEN } from '@/ui/SyslinkScreen';
import { SetupPage, SetupReadiness, isPx4, setupGroups, setupPage, setupReadiness } from './SetupView';

describe('SetupViewTest', () => {
  it('setup wording follows VehicleSummary and SetupView', () => {
    expect(setupReadiness({ ready: false, setupComplete: true })?.setupComplete).toBe(true);
    expect(setupReadiness({ ready: false })?.setupComplete).toBeNull();
  });

  it('a core that declines to judge is not judged as not ready', () => {
    expect(
      setupReadiness({ ready: null, headline: 'No vehicle connected' })?.ready,
      'readiness() answers Option<bool> and returns None when it has no basis for a verdict - its own test says the false that means "checked and not ready" once drew an error heading beside an instruction to connect a vehicle. optBoolean flattens JSON null to false, so the head could not hold that third state at all',
    ).toBeNull();
  });

  it('a verdict that was reached is still carried', () => {
    expect(setupReadiness({ ready: true })?.ready).toBe(true);
    expect(setupReadiness({ ready: false })?.ready).toBe(false);
  });

  const corePages = {
    groups: [
      { title: 'Vehicle', pages: [{ name: 'Summary', native: true }] },
      {
        title: 'Setup',
        pages: [
          { name: 'Sensors', native: true }, { name: 'Radio', native: true },
          { name: 'Frame', native: true }, { name: 'Flight Modes', native: true },
          { name: 'Safety', native: true }, { name: 'Power', native: true },
          { name: 'Motors', native: true }, { name: 'Tuning', native: true },
          { name: 'Camera', native: true }, { name: 'Lights', native: true },
          { name: 'Flight Behavior', native: false },
        ],
      },
      { title: 'Advanced', pages: [{ name: 'Remote Support', native: true }, { name: 'Parameters', native: true }] },
    ],
  };

  it('every page the head names is a page the core offers', () => {
    const offered = new Set(setupGroups(corePages).flatMap((it) => it.pages).map((it) => it.name));
    const claimed = [RADIO, SENSORS, 'Remote Support'];

    expect(claimed.filter((it) => !offered.has(it))).toEqual([]);
  });

  it('a page opens on what this head has, not on a field the core may stop serving', () => {
    const served = { groups: [{ title: 'S', pages: [{ name: 'Radio', parameterSections: false }] }] };
    const page = setupPage(served, RADIO);

    expect(headCanOpen(page, RADIO), 'Radio has a screen here, so no flag from the core decides it').toBe(true);
  });

  it('a page the head has no screen for does not open', () => {
    const summary = SetupPage({ name: 'Summary', parameterSections: false });
    const frame = SetupPage({ name: 'Frame', parameterSections: false });

    expect(headCanOpen(summary, 'Summary'), 'an unnamed page once opened the Remote Support screen, which forwards live position to a third party').toBe(false);
    expect(headCanOpen(frame, 'Frame')).toBe(false);
  });

  it('every native screen the setup page routes to also opens from the list', () => {
    expect(headCanOpen(SetupPage({ name: 'Frame', parameterSections: false, screen: APM_AIRFRAME_SCREEN }), 'Frame')).toBe(true);
    expect(headCanOpen(SetupPage({ name: 'Syslink', parameterSections: false, screen: SYSLINK_SCREEN }), 'Syslink')).toBe(true);
    expect(headCanOpen(SetupPage({ name: 'Failsafes', parameterSections: false, screen: NOT_SUPPORTED_SCREEN }), 'Failsafes')).toBe(true);
  });

  it('Motors opens here now, because the operator asked for it rather than the desktop', () => {
    const motors = SetupPage({ name: 'Motors', parameterSections: false });

    expect(headCanOpen(motors, MOTORS)).toBe(true);
  });

  it('the screens the head does have still open', () => {
    expect(headCanOpen(SetupPage({ name: SENSORS, parameterSections: false }), SENSORS)).toBe(true);
    expect(headCanOpen(SetupPage({ name: RADIO, parameterSections: false }), RADIO)).toBe(true);
    expect(headCanOpen(SetupPage({ name: REMOTE_SUPPORT, parameterSections: false }), REMOTE_SUPPORT)).toBe(true);
    expect(headCanOpen(SetupPage({ name: 'Safety', parameterSections: true }), 'Safety')).toBe(true);
  });

  it('a page the core describes as parameters opens, and no page opens without one', () => {
    expect(headCanOpen(SetupPage({ name: 'Tuning', parameterSections: true }), 'Tuning')).toBe(true);
    expect(headCanOpen(null, SENSORS)).toBe(false);
  });
});

describe('SetupReadinessTest', () => {
  it('no view means no readiness', () => {
    expect(setupReadiness(null)).toBeNull();
  });

  it('readiness comes from the core verbatim', () => {
    const readiness = setupReadiness({ ready: false, headline: '2 components need setup', detail: 'A sensor is unhealthy.' });
    expect(readiness?.ready).toBe(false);
    expect(readiness?.headline).toBe('2 components need setup');
    expect(readiness?.detail).toBe('A sensor is unhealthy.');
  });

  it('a ready vehicle carries no headline', () => {
    const readiness = setupReadiness({ ready: true, headline: '', detail: '' });
    expect(readiness?.ready).toBe(true);
    expect(readiness?.headline).toBe('');
  });
});

describe('SetupFirmwareTest', () => {
  it('the served firmware token answers px4, apm and no vehicle apart', () => {
    const px4 = setupReadiness({ connected: true, firmware: 'px4' });
    const apm = setupReadiness({ connected: true, firmware: 'apm' });
    const none = setupReadiness({ connected: false, firmware: 'none' });

    expect(isPx4(px4)).toBe(true);
    expect(isPx4(apm), 'apm is not px4').toBe(false);
    expect(isPx4(none), 'and no vehicle is not px4 either, which a boolean could not distinguish').toBe(false);
    expect(isPx4(null)).toBe(false);
  });

  it('connected comes from the same view rather than a second read', () => {
    expect(setupReadiness({ connected: true })?.connected).toBe(true);
    expect(setupReadiness({ connected: false })?.connected).toBe(false);
  });

  it('a component needing attention names what to do, calibrating sensors and radio', () => {
    expect(attentionAction('APMRadioComponent')).toBe('Calibrate');
    expect(attentionAction('SensorsComponent')).toBe('Calibrate');
    expect(attentionAction('APMPowerComponent')).toBe('Set up');
  });

  it('the header names firmware then frame and speaks only when the list cannot', () => {
    expect(setupSubtitle('Quadrotor', 'ArduPilot 4.5.7')).toBe('ArduPilot 4.5.7 · Quadrotor');
    const pending = SetupReadiness({ ready: false, setupComplete: false, headline: '1 component needs setup', detail: 'Radio', connected: true, firmware: '' });
    expect(readinessNote(pending, true)).toBeNull();
    expect(readinessNote(pending, false)).toBe('1 component needs setup. Radio');
    expect(readinessNote({ ...pending, setupComplete: true }, false)).toBeNull();
  });

  it('the parameters row counts what the vehicle has, as Penpot', () => {
    expect(parameterCountText(1204)).toBe('1,204 parameters');
    expect(parameterCountText(0)).toBeNull();
  });
});
