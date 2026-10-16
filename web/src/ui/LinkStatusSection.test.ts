import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { linkStatusRows } from './LinkStatusSection';

describe('LinkStatusSectionTest', () => {
  it('link status rows read in order', () => {
    const view = parseJson('{"rows":[{"label":"Messages Sent","value":"62"},{"label":"Loss Rate","value":"3%"}]}');
    expect(linkStatusRows(view)).toEqual([
      ['Messages Sent', '62'],
      ['Loss Rate', '3%'],
    ]);
    expect(linkStatusRows(null)).toEqual([]);
  });
});
