import { describe, expect, it } from 'vitest';
import { AppLogFilter, appLogFileName, appLogMime, appLogPath, appLogRead, mergedEntries } from './AppLogPage';

describe('AppLogPageTest', () => {
  it('search text is encoded so commas stay inside one argument', () => {
    expect(appLogPath(AppLogFilter({ levelIndex: 2, category: 'hub', text: 'a, b', regex: true }), 7)).toBe('view.appLog(1,hub,a%2C+b,1,7)');
    expect(appLogPath(AppLogFilter({ levelIndex: 0 }), null)).toBe('view.appLog(0,,,0,)');
  });

  it('held entries evicted from the core are dropped before new ones are appended', () => {
    const read = appLogRead({
      class: 'AppLog',
      levels: [],
      categories: [],
      first: 5,
      entries: [{ sequence: 6, level: 2, message: 'W lost', category: 'hub', timestamp: '03:20:11.000', source: '' }],
    });
    expect(read).not.toBeNull();
    const held = [4, 5].map((sequence) => ({ sequence, level: 1, message: 'I', category: 'hub', timestamp: '', source: '' }));
    expect(mergedEntries(held, read!).map((entry) => entry.sequence)).toEqual([5, 6]);
  });

  it('the save name and type follow the log save format setting like the QGC dialog suffix', () => {
    expect(appLogFileName({ value: 0 })).toBe('QGCConsole.txt');
    expect(appLogFileName({ value: 1 })).toBe('QGCConsole.csv');
    expect(appLogFileName(null)).toBe('QGCConsole.txt');
    expect(appLogMime('QGCConsole.csv')).toBe('text/csv');
    expect(appLogMime('QGCConsole.txt')).toBe('text/plain');
  });
});
