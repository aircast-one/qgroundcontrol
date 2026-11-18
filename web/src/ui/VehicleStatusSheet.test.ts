import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { FlyState, flyState } from './FlyState';
import { GuidedOffer } from './GuidedActions';
import { ARM_UNAVAILABLE, ArmControls, armControls, armSlideShown, deckRequestRefusal, expandedAfterTap, shownSensors, statusSummary } from './VehicleStatusSheet';
import { ArmingCheck } from './VehicleMessages';

describe('VehicleStatusSheetTest', () => {
  it('the status sheet lists only faulty sensors until asked for the rest, so a switched-off one does not read as a fault', () => {
    const sensors = [
      { name: 'Gyro', state: 'unhealthy', label: 'Error' },
      { name: 'Baro', state: 'healthy', label: 'Normal' },
      { name: 'Geofence', state: 'disabled', label: 'Disabled' },
    ];
    expect(shownSensors(sensors, false).map((it) => it.name)).toEqual(['Gyro']);
    expect(shownSensors(sensors, true).map((it) => it.name)).toEqual(['Gyro', 'Baro', 'Geofence']);
  });

  const state = ({ armed = false, nominal = true, fault = false, canArm = true, phase = '', connected = true }: { armed?: boolean; nominal?: boolean; fault?: boolean; canArm?: boolean; phase?: string; connected?: boolean } = {}) =>
    FlyState({
      connected,
      armed,
      contactLost: false,
      state: phase,
      stateText: '',
      staleNotice: '',
      mode: '',
      rcSupported: false,
      rcSignalText: '',
      rcSignal: null,
      rcOverride: null,
      telemetry: null,
      nominal,
      fault,
      canArm,
    });

  it('the arm slide leaves the sheet while the aircraft flies or lands, where it would cut the motors', () => {
    expect(['flying', 'landing'].map((phase) => armSlideShown(state({ armed: true, phase })))).toEqual([false, false]);
    expect(['armed', 'disarmed'].map((phase) => armSlideShown(state({ armed: phase === 'armed', phase })))).toEqual([true, true]);
    expect(armSlideShown(state({ connected: false, phase: 'disarmed' })), 'nothing to arm without a vehicle').toBe(false);
    expect(armSlideShown(null)).toBe(false);
  });

  it("the status drawer's arm controls follow MainStatusIndicator", () => {
    expect(armControls(state(), false)).toEqual(ArmControls('Slide to arm', true, false, false, false));
    expect(armControls(state({ armed: true }), false)).toEqual(ArmControls('Slide to disarm', true, false, false, false));
    expect(armControls(state({ nominal: false }), false)).toEqual(ArmControls('Slide to arm', true, true, false, false));
    expect(armControls(state({ nominal: false, fault: true, canArm: false }), false)).toEqual(ArmControls('Slide to arm', false, false, true, false));
    expect(armControls(state({ canArm: false }), true)).toEqual(ArmControls('Slide to arm', false, false, false, true));
  });

  it('a slide the deck cannot act on says why instead of closing silently', () => {
    const blocked = GuidedOffer({ id: 'arm', title: 'Arm', offer: 'disabled', reason: 'Complete the preflight checklist first', prompt: '', destructive: false, carriesValue: false });
    expect(deckRequestRefusal(blocked)).toBe('Complete the preflight checklist first');
    expect(deckRequestRefusal(null)).toBe(ARM_UNAVAILABLE);
  });

  it('a check with a description toggles open and shut, one without stays shut, like the Overall Status delegate', () => {
    const detailed = ArmingCheck('GPS not ready', 'Wait for a fix', 'error');
    const bare = ArmingCheck('Battery low', '', 'warning');
    expect(expandedAfterTap(new Set(), 0, detailed)).toEqual(new Set([0]));
    expect(expandedAfterTap(new Set([0]), 0, detailed)).toEqual(new Set());
    expect(expandedAfterTap(new Set(), 1, bare)).toEqual(new Set());
  });

  it('a status line that names a payload problem carries the one-tap fix the payload panel offers for it', () => {
    const OPEN = 'The gripper stays open after the first drop, so the other 3 drops release nothing. Turn on auto-close so it closes after each drop.';
    const view = (problem: string) => flyState(parseJson(`{"class":"FlyState","connected":true,"stateText":"Not Fully Ready","summaryDetail":"${OPEN}","summaryProblem":${problem}}`));

    expect(statusSummary(view(`{"code":"staysOpen","text":"${OPEN}","blocking":true,"fix":"Turn on auto-close"}`))?.problem).toEqual({ code: 'staysOpen', text: OPEN, blocking: true, fix: 'Turn on auto-close' });
    expect(statusSummary(view('null'))).toEqual({ title: 'Not fully ready', detail: OPEN, problem: null });
    expect(statusSummary(flyState(parseJson('{"class":"FlyState","summaryDetail":""}')))).toBeNull();
  });
});
