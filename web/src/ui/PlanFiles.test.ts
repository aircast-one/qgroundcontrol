import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { PlanConfirm, confirmCopy, planActions, planHistory, planStatusText, planTitle, saveBlockedReason } from './PlanFileRules';

const json = (text: string): Json => parseJson(text)!;

describe('SaveBlockedTest', () => {
  const view = (body: string) => json(`{"kind":"object","readiness":${body}}`);

  it('a plan the core calls ready is not blocked', () => {
    expect(saveBlockedReason(view('{"state":0,"ready":true,"reason":""}'))).toBeNull();
  });

  it("the core's own sentence is shown, not this head's paraphrase of the same state", () => {
    expect(saveBlockedReason(view('{"state":1,"ready":false,"reason":"Waiting for terrain heights before the plan can be saved or sent."}'))).toBe(
      'Waiting for terrain heights before the plan can be saved or sent.',
    );
    expect(saveBlockedReason(view('{"state":2,"ready":false,"reason":"An item is still being drawn, so the plan cannot be saved or sent."}'))).toBe(
      'An item is still being drawn, so the plan cannot be saved or sent.',
    );
  });

  it('no answer blocks the save rather than letting it through unchecked', () => {
    expect(saveBlockedReason(null)).toBe('The plan could not be checked for saving.');
    expect(saveBlockedReason(json('{"kind":"null"}'))).toBe('The plan could not be checked for saving.');
  });

  it('a state the core has no sentence for still refuses rather than passing silently', () => {
    expect(saveBlockedReason(view('{"state":9,"ready":false,"reason":""}'))).toBe('The plan could not be checked for saving.');
  });
});

describe('PlanStatusTest', () => {
  const view = (status: string): Json => ({ kind: 'object', status });

  it('the core spells the status, including whether changes are unsaved or unsent', () => {
    expect(planStatusText(view('ridge.plan · not uploaded'))).toBe('ridge.plan · not uploaded');
    expect(planStatusText(view('ridge.plan · unsaved changes'))).toBe('ridge.plan · unsaved changes');
  });

  it('no answer shows nothing, so a silent core is visible rather than papered over', () => {
    expect(planStatusText(null)).toBe('');
    expect(planStatusText(json('{"kind":"null"}'))).toBe('');
  });
});

describe('PlanActionsTest', () => {
  const actions = ({ open = true, save = true, exportKml = true, newPlan = true, clearMission = true } = {}) =>
    planActions({ kind: 'object', actions: { open, save, exportKml, newPlan, clearMission } });

  it("each action is the core's answer rather than this head's arithmetic", () => {
    expect(actions({ save: false }).save).toBe(false);
    expect(actions({ save: false }).open).toBe(true);
    expect(actions({ exportKml: false }).exportKml).toBe(false);
  });

  it("clearMission is the core's name for clearing the vehicle, which is what this button does", () => {
    expect(actions({ clearMission: false }).clearFromVehicle).toBe(false);
    expect(actions({ clearMission: true }).clearFromVehicle).toBe(true);
  });

  it('download from vehicle follows the core and confirms over unsaved changes', () => {
    expect(planActions(json('{"actions":{"download":true}}')).download).toBe(true);
    expect(planActions(json('{"actions":{"download":false}}')).download).toBe(false);
    expect(confirmCopy(PlanConfirm.Download).title).toBe('Plan overwrite');
  });

  it("opening over changes and creating over items ask in PlanView's words", () => {
    expect(confirmCopy(PlanConfirm.Open).title).toBe('Plan overwrite');
    expect(confirmCopy(PlanConfirm.Open).body).toBe('You have unsaved/unsent changes. Loading from a file will lose these changes. Are you sure you want to load from a file?');
    expect(confirmCopy(PlanConfirm.NewPlan).title).toBe('Create Plan');
  });

  it('no answer offers nothing rather than guessing what is allowed', () => {
    const none = planActions(null);
    expect(none.open).toBe(false);
    expect(none.save).toBe(false);
    expect(none.newPlan).toBe(false);
  });
});

describe('PlanHistoryTest', () => {
  it('a plan with nothing behind it offers neither', () => {
    expect(planHistory(json('{"kind":"object","canUndo":false,"canRedo":false}'))).toEqual({ canUndo: false, canRedo: false });
  });

  it('an edited plan can be undone but not yet redone', () => {
    expect(planHistory(json('{"kind":"object","canUndo":true,"canRedo":false}'))).toEqual({ canUndo: true, canRedo: false });
  });

  it('a view that never answered offers nothing, rather than enabled buttons that refuse', () => {
    expect(planHistory(null)).toEqual({ canUndo: false, canRedo: false });
    expect(planHistory(json('{}'))).toEqual({ canUndo: false, canRedo: false });
  });

  it('the title is the file\'s base name, or Untitled Plan like PlanToolBarIndicators', () => {
    expect(planTitle('ridge.plan')).toBe('ridge');
    expect(planTitle('a.b.plan')).toBe('a.b');
    expect(planTitle('ridge')).toBe('ridge');
    expect(planTitle('.plan')).toBe('.plan');
    expect(planTitle(null)).toBe('Untitled Plan');
    expect(planTitle('')).toBe('Untitled Plan');
  });
});
