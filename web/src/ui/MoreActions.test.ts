import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { EMERGENCY_STOP } from './EmergencyStop';
import { GuidedOffer, guidedOffers, resumeFromSequence } from './GuidedActions';
import { gripperOffers } from './GripperPanel';
import { PAUSE, SHEET_ACTIONS, autoMissionPopup, guidedCommand, moreActions, popupReplacesOpenConfirm } from './MoreActions';

const json = (text: string): Json => parseJson(text) ?? {};

const offers = (...entries: readonly (readonly [string, string])[]): ReadonlyMap<string, GuidedOffer> =>
  guidedOffers(
    json(
      `{"actions":[${entries
        .map(([id, offer]) => `{"id":"${id}","title":"${id}","offer":"${offer}","reason":"","prompt":"","destructive":false,"carriesValue":false}`)
        .join(',')}]}`,
    ),
  );

describe('MoreActionsTest', () => {
  it('an ROI lock the vehicle is holding can be released from this head', () => {
    expect(
      moreActions(offers(['cancelRoi', 'ready'])).map((it) => it.id),
      'the core offers CancelRoi only on roi_supported && roi_active && flying, so the sheet showing it means a lock IS in force - and with roiActive drawn nowhere either, leaving it out left the camera pinned to a point with no indication and no escape',
    ).toEqual(['cancelRoi']);
    expect(guidedCommand('cancelRoi', null)).not.toBeNull();
  });

  it('resuming a mission carries the waypoint the core holds, never one the head counted', () => {
    expect(guidedCommand('resumeMission', 5)).not.toBeNull();
    expect(moreActions(offers(['resumeMission', 'ready'])).map((it) => it.id)).toEqual(['resumeMission']);
  });

  it('without the waypoint there is no resume command to run', () => {
    expect(
      guidedCommand('resumeMission', null),
      'can_resume() requires resume_from_sequence > 0 and the field is served on that same predicate, so a ready offer always carries the number - this is the unreachable half, refusing to send rather than sending a resume to waypoint zero',
    ).toBeNull();
  });

  it('the waypoint is read from the core, and a withheld one is not a zero', () => {
    expect(resumeFromSequence(json('{"resumeFromSequence":5}'))).toBe(5);
    expect(resumeFromSequence(json('{"resumeFromSequence":null}'))).toBeNull();
    expect(resumeFromSequence(json('{}'))).toBeNull();
    expect(
      resumeFromSequence(json('{"resumeFromSequence":0}')),
      "guided.rs:274 withholds a non-positive sequence, so this guard is unreachable from today's core - it is kept because a resume sent to waypoint zero would fly the plan again from the start, which is the one thing resume exists not to do",
    ).toBeNull();
    expect(resumeFromSequence(null)).toBeNull();
  });

  it('the sheet carries what the row does not', () => {
    const extra = moreActions(offers(['arm', 'ready'], ['takeoff', 'ready'], ['startMission', 'ready'], ['pause', 'blocked'], ['landAbort', 'ready']));

    expect(extra.map((it) => it.id)).toEqual(['startMission', 'pause', 'landAbort']);
  });

  it("the gripper panel lists Release Grab Hold in QGC's order", () => {
    const extra = moreActions(offers(['hold', 'ready'], ['grab', 'ready'], ['release', 'disabled'], ['startMission', 'ready']));

    expect(gripperOffers(extra).map((it: GuidedOffer) => it.id)).toEqual(['release', 'grab', 'hold']);
    expect(gripperOffers(moreActions(offers(['grab', 'hidden']))).map((it: GuidedOffer) => it.id)).toEqual([]);
  });

  it('an action the core hides is not offered', () => {
    const extra = moreActions(offers(['grab', 'hidden'], ['release', 'ready']));

    expect(extra.map((it) => it.id)).toEqual(['release']);
  });

  it('an action this head cannot send is not offered', () => {
    expect(moreActions(offers(['orbit', 'ready'])).map((it) => it.id)).toEqual([]);
  });

  it('every action the sheet shows can be sent, except pause which asks for a height', () => {
    const ids = moreActions(offers(...[...SHEET_ACTIONS, EMERGENCY_STOP].map((it) => [it, 'ready'] as const))).map((it) => it.id);

    expect(ids.length, 'every id in SHEET_ACTIONS, so adding one to the set without a command here is caught').toBe(SHEET_ACTIONS.size);
    expect(ids.filter((it) => it !== PAUSE && guidedCommand(it, 5) === null)).toEqual([]);
    expect(guidedCommand(PAUSE, 5)).toBeNull();
  });
});

describe('AutoMissionPopupTest', () => {
  const offer = (id: string, state: string) => GuidedOffer({ id, title: id, offer: state, reason: '', prompt: '', destructive: false, carriesValue: false });

  it('start mission pops up the moment it becomes available and not again', () => {
    const ready = new Map([['startMission', offer('startMission', 'ready')]]);
    expect(autoMissionPopup(new Set(), ready, true)?.id).toBe('startMission');
    expect(autoMissionPopup(new Set(['startMission']), ready, true)).toBeNull();
    expect(autoMissionPopup(new Set(), ready, false)).toBeNull();
    expect(autoMissionPopup(new Set(), new Map([['startMission', offer('startMission', 'blocked')]]), true)).toBeNull();
  });

  it('land abort pops up on a fixed-wing approach even with mission popups off', () => {
    const approach = new Map([['landAbort', offer('landAbort', 'ready')]]);
    expect(autoMissionPopup(new Set(), approach, false)?.id, "GuidedActionsController's onShowLandAbortChanged confirms it whatever enableAutomaticMissionPopups says").toBe('landAbort');
    expect(autoMissionPopup(new Set(['landAbort']), approach, false)).toBeNull();
    expect(popupReplacesOpenConfirm('landAbort'), 'confirmAction(actionLandAbort) replaces whatever confirm is open').toBe(true);
    expect(popupReplacesOpenConfirm('startMission')).toBe(false);
  });

  it('continue mission pops up too', () => {
    expect(autoMissionPopup(new Set(), new Map([['continueMission', offer('continueMission', 'ready')]]), true)?.id).toBe('continueMission');
  });
});
