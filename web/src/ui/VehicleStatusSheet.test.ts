import { describe, expect, it } from 'vitest';
import { FlyState } from './FlyState';
import { GuidedOffer } from './GuidedActions';
import { ARM_UNAVAILABLE, ArmControls, armControls, deckRequestRefusal, expandedAfterTap, messagesToggleText, shownSensors } from './VehicleStatusSheet';
import { ArmingCheck } from './VehicleMessages';

describe('VehicleStatusSheetTest', () => {
  it('the status sheet lists faulty sensors until asked for the rest', () => {
    const sensors = [
      { name: 'Gyro', state: 'unhealthy', label: 'Error' },
      { name: 'Baro', state: 'healthy', label: 'Normal' },
    ];
    expect(shownSensors(sensors, false).map((it) => it.name)).toEqual(['Gyro']);
    expect(shownSensors(sensors, true).map((it) => it.name)).toEqual(['Gyro', 'Baro']);
  });

  const state = ({ armed = false, nominal = true, fault = false, canArm = true }: { armed?: boolean; nominal?: boolean; fault?: boolean; canArm?: boolean } = {}) =>
    FlyState({
      connected: true,
      armed,
      contactLost: false,
      state: '',
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

  it('the messages card starts collapsed and toggles like the MainStatusIndicator disclosure', () => {
    expect(messagesToggleText(false)).toBe('Show messages');
    expect(messagesToggleText(true)).toBe('Hide messages');
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
});
