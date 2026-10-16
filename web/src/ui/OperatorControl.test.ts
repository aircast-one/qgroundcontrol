import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  acquireEnabled,
  acquireLabel,
  allowTakeoverEditable,
  controlIsElsewhere,
  controlLine,
  controlSectionTitle,
  controlStation,
  controlWaitLine,
  holderLine,
  inControlLine,
  requestSentLabel,
  requestTimeoutEditable,
  requestTimeoutSeconds,
  takeoverChangeable,
  takeoverLine,
} from './OperatorControl';

const served = (body: string): Json => parseJson(`{"kind":"object","class":"OperatorControl",${body}}`) ?? {};

const unavailable = served(`"available":false,"known":false,"inControl":null,"holderSystemId":null,
   "takeoverAllowed":null,"systemManager":null,"reason":"No vehicle is connected." `);

const silent = served(`"available":true,"known":false,"inControl":null,"holderSystemId":null,"takeoverAllowed":null,
   "requestAllowed":true,"reason":"This vehicle has not said who is flying it." `);

const elsewhere = (takeover: boolean, request = true): Json =>
  served(`"available":true,"known":true,"inControl":false,"holderSystemId":42,"takeoverAllowed":${takeover},
     "requestAllowed":${request},"reason":"Another ground station is flying this vehicle." `);

const ours = served(`"available":true,"known":true,"inControl":true,"holderSystemId":255,"takeoverAllowed":true,
   "requestAllowed":true,"reason":"" `);

describe('OperatorControlTest', () => {
  it('a vehicle that is not there is not a station question', () => {
    expect(controlStation(unavailable)).toBeNull();
    expect(controlStation(null)).toBeNull();
  });

  it('a vehicle that has never said who is flying it is not reported as taken', () => {
    const station = controlStation(silent)!;
    expect(station.known).toBe(false);
    expect(station.inControl).toBeNull();
    expect(controlIsElsewhere(station)).toBe(false);
    expect(controlLine(station), 'a single-station setup would carry this sentence forever, and QGC hides the indicator entirely until the vehicle answers').toBeNull();
  });

  it('holding control offers no request, only the takeover condition', () => {
    const station = controlStation(ours)!;
    expect(controlIsElsewhere(station)).toBe(false);
    expect(controlLine(station)).toBeNull();
    expect(acquireLabel(station)).toBeNull();
    expect(inControlLine(station)).toBe('System in control: This GCS (255)');
    expect(takeoverLine(station)).toBe('Takeover allowed');
    expect(takeoverChangeable(station, true), 'Change is off until the box differs from what the vehicle reports').toBe(false);
    expect(takeoverChangeable(station, false)).toBe(true);
    expect(takeoverChangeable(controlStation(elsewhere(true)), false)).toBe(false);
    expect(allowTakeoverEditable(station), 'QGC enables the box while this station holds control').toBe(true);
    expect(allowTakeoverEditable(controlStation(elsewhere(true))), 'or while the holder allows a takeover').toBe(true);
    expect(allowTakeoverEditable(controlStation(elsewhere(false)))).toBe(false);
  });

  it('the takeover condition shows whoever holds control, and the request counts down', () => {
    expect(takeoverLine(controlStation(elsewhere(false)))).toBe('Takeover NOT allowed');
    expect(takeoverLine(controlStation(silent))).toBeNull();
    expect(inControlLine(controlStation(elsewhere(true)))).toBeNull();
    expect(requestSentLabel(9_500)).toBe('Request sent: 9.5');
    expect(requestSentLabel(-20)).toBe('Request sent: 0.0');
  });

  it('another station is named by the system it flies under', () => {
    const station = controlStation(elsewhere(true))!;
    expect(controlIsElsewhere(station)).toBe(true);
    expect(controlLine(station)).toBe('Another ground station is flying this vehicle. (system 42)');
    expect(holderLine(station), "GCSControlIndicator lists the holder as 'System in control:' and its id").toBe('System in control: 42');
    expect(holderLine(controlStation(ours))).toBeNull();
    expect(holderLine(controlStation(silent))).toBeNull();
  });

  it('the offer follows whether takeover was allowed', () => {
    expect(acquireLabel(controlStation(elsewhere(true)))).toBe('Acquire control');
    expect(acquireLabel(controlStation(elsewhere(false)))).toBe('Send request');
  });

  it('a pending request keeps the button but disables it like the QML', () => {
    const pending = controlStation(elsewhere(false, false))!;
    expect(acquireLabel(pending)).toBe('Send request');
    expect(acquireEnabled(pending, false)).toBe(false);
    expect(acquireEnabled(controlStation(elsewhere(false))!, true)).toBe(false);
    expect(acquireEnabled(controlStation(elsewhere(false))!, false)).toBe(true);
  });

  it('a station that never answered is not offered a request either', () => {
    expect(acquireLabel(controlStation(silent))).toBeNull();
  });

  it('the timeout is only spent when takeover has to be asked for', () => {
    expect(
      requestTimeoutSeconds(controlStation(elsewhere(true))!, 10),
      "Vehicle::requestOperatorControl clamps anything outside 3..60 to the setting's default, so this 0 is what the API takes and never what the command carries - it means do not start a waiting timer, which is what the QML does with it too",
    ).toBe(0);
    expect(requestTimeoutSeconds(controlStation(elsewhere(false))!, 10)).toBe(10);
  });

  it('an outstanding request is named while the other station has not answered', () => {
    const waiting = controlStation(elsewhere(false, false));
    expect(controlWaitLine(waiting)).toBe('Waiting for the other station to answer');
  });

  it('a station that may still be asked is not described as waiting', () => {
    expect(controlWaitLine(controlStation(elsewhere(false)))).toBeNull();
  });

  it('the vehicle we fly is never described as waiting on us', () => {
    expect(controlWaitLine(controlStation(ours))).toBeNull();
    expect(controlWaitLine(controlStation(silent))).toBeNull();
  });

  it('the wait line stands beside the disabled button', () => {
    const waiting = controlStation(elsewhere(true, false))!;
    expect(acquireLabel(waiting)).toBe('Acquire control');
    expect(acquireEnabled(waiting, false)).toBe(false);
    expect(controlWaitLine(waiting)).toBe('Waiting for the other station to answer');
  });

  it('theSheetTitlesAndTimeoutFieldFollowGcsControlIndicator', () => {
    const asking = controlStation(elsewhere(false))!;
    const taking = controlStation(elsewhere(true))!;
    const holding = controlStation(ours)!;
    expect(controlSectionTitle(asking)).toBe('Send control request');
    expect(controlSectionTitle(holding)).toBe('Change takeover condition');
    expect(controlSectionTitle(controlStation(silent)!)).toBeNull();
    expect(requestTimeoutEditable(asking)).toBe(true);
    expect(requestTimeoutEditable(taking)).toBe(false);
    expect(requestTimeoutEditable(holding)).toBe(false);
  });
});
