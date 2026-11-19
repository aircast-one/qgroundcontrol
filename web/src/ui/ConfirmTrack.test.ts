import { describe, expect, it } from 'vitest';
import { holdLabel } from './HoldToConfirm';
import { MISSION_ACTIONS, MISSION_ON_DRONE, MissionIdentity, confirmReadiness, landFrom, missionIdentity, rtlPreview, sentIsStillShowing, sentText, sentUnlessRefused } from './ConfirmTrack';
import { Readiness } from './FlyState';
import { guidedCommitLabel } from './GuidedValuePanel';

describe('ConfirmTrackTest', () => {
  it('sent stands until the served actions change', () => {
    expect(
      sentIsStillShowing('Land', '{A}', '{A}'),
      'the operator needs to see the command left the handset while the link is still carrying it; reverting to buttons straight away reads as nothing happened',
    ).toBe(true);
    expect(sentIsStillShowing('Land', '{A}', '{B}'), "once the vehicle's own flags move, the next state answers the question and the notice has nothing left to say").toBe(false);
  });

  it('nothing was sent means nothing is shown', () => {
    expect(sentIsStillShowing(null, '{A}', '{A}')).toBe(false);
    expect(sentIsStillShowing('Land', null, '{A}')).toBe(false);
  });

  it('a send recorded against no reading does not stick', () => {
    expect(sentIsStillShowing('Land', null, null), 'a null snapshot at send would match a null live read and hold the notice forever').toBe(false);
  });

  it('the notice names the action that was sent', () => {
    expect(sentText('Return')).toBe('Sent · Return');
    expect(sentText('Emergency Stop')).toBe('Sent · Emergency Stop');
  });

  it('an action the core refuses shows only the refusal, never sent', () => {
    expect(sentUnlessRefused('Drop bay 4', 'The gripper is still open from the last drop. It closes in 1 s.')).toBeNull();
    expect(sentUnlessRefused('Drop bay 4', null)).toBe('Drop bay 4');
  });

  it('the land confirm reads the height it lands from, and nothing when the vehicle has not said', () => {
    expect(landFrom({ items: [{ missing: false, value: '42.0', units: 'm' }] })).toEqual(['42.0', 'm']);
    expect(landFrom({ items: [{ missing: true, value: '—', units: '' }] })).toBeNull();
    expect(landFrom(null)).toBeNull();
  });

  it('starting a mission names the mission the drone will fly', () => {
    const summary = { rows: [{ id: 'distance', label: 'Distance', value: '1.40 km' }, { id: 'time', label: 'Time', value: '6 min' }] };
    const uploaded = { hasMissionItems: true, dirty: false, file: 'ridge.plan', status: 'Uploaded · 12 items' };
    expect(missionIdentity(uploaded, summary)).toEqual(MissionIdentity({ title: 'ridge', facts: 'Uploaded · 12 items · 1.40 km · 6 min', warning: null }));
    const edited = { hasMissionItems: true, dirty: true, file: 'ridge.plan', status: 'Edited · 13 items' };
    expect(missionIdentity(edited, summary).title).toBe(MISSION_ON_DRONE);
    expect(missionIdentity(edited, summary).warning?.startsWith('ridge has changes')).toBe(true);
    expect(missionIdentity({ hasMissionItems: false }, summary).title).toBe(MISSION_ON_DRONE);
    expect(holdLabel('Start mission')).toBe('Slide to start mission');
    expect(holdLabel('Land')).toBe('Slide to land');
    expect(holdLabel('')).toBe('Slide to confirm');
    expect(MISSION_ACTIONS.has('land')).toBe(false);
  });

  it('starting a mission warns like take off does, since it takes off too', () => {
    const warning = Readiness({ text: 'Not fully ready. Battery at 6%. Charge or swap it before flight.', blocks: false });
    expect(confirmReadiness('startMission', warning)).toEqual(warning);
    expect(holdLabel(guidedCommitLabel('Start mission', confirmReadiness('startMission', warning)))).toBe('Slide to start mission anyway');
    expect(confirmReadiness('takeoff', warning), 'a wing takes off from this confirm rather than the height panel').toEqual(warning);
    expect(holdLabel(guidedCommitLabel('Take off', confirmReadiness('takeoff', warning)))).toBe('Slide to take off anyway');
    expect(confirmReadiness('land', warning), 'landing is never refused for an unready aircraft').toBeNull();
    expect(confirmReadiness(null, warning)).toBeNull();
  });

  it('the return confirm reads how far home is, the height it flies back at and the charge on landing', () => {
    expect(rtlPreview({ available: true, distanceText: '153 m', returnAltitudeText: '15 m', climbs: true, verdict: 'About 64% left on landing', underReserve: false })).toEqual({
      distance: '153 m',
      altitude: '15 m',
      climbs: true,
      verdict: 'About 64% left on landing',
      underReserve: false,
    });
    expect(rtlPreview({ available: true, distanceText: '153 m', returnAltitudeText: null, verdict: null })?.altitude).toBeNull();
    expect(rtlPreview({ available: false })).toBeNull();
  });
});
