import { describe, expect, it } from 'vitest';
import { Tab, TabEntries, reselectClearsAnalyze, visibleTabs } from './MainActivity';

describe('TabReselectTest', () => {
  it('re-selecting Analyze drops its sub-page', () => {
    expect(reselectClearsAnalyze(Tab.Analyze, Tab.Analyze)).toBe(true);
  });

  it('re-selecting another tab leaves the Analyze sub-page alone', () => {
    expect(TabEntries.filter((it) => it !== Tab.Analyze).map((entry) => reselectClearsAnalyze(entry, entry))).toEqual([false, false]);
  });

  it('switching to a different tab is not a re-selection at all', () => {
    expect(reselectClearsAnalyze(Tab.Plan, Tab.Analyze)).toBe(false);
    expect(reselectClearsAnalyze(Tab.Analyze, Tab.Plan)).toBe(false);
  });

  it('Analyze leaves the navigation while Advanced Mode is off like SelectViewDropdown', () => {
    expect(visibleTabs(false)).not.toContain(Tab.Analyze);
    expect(visibleTabs(true)).toEqual(TabEntries);
  });

  it('a notice destination names its tab', () => {
    expect(Tab.from('PLAN')).toBe(Tab.Plan);
    expect(Tab.from('setup')).toBeNull();
  });
});
