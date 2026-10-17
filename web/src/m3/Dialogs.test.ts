import { describe, expect, it } from 'vitest';
import { anchorPlacement } from './Dialogs';

describe('DialogsTest', () => {
  const viewport = { width: 1440, height: 900 };

  it('an anchored menu opens below a click in the upper part of the screen', () => {
    expect(anchorPlacement({ x: 400, y: 200 }, viewport)).toEqual({ '--anchor-left': '400px', '--anchor-top': '200px' });
  });

  it('near the bottom it opens upwards from the click, and it never runs off the right or left edge', () => {
    expect(anchorPlacement({ x: 1400, y: 800 }, viewport)).toEqual({ '--anchor-left': '1104px', '--anchor-bottom': '100px' });
    expect(anchorPlacement({ x: 2, y: 10 }, viewport)['--anchor-left']).toBe('16px');
  });
});
