// @vitest-environment jsdom
import { screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mutableStateOf } from '@/kotlin/State';
import { CRASHED_TITLE, recoveringRoot } from './RecoveringRoot';

const failing = mutableStateOf(true);

const FlyView = () => {
  if (failing.value) throw new Error('motor 3 has no reading');
  return <p>Fly view</p>;
};

describe('a screen that throws while drawing never leaves the window blank', () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="root"></div>';
    vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', false);
    vi.stubGlobal('reportError', vi.fn());
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('draws the app again after one failure and puts the error and the component that threw in the app log', async () => {
    failing.value = true;
    const lines: string[] = [];
    const show = recoveringRoot(document.getElementById('root')!, (line) => {
      lines.push(line);
      failing.value = false;
    });
    show(<FlyView />);
    expect(await screen.findByText('Fly view')).toBeTruthy();
    expect(lines).toHaveLength(1);
    expect(lines[0]).toMatch(/^event=ui_crash reason="Error: motor 3 has no reading" stack="Error: motor 3 has no reading < at FlyView .*" components="at FlyView /);
    expect(reportError).toHaveBeenCalledTimes(1);
  });

  it('shows the error with a Reload button when the app fails again straight after coming back', async () => {
    failing.value = true;
    const lines: string[] = [];
    const show = recoveringRoot(document.getElementById('root')!, (line) => lines.push(line));
    show(<FlyView />);
    expect(await screen.findByText(CRASHED_TITLE)).toBeTruthy();
    expect(lines).toHaveLength(2);
    expect(screen.getByText('Error: motor 3 has no reading')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Reload' })).toBeTruthy();
  });
});
