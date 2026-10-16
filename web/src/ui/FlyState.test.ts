import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { HOLD_TO_TAKE_OFF, NO_LINK, NO_SIGNAL_CODE, takeoffLabel } from './FlyDeck';
import { FlyView, flyViewShown } from './FlyShell';
import {
  ALL_CHECKS_PASSED,
  ChipTone,
  FlyState,
  READINESS_BLOCKED,
  Readiness,
  chipTone,
  disarmNotice,
  flownDistanceText,
  flyState,
  guidedReadiness,
  landedSummary,
  offlineMainStatus,
  readinessWarning,
  vehicleSubtitle,
} from './FlyState';
import { GuidedOffer } from './GuidedActions';

const json = (text: string): Json => parseJson(text) ?? {};

describe('FlyStateTest', () => {
  it('a link nobody is monitoring is not reported as having contact', () => {
    const unknown = flyState(json('{"kind":"object","class":"FlyState","connected":true,"contactLost":null}'));
    expect(
      unknown?.contactLost,
      'flystate.rs serves contactLost as an Option and says so: communicationLostEnabled ' +
        'off means unknown, and unknown is not evidence of a loss. optBoolean flattens ' +
        'JSON null to false, which turns "nobody is watching this link" into "contact ' +
        'is fine" - the direction that reassures',
    ).toBeNull();
    expect(flyState(json('{"kind":"object","class":"FlyState","contactLost":false}'))?.contactLost).toBe(false);
    expect(flyState(json('{"kind":"object","class":"FlyState","contactLost":true}'))?.contactLost).toBe(true);
  });

  const inContact = json(`{"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
      "contactLost":false,"state":"disarmed","stateText":"Stabilize · Disarmed",
      "staleNotice":"","mode":"Stabilize"}`);

  const lost = json(`{"class":"FlyState","connected":true,"armed":false,"flying":false,"landing":false,
      "contactLost":true,"state":"contactLost","stateText":"Communication lost",
      "staleNotice":"No contact — these are the last values the vehicle sent.",
      "mode":"Stabilize"}`);

  it('the sentence and the token come from the core, not from the head', () => {
    const state = flyState(inContact);

    expect(state?.state).toBe('disarmed');
    expect(state?.stateText).toBe('Stabilize · Disarmed');
    expect(flyState(json('{"kind":"object","class":"FlyState","connected":true,"stateText":"Ready to Fly"}'))?.stateText).toBe('Ready to fly');
  });

  it('a vehicle in contact carries no notice, so nothing dims', () => {
    const state = flyState(inContact);

    expect(state?.contactLost).toBe(false);
    expect(state?.staleNotice).toBe('');
  });

  it('lost contact carries the notice and the head shows it verbatim', () => {
    const state = flyState(lost);

    expect(state?.contactLost).toBe(true);
    expect(state?.stateText).toBe('Communication lost');
    expect(state?.staleNotice).toBe('No contact — these are the last values the vehicle sent.');
  });

  it('a silent aircraft makes no readiness claims, and take off says why it cannot', () => {
    const silentButUnready = { ...lost, nominal: false, summaryDetail: 'GPS turned off. Everything else reports normal.' };
    expect(readinessWarning(flyState(silentButUnready))).toBeNull();
    const noSignal = GuidedOffer({ id: 'takeoff', title: 'Takeoff', offer: 'hidden', reason: 'Any wording the core chooses.', prompt: '', destructive: false, carriesValue: true, reasonCode: NO_SIGNAL_CODE });
    expect(takeoffLabel(noSignal, null)).toBe(NO_LINK);
    expect(takeoffLabel(GuidedOffer({ ...noSignal, offer: 'ready', reason: '', reasonCode: '' }), null)).toBe(HOLD_TO_TAKE_OFF);
  });

  it("the notice uses the core's dash, not an ascii hyphen", () => {
    expect(flyState(lost)?.staleNotice.includes('—')).toBe(true);
    expect(flyState(lost)?.staleNotice.includes(' - ')).toBe(false);
  });

  it('a payload that is not the fly state yields nothing', () => {
    expect(flyState(null)).toBeNull();
    expect(flyState(json('{}'))).toBeNull();
    expect(flyState(json('{"class":"Calibration"}'))).toBeNull();
  });

  it('a payload using an invented name for the notice leaves it blank', () => {
    const invented = json('{"class":"FlyState","contactLost":true,"stale":"No contact","notice":"No contact"}');

    expect(flyState(invented)?.staleNotice).toBe('');
  });
});

describe('VehicleSubtitleTest', () => {
  const state = ({ connected = true, contactLost = false, stateText = 'Disarmed', mode = 'Stabilize' }: { connected?: boolean; contactLost?: boolean; stateText?: string; mode?: string } = {}) =>
    FlyState({ connected, armed: false, contactLost, state: 'disarmed', stateText, staleNotice: '', mode, rcSupported: false, rcSignalText: '', rcSignal: null, rcOverride: null, telemetry: null });

  it('the header keeps the flight mode the core reports separately', () => {
    expect(vehicleSubtitle(state())).toBe('Stabilize · Disarmed');
  });

  it('a lost link says so on its own, because the mode is no longer known', () => {
    expect(vehicleSubtitle(state({ contactLost: true, stateText: 'Communication lost' }))).toBe('Communication lost');
  });

  it('no vehicle says so rather than showing an empty line', () => {
    expect(vehicleSubtitle(null)).toBe('No vehicle');
    expect(vehicleSubtitle(state({ connected: false }))).toBe('No vehicle');
  });

  it('with no vehicle the header shows the link state like MainStatusIndicator', () => {
    const offline = offlineMainStatus(json('{"kind":"object","title":"Connection Failed","mainStatus":"Can\'t Connect"}'));
    expect(vehicleSubtitle(state({ connected: false }), offline)).toBe("Can't connect");
    expect(vehicleSubtitle(null, offlineMainStatus(json('{"mainStatus":"Connect a Vehicle"}')))).toBe('Connect a vehicle');
    expect(vehicleSubtitle(state(), offline), 'a connected vehicle ignores the link state').toBe('Stabilize · Disarmed');
    expect(offlineMainStatus(json('{"mainStatus":""}'))).toBeNull();
  });

  it('a blank mode does not leave a dangling separator', () => {
    expect(vehicleSubtitle(state({ stateText: 'Armed', mode: '' }))).toBe('Armed');
  });
});

describe('FlyStateArmedTest', () => {
  const view = (armed: boolean, connected = true) =>
    flyState(
      json(`{"kind":"object","class":"FlyState","connected":${connected},"armed":${armed},
         "contactLost":false,"state":"armed","stateText":"Armed","staleNotice":"",
         "mode":"Stabilize","rcSupported":false,"rcSignal":null,"rcSignalText":null}`),
    );

  it('armed comes from the view rather than a second read of the vehicle', () => {
    expect(view(true)?.armed).toBe(true);
    expect(view(false)?.armed).toBe(false);
  });

  it('no vehicle is not an armed vehicle', () => {
    expect(flyState(json('{"kind":"object","class":"FlyState","connected":false}'))?.armed).toBe(false);
  });

  it("the status sheet summary is the core's line", () => {
    const state = flyState(json('{"class":"FlyState","connected":true,"stateText":"Not Fully Ready","summaryDetail":"Mag turned off. Everything else reports normal."}'));
    expect(state?.summaryDetail).toBe('Mag turned off. Everything else reports normal.');
    expect(flyState(json('{"class":"FlyState","summaryDetail":null}'))?.summaryDetail).toBe('');
  });
});

describe('FlyReadinessTest', () => {
  const state = ({ armed = false, ready = true, nominal = true, fault = false, flying = 'disarmed' }: { armed?: boolean; ready?: boolean; nominal?: boolean; fault?: boolean; flying?: string } = {}) =>
    FlyState({
      connected: true,
      armed,
      contactLost: false,
      state: flying,
      stateText: ready ? 'Ready to fly' : 'Not ready',
      staleNotice: '',
      mode: 'Stabilize',
      rcSupported: false,
      rcSignalText: '',
      rcSignal: null,
      rcOverride: null,
      telemetry: null,
      summaryDetail: '2 checks need attention before arming.',
      nominal,
      fault,
      readyToFly: ready,
    });

  it('aVehicleThatIsNotReadyIsNotShownGreen', () => {
    expect(chipTone(state({ ready: false }), false)).toBe(ChipTone.Neutral);
    expect(chipTone(state(), false)).toBe(ChipTone.Success);
    expect(chipTone(state({ nominal: false }), false)).toBe(ChipTone.Warning);
    expect(chipTone(state({ fault: true }), false)).toBe(ChipTone.Error);
    expect(chipTone(state({ armed: true, ready: false }), false)).toBe(ChipTone.Success);
  });

  it('theReadinessWarningSaysWhyAndOnlyBeforeArming', () => {
    expect(readinessWarning(state({ ready: false }))).toBe('Not ready. 2 checks need attention before arming.');
    expect(readinessWarning(state())).toBeNull();
    expect(readinessWarning(state({ armed: true, ready: false }))).toBeNull();
    expect(readinessWarning({ ...state({ ready: false }), summaryDetail: ALL_CHECKS_PASSED })).toBe('Not ready. Aircraft setup is not complete.');
  });

  it('aReadinessIssueBlocksOnlyWhenTheVehicleCannotArm', () => {
    expect(guidedReadiness(state({ ready: false }))).toEqual(Readiness({ text: 'Not ready. 2 checks need attention before arming.', blocks: false }));
    expect(guidedReadiness({ ...state({ ready: false }), canArm: false })).toEqual(Readiness({ text: `Not ready. 2 checks need attention before arming. ${READINESS_BLOCKED}`, blocks: true }));
    expect(guidedReadiness(state())).toBeNull();
  });

  it('disarmingSaysWhetherTheVehicleFlew', () => {
    expect(disarmNotice(true, true, false)).toBe('Landed and disarmed');
    expect(disarmNotice(true, false, false)).toBe('Disarmed');
    expect(disarmNotice(false, false, false)).toBeNull();
  });

  it('aLandingSaysHowLongHowFarAndHowMuchBatteryTheFlightTook', () => {
    expect(landedSummary(192, '240 m', 23)).toBe('03\'12" · 240 m · 23% battery used');
    expect(landedSummary(45, null, null)).toBe('00\'45"');
    expect(landedSummary(null, '', 0)).toBe('');
    expect(flownDistanceText(json('{"kind":"fact","value":53.0,"valueString":"53.0","units":"m"}')), 'the core answers vehicle.flightDistance with the fact itself').toBe('53.0 m');
    expect(flownDistanceText(json('{"kind":"fact","value":0.0,"valueString":"0.0","units":"m"}')), 'a flight straight up and down covered no ground worth saying').toBeNull();
    expect(flownDistanceText(json('{"kind":"fact","value":null,"valueString":""}'))).toBeNull();
  });

  it('theMapLeadsWhileArmedWithNoVideoSource', () => {
    expect(flyViewShown(FlyView.Video, true, true)).toBe(FlyView.Map);
    expect(flyViewShown(FlyView.Video, false, true)).toBe(FlyView.Video);
    expect(flyViewShown(FlyView.Video, true, false)).toBe(FlyView.Video);
  });
});
