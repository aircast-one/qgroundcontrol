import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { escCellText, escSummary, splitFocus } from './EscIndicator';

describe('EscIndicatorTest', () => {
  it("the cell and page read the core's ESC summary", () => {
    const summary = escSummary(
      parseJson(
        '{"shown":true,"onlineCount":4,"healthy":false,"healthText":"ERR","healthyMotorsText":"4/4","totalErrors":3,"hints":["Motor 3 is at 92 C, past the 90 C limit.","ESC 8 matches no motor"],' +
          '"motors":[{"title":"Motor 3","motor":3,"level":"critical","reasons":["Motor 3 is at 92 C, past the 90 C limit."],"rpm":"4200 rpm","temperature":"92 C","voltage":"16 V","current":"","errors":"1"},{"title":"ESC 8","motor":null,"level":"normal"}]}',
      ),
    )!;
    expect(escCellText(summary)).toBe('ESC 4 ERR');
    expect(summary.hints).toEqual(['Motor 3 is at 92 C, past the 90 C limit.', 'ESC 8 matches no motor']);
    expect(summary.motors.map((motor) => [motor.title, motor.motor, motor.level])).toEqual([
      ['Motor 3', 3, 'critical'],
      ['ESC 8', null, 'normal'],
    ]);
    expect(summary.motors[0]!.reasons, 'the motor repeats its own line from the summary').toEqual(['Motor 3 is at 92 C, past the 90 C limit.']);
    expect(summary.motors[1]!.reasons).toEqual([]);
    expect(summary.motors[0]!.rows[0]).toEqual(['RPM', '4200 rpm']);
    expect(escSummary(parseJson('{"shown":false}'))).toBeNull();
  });

  it('a tapped rotor is pulled out to lead the panel, and the rest keep the order the core gave them', () => {
    const motor = (title: string, number: number | null) => ({ title, motor: number, level: 'normal' as const, reasons: [], rows: [] });
    const listed = [motor('Motor 5', 5), motor('Motor 2', 2), motor('ESC 8', null)];
    const tapped = splitFocus(listed, 2);
    expect([tapped.focused?.title, tapped.rest.map((it) => it.title)]).toEqual(['Motor 2', ['Motor 5', 'ESC 8']]);
    expect(splitFocus(listed, null)).toEqual({ focused: null, rest: listed });
    expect(splitFocus(listed, 4), 'a rotor with no ESC entry focuses nothing').toEqual({ focused: null, rest: listed });
  });
});
