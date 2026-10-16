import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { followMeLabel, followMeReading } from './FollowMe';

const json = (text: string): Json => parseJson(text) ?? {};

const view = ({
  mode = '"followMe"',
  enabled = true,
  wouldSend = true,
  reason = 'null',
  vehicles = '[{"id":1,"following":true,"refusal":""}]',
}: { mode?: string; enabled?: boolean; wouldSend?: boolean; reason?: string; vehicles?: string } = {}): Json =>
  json(`{"kind":"object","class":"FollowMe","mode":${mode},"enabled":${enabled},
         "wouldSend":${wouldSend},"reason":${reason},"vehicles":${vehicles},"count":1}`);

describe('FollowMeTest', () => {
  it('another view is not a follow reading', () => {
    expect(followMeReading(null)).toBeNull();
    expect(followMeReading(json('{"kind":"object","class":"Orbit"}'))).toBeNull();
  });

  it('the default setting does not announce that a feature nobody asked for is not happening', () => {
    expect(
      followMeLabel(followMeReading(view({ mode: '"followMe"', wouldSend: false, reason: '"noVehicleInFollowMode"' }))),
      'followTarget defaults to 2, which Mode::from_setting maps to followMe - so a fresh install with a connected vehicle that is not in Follow Me flight mode is the ordinary resting state, and every operator was being told about it on the Fly screen for a feature they never turned on',
    ).toBeNull();
  });

  it('the core cannot report that reason under always, so the mode does not need testing', () => {
    expect(
      followMeLabel(followMeReading(view({ mode: '"always"', wouldSend: false, reason: '"noVehicleInFollowMode"' }))),
      'receives() is Mode::Always => true, so with a non-empty fleet reason() can never return NoVehicleInFollowMode under always - a head that also checked the mode would be guarding a state the core cannot produce, and macOS treats the token as resting whatever the mode for the same reason',
    ).toBeNull();
  });

  it('once a vehicle is in Follow Me mode every other trouble is still reported', () => {
    expect(
      followMeLabel(followMeReading(view({ mode: '"followMe"', wouldSend: false, reason: '"noFix"' }))),
      "reason() returns NoVehicleInFollowMode before it looks at the fix at all, so any other reason means a vehicle IS in Follow Me mode and the stream is failing anyway - that is the operator's own request breaking, not a resting default",
    ).toBe('Not following you — this phone has no position yet');
  });

  it('a vehicle taking the stream says so plainly', () => {
    expect(followMeLabel(followMeReading(view()))).toBe('Following you');
  });

  it('a switched-off feature shows nothing at all, rather than a complaint', () => {
    const off = view({ mode: '"never"', enabled: false, wouldSend: false, reason: '"modeNever"' });

    expect(followMeLabel(followMeReading(off))).toBeNull();
  });

  it('switched on and not sending names the thing standing in the way', () => {
    const stuck = (token: string) => followMeLabel(followMeReading(view({ mode: '"always"', enabled: false, wouldSend: false, reason: `"${token}"` })));

    expect(stuck('fixStale')).toBe("Not following you — this phone's position has stopped updating");
    expect(stuck('fixInvalid')).toBe("Not following you — this phone's position is not valid");
    expect(stuck('allVehiclesRefused')).toBe('Not following you — the vehicle refused the position');
  });

  it('a token this head has never seen still produces a sentence', () => {
    expect(followMeLabel(followMeReading(view({ enabled: false, wouldSend: false, reason: '"somethingNew"' })))).toBe('Not following you — the position is not being sent');
  });

  it('more than one follower is worth counting, one is not', () => {
    const pair = view({ vehicles: '[{"id":1,"following":true},{"id":2,"following":true}]' });

    expect(followMeLabel(followMeReading(pair))).toBe('Following you · 2 vehicles');
  });

  it('enabled is the timer running, not the operator asking, so the chip cannot key on it', () => {
    const asked = view({
      mode: '"always"',
      enabled: false,
      wouldSend: false,
      reason: '"fixStale"',
      vehicles: '[{"id":1,"following":false,"refusal":"notInFollowMode"}]',
    });

    expect(followMeLabel(followMeReading(asked))).toBe("Not following you — this phone's position has stopped updating");
  });

  it('no vehicle at all is nothing to say, however the setting reads', () => {
    const alone = view({ enabled: false, wouldSend: false, reason: '"noVehicles"', vehicles: '[]' });

    expect(followMeLabel(followMeReading(alone))).toBeNull();
  });
});
