import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { TIME_UNKNOWN, TIME_UNRECEIVED, type LogsView, compactLogTime, downloadCard, logLocalTime, logSections, logTimeText, logsView, savedToText, shouldAutoRefreshLogs } from './LogDownloadScreen';

const served = `
  {"connected":true,"busy":false,"canRefresh":true,"canDownload":true,
   "canCancel":false,"canErase":true,"anyDownloaded":true,
   "emptyText":"","eraseWarning":"This erases every log on the vehicle.",
   "entries":[
     {"index":0,"id":1,"sizeBytes":4096,"sizeText":"4.0KB","status":"Downloaded",
      "received":true,"selected":false,"time":"2026-09-08T01:20:00",
      "timeState":"known"},
     {"index":1,"id":2,"sizeBytes":10240,"sizeText":"10.0KB","status":"Available",
      "received":true,"selected":true,"time":"2026-09-08T02:20:00",
      "timeState":"known"}]}
`;

const read = (text: string): LogsView => {
  const logs = logsView(parseJson(text));
  if (logs === null) throw new Error('the view did not parse');
  return logs;
};

describe('LogDownloadScreenTest', () => {
  it("logs split into the vehicle's and the phone's by status id", () => {
    const saved = served.replace('"status":"Downloaded",', '"status":"Downloaded","statusId":"downloaded",');
    const sections = logSections(read(saved).entries);
    expect(sections.map(([title, logs]) => [title, logs.map((it) => it.id)])).toEqual([
      ['On the vehicle', [2]],
      ['On this phone', [1]],
    ]);
    expect(logSections(read(served).entries).map(([title]) => title)).toEqual(['On the vehicle']);
  });

  it('the download card names the log being fetched by its status id, not its translated status', () => {
    const downloading = served.replace('"status":"Available",', '"status":"1.2 MB (300 KB/s)","statusId":"downloading",');
    expect(downloadCard(read(downloading).entries)).toEqual(['Downloading log 2', '1.2 MB (300 KB/s)']);
    expect(downloadCard(read(served).entries)).toEqual(['Downloading', '']);
  });

  it("the entries carry the core's formatted size and time", () => {
    const logs = read(served);
    expect(logs.entries.map((it) => it.sizeStr)).toEqual(['4.0KB', '10.0KB']);
    expect(logs.entries.every((it) => it.time.trim() !== '' && it.time !== 'Date unknown')).toBe(true);
    expect(logs.entries.map((it) => it.id)).toEqual([1, 2]);
    expect(logs.entries.map((it) => it.status)).toEqual(['Downloaded', 'Available']);
  });

  it("the buttons follow the core rather than the head's own arithmetic", () => {
    const logs = read(served);
    expect(logs.canRefresh).toBe(true);
    expect(logs.canDownload).toBe(true);
    expect(logs.canCancel).toBe(false);
    expect(logs.anyDownloaded).toBe(true);
    expect(logs.eraseWarning).toBe('This erases every log on the vehicle.');
  });

  it('a vehicle with no logs is connected with an empty list', () => {
    const logs = read('{"connected":true,"entries":[],"emptyText":"This vehicle reports no flight logs."}');
    expect(logs.connected).toBe(true);
    expect(logs.entries).toEqual([]);
    expect(logs.emptyText).toBe('This vehicle reports no flight logs.');
  });

  it('no view at all is no screen state', () => {
    expect(logsView(null)).toBeNull();
  });

  it('opening the page refreshes the list of a connected vehicle unless a transfer is running', () => {
    expect(shouldAutoRefreshLogs(true, false)).toBe(true);
    expect(shouldAutoRefreshLogs(true, true)).toBe(false);
    expect(shouldAutoRefreshLogs(false, false)).toBe(false);
  });
});

describe('LogTimeTextTest', () => {
  const fixed = (_: Date): string => 'FORMATTED';

  it('an unreceived entry shows no time', () => {
    expect(logTimeText('2026-09-10T12:00:00Z', TIME_UNRECEIVED, fixed)).toBe('');
  });

  it('a vehicle with an unset clock says so rather than showing 1970', () => {
    expect(logTimeText('1970-01-01T00:00:00Z', TIME_UNKNOWN, fixed)).toBe('Date unknown');
  });

  it('a known time is handed to the local formatter', () => {
    expect(logTimeText('2026-09-10T12:00:00Z', 'known', fixed)).toBe('FORMATTED');
  });

  it('an unparseable time does not crash the list', () => {
    expect(logTimeText('not a time', 'known', fixed)).toBe('Date unknown');
  });

  it('a Qt local time with no zone designator is what the bridge actually sends', () => {
    const seen = { times: [] as Date[] };
    logTimeText('2026-09-08T01:20:00', 'known', (it) => {
      seen.times = [...seen.times, it];
      return 'x';
    });
    expect(seen.times.map((it) => it.getTime())).toEqual([new Date(2026, 8, 8, 1, 20, 0).getTime()]);
  });

  it('an offset form is converted rather than rejected', () => {
    expect(logTimeText('2026-09-08T01:20:00+03:00', 'known', fixed)).toBe('FORMATTED');
  });
});

describe('LogTimeZoneTest', () => {
  it('a zone-less time is wall clock and must not be read as UTC', () => {
    expect(logLocalTime('2026-09-08T14:42:51')?.getTime()).toBe(new Date(2026, 8, 8, 14, 42, 51).getTime());
  });

  it("an offset time is converted into the reader's zone", () => {
    expect(logLocalTime('2026-09-08T14:42:51+04:00')?.getTime()).toBe(Date.UTC(2026, 8, 8, 10, 42, 51));
  });
});

describe('LogSavePathTest', () => {
  const logs = (anyDownloaded: boolean, savePath: string, reason: string) =>
    logsView(parseJson(`{"kind":"object","class":"LogDownload","entries":[],"busy":false,"anyDownloaded":${anyDownloaded},"savePath":${savePath},"savePathReason":${reason}}`));

  it('nothing downloaded says nothing about where it went', () => {
    expect(savedToText(logs(false, '"/sdcard/Logs"', 'null'))).toBeNull();
    expect(savedToText(null)).toBeNull();
  });

  it('a known directory is named', () => {
    expect(savedToText(logs(true, '"/sdcard/Logs"', 'null'))).toBe('Saved to /sdcard/Logs');
  });

  it('an empty path with a reason says the reason, because the two empties differ', () => {
    expect(savedToText(logs(true, '""', '"The folder chosen for logs is no longer there."'))).toBe('The folder chosen for logs is no longer there.');
  });

  it('an empty path with no reason stays silent rather than printing Saved to nowhere', () => {
    expect(savedToText(logs(true, '""', 'null'))).toBeNull();
  });

  it('a timestamp that parses as neither shape is null rather than a wrong date', () => {
    expect(logLocalTime('')).toBeNull();
    expect(logLocalTime('not a date')).toBeNull();
  });

  it('a log time reads compactly like the Penpot list', () => {
    const today = new Date(2026, 9, 3);
    expect(compactLogTime(new Date(2026, 9, 2, 9, 0), today)).toBe('Yesterday');
    expect(compactLogTime(new Date(2026, 8, 8, 2, 20), today)).toBe('Sep 8');
    expect(compactLogTime(new Date(2025, 8, 8, 2, 20), today)).toBe('Sep 8, 2025');
    expect(compactLogTime(new Date(2026, 9, 3, 14, 2), today)).not.toContain('Oct');
  });
});
