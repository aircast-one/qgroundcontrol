import { describe, expect, it } from 'vitest';
import { LoadStep, loadStep, replaceWarning } from './LoadConfirm';

describe('LoadConfirmTest', () => {
  it('a clean plan loads without asking', () => {
    expect(loadStep(false, false)).toBe(LoadStep.Load);
  });

  it('unsent changes are not discarded on the first tap', () => {
    expect(loadStep(true, false)).toBe(LoadStep.Confirm);
  });

  it('the second tap goes through', () => {
    expect(loadStep(true, true)).toBe(LoadStep.Load);
  });

  it('an armed confirm on a clean plan still just loads', () => {
    expect(loadStep(false, true)).toBe(LoadStep.Load);
  });

  it('a dirty plan asks even with no mission items, as downloadClicked checks only dirty', () => {
    expect(loadStep(true, false)).toBe(LoadStep.Confirm);
  });

  it('the replace warning counts the plan that would be lost', () => {
    expect(replaceWarning(6)).toBe('Your unsaved plan here (6 items) will be replaced.');
    expect(replaceWarning(1)).toBe('Your unsaved plan here (1 item) will be replaced.');
  });
});
