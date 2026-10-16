import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { escCellText, escSummary } from './EscIndicator';

describe('EscIndicatorTest', () => {
  it("the cell and page read the core's ESC summary", () => {
    const summary = escSummary(
      parseJson(
        '{"shown":true,"onlineCount":4,"healthy":true,"healthText":"OK","healthyMotorsText":"4/4","totalErrors":3,' +
          '"motors":[{"title":"Motor 1","healthy":true,"rpm":"4200 rpm","temperature":"","voltage":"16 V","current":"","errors":"1"}]}',
      ),
    )!;
    expect(escCellText(summary)).toBe('ESC 4 OK');
    expect(summary.motors[0]!.title).toBe('Motor 1');
    expect(summary.motors[0]!.rows[0]).toEqual(['RPM', '4200 rpm']);
    expect(escSummary(parseJson('{"shown":false}'))).toBeNull();
  });
});
