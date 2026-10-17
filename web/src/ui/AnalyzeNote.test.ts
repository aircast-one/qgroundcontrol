import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { AnalyzePage, AnalyzePageEntries, analyzeGate, analyzeNote, analyzeStatus, analyzeSubtitle } from './AnalyzeScreen';
import { inspectorRateText } from './InspectorScreen';
import { SetupState } from './SetupRow';
import { MessageSeverity, VehicleMessage } from './VehicleMessages';
import { type VibrationAxis, type VibrationReading, vibrationCaveat } from './VibrationScreen';

const NONE = 'This vehicle is not reporting vibration';

describe('AnalyzeNoteTest', () => {
  it('the console row carries no firmware note, as the AnalyzeView page list has none', () => {
    expect(analyzeNote(AnalyzePage.Console, true, null)).toBeNull();
  });

  it('a vibration row says when the vehicle is sending none', () => {
    expect(analyzeNote(AnalyzePage.Vibration, true, NONE)).toBe(NONE);
    expect(analyzeNote(AnalyzePage.Vibration, true, null)).toBeNull();
  });

  it('the row and the screen answer partly-reported from one source', () => {
    const view = (available: boolean, reason: string) => parseJson(`{"available":${available},"silentReason":${reason}}`);

    expect(vibrationCaveat(view(false, '"notReported"'))).toBe('This vehicle is not reporting vibration');
    expect(vibrationCaveat(view(false, 'null'))).toBe('This vehicle is reporting only some vibration axes');
    expect(vibrationCaveat(view(true, 'null'))).toBeNull();
    expect(vibrationCaveat(null)).toBeNull();
  });

  it('with no vehicle every row stays quiet rather than repeating the same sentence five times', () => {
    expect(AnalyzePageEntries.map((page) => analyzeNote(page, false, NONE))).toEqual(AnalyzePageEntries.map(() => null));
  });

  it('the rows this head cannot answer for carry nothing', () => {
    expect([AnalyzePage.LogDownload, AnalyzePage.Inspector, AnalyzePage.GeoTag].map((page) => analyzeNote(page, true, NONE))).toEqual([null, null, null]);
  });

  it('theListShowsUnreadMessagesAndTheVibrationVerdict', () => {
    expect(analyzeStatus(AnalyzePage.Messages, 3, null)).toEqual(['3', SetupState.NeedsAttention]);
    expect(analyzeStatus(AnalyzePage.Messages, 0, null)).toEqual(['', SetupState.Neutral]);
    expect(analyzeStatus(AnalyzePage.Vibration, 0, 'danger')).toEqual(['Unsafe', SetupState.NeedsAttention]);
    expect(analyzeStatus(AnalyzePage.Vibration, 0, 'normal')).toEqual(['Healthy', SetupState.Done]);
    expect(analyzeStatus(AnalyzePage.Console, 5, 'danger')).toEqual(['', SetupState.Neutral]);
  });

  it('the messages row reads what the vehicle has said, the others what the tool does', () => {
    const said = [
      VehicleMessage({ index: 0, time: '', severity: '', level: MessageSeverity.Warning, text: 'Low battery' }),
      VehicleMessage({ index: 1, time: '', severity: '', level: MessageSeverity.Normal, text: 'Armed' }),
    ];
    expect(analyzeSubtitle(AnalyzePage.Messages, said)).toBe('1 warning · 1 message');
    expect(analyzeSubtitle(AnalyzePage.Messages, [])).toBe(AnalyzePage.Messages.description);
    expect(analyzeSubtitle(AnalyzePage.Console, said)).toBe(AnalyzePage.Console.description);
  });

  it('the vibration row reads the three axes in their unit', () => {
    const axis = (name: string, value: number | null): VibrationAxis => ({ axis: name, value, fraction: 0, severity: null });
    const reading: VibrationReading = { units: 'm/s²', scaleMaximum: 60.0, warningLevel: 30.0, dangerLevel: 60.0, axes: [axis('X', 12.4), axis('Y', 8.6), axis('Z', 18.0)], clipCounts: [] };
    expect(analyzeSubtitle(AnalyzePage.Vibration, [], reading)).toBe('X 12 · Y 9 · Z 18 m/s²');
    expect(analyzeSubtitle(AnalyzePage.Vibration, [], { ...reading, axes: [axis('X', null)] })).toBe(AnalyzePage.Vibration.description);
    expect(analyzeSubtitle(AnalyzePage.Vibration, [], null)).toBe(AnalyzePage.Vibration.description);
  });

  it('the inspector row reads the total message rate', () => {
    expect(inspectorRateText(parseJson('{"available":true,"messages":[{"rateHz":5.0},{"rateHz":10.4},{"rateHz":0}]}'))).toBe('15 messages/s');
    expect(inspectorRateText(parseJson('{"available":true,"messages":[]}'))).toBeNull();
    expect(analyzeSubtitle(AnalyzePage.Inspector, [], null, null)).toBe(AnalyzePage.Inspector.description);
  });

  it('the pages QGCCorePlugin marks requiresVehicle say so with no vehicle, like AnalyzeView', () => {
    expect(new Set(AnalyzePageEntries.filter((it) => analyzeGate(it, false) !== null))).toEqual(new Set([AnalyzePage.LogDownload, AnalyzePage.Vibration, AnalyzePage.Inspector, AnalyzePage.Console]));
    expect(analyzeGate(AnalyzePage.Console, false)).toBe('Requires a connected vehicle');
    expect(AnalyzePageEntries.map((it) => analyzeGate(it, true))).toEqual(AnalyzePageEntries.map(() => null));
  });
});
