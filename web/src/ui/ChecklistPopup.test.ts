import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { type PreflightGroup, checklistHeading, checklistIsComplete, checklistPopupIsDue, collapsedAfterPass, groupEnabled, groupHeading, preflight } from './Preflight';

const served = (checks: readonly string[], blocked = 0) =>
  preflight(
    parseJson(
      `{"kind":"object","class":"Preflight","total":${checks.length},
        "blocked":[${Array.from({ length: blocked }, (_, it) => `"stop ${it}"`).join(',')}],
        "groups":[{"name":"Before flight","checks":[${checks.join(',')}]}]}`,
    ),
  );

const manual = (name: string) => `{"name":"${name}","verdict":"manual","text":"${name}"}`;
const automatic = (name: string) => `{"name":"${name}","verdict":"pass","text":"${name}"}`;

describe('ChecklistPopupTest', () => {
  it('an unticked manual check leaves the list incomplete', () => {
    expect(checklistIsComplete(served([manual('Props on')]), new Set())).toBe(false);
    expect(checklistIsComplete(served([manual('Props on')]), new Set(['Props on']))).toBe(true);
  });

  it('a blocking check is never complete, however much is ticked', () => {
    expect(
      checklistIsComplete(served([manual('Props on')], 1), new Set(['Props on'])),
      'a blocker stops the flight; ticking the manual items around it does not clear it',
    ).toBe(false);
  });

  it('a list with nothing to tick is already complete', () => {
    expect(checklistIsComplete(served([automatic('Battery')]), new Set())).toBe(true);
  });

  it('no checklist at all is not complete', () => {
    expect(checklistIsComplete(null, new Set()), 'absent is not passed - with no vehicle there is nothing to have checked').toBe(false);
  });

  it('the popup never opens over a decision in progress', () => {
    expect(
      checklistPopupIsDue(true, true, true, false, true),
      'a hold-to-confirm or a value panel is the operator mid-decision; a checklist landing on top of it takes the screen at the worst moment',
    ).toBe(false);
    expect(
      checklistPopupIsDue(true, true, true, false, false),
      'and it is owed once they are done, not cancelled - the effect re-keys on the flag so closing the decision brings it back',
    ).toBe(true);
  });

  it('the popup is due only when every flag Qt reads is set', () => {
    expect(checklistPopupIsDue(true, true, true, false)).toBe(true);
    expect(checklistPopupIsDue(false, true, true, false), 'no vehicle, nothing to check').toBe(false);
    expect(checklistPopupIsDue(true, false, true, false), 'the operator turned the checklist off').toBe(false);
    expect(checklistPopupIsDue(true, true, false, false), 'enforcement off means the list is available but not pushed').toBe(false);
    expect(checklistPopupIsDue(true, true, true, true), 'already done, so opening it would interrupt for nothing').toBe(false);
  });

  it('groups unlock in order and say when they pass, like PreFlightCheckModel enforceOrder', () => {
    const first: PreflightGroup = { name: 'Initial checks', checks: [{ name: 'hardware', prompt: '', verdict: 'manual', reason: '', blocked: false }] };
    const second: PreflightGroup = { name: 'Please arm the vehicle here', checks: [{ name: 'arm', prompt: '', verdict: 'manual', reason: '', blocked: false }] };
    const groups = [first, second];
    expect(groups.map((_, it) => groupEnabled(groups, it, new Set()))).toEqual([true, false]);
    expect(groups.map((_, it) => groupEnabled(groups, it, new Set(['hardware'])))).toEqual([true, true]);
    expect(groupHeading(first, new Set(['hardware']))).toBe('Initial checks (passed)');
    expect(groupHeading(second, new Set(['hardware']))).toBe('Please arm the vehicle here');
    expect(checklistHeading(true)).toBe('Pre-flight checklist (passed)');
    expect(checklistHeading(false)).toBe('Pre-flight checklist in progress');
  });

  it('only a group that has just passed collapses, like PreFlightCheckList _handleGroupPassedChanged', () => {
    expect(collapsedAfterPass(new Set(), new Set(), new Set(['Initial checks']))).toEqual(new Set(['Initial checks']));
    expect(collapsedAfterPass(new Set(), new Set(['Initial checks']), new Set(['Initial checks', 'Arm'])), 'a group the operator reopened stays open when another passes').toEqual(new Set(['Arm']));
  });
});
