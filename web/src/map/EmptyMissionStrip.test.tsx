// @vitest-environment jsdom
import { cleanup, render } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { EmptyMissionStrip, FIRST_TAP_SETS_HOME } from './PlanStrip';

afterEach(cleanup);

describe('EmptyMissionStripTest', () => {
  it('says the first click sets home only when it does, not when a connected aircraft supplies home', () => {
    const shown = (setsHome: boolean) => render(<EmptyMissionStrip setsHome={setsHome} onTemplates={null} onDownload={null} />).container.textContent ?? '';
    expect(shown(true)).toContain(FIRST_TAP_SETS_HOME);
    cleanup();
    expect(shown(false)).not.toContain(FIRST_TAP_SETS_HOME);
  });
});
