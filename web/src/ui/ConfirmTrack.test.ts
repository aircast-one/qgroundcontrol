import { describe, expect, it } from 'vitest';
import { holdLabel } from './HoldToConfirm';
import { MISSION_ACTIONS, MISSION_ON_DRONE, MissionIdentity, landFrom, missionIdentity, sentIsStillShowing, sentText } from './ConfirmTrack';

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
});
