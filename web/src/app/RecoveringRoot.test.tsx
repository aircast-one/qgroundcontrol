// @vitest-environment jsdom
import { act, fireEvent, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { mutableStateOf } from '@/kotlin/State';
import { CRASHED_TITLE, type PageFacts, RELOAD_KEEPS_LINK, RELOAD_RESTARTS, pageErrorLine, recordPageErrors, recoveringRoot } from './RecoveringRoot';

const failing = mutableStateOf(true);

const FlyView = () => {
  if (failing.value) throw new Error('motor 3 has no reading');
  return <p>Fly view</p>;
};

const DESKTOP: PageFacts = { screen: () => '{"tab":"Fly","flyView":"Map"}', desktop: true, version: '5.6.5-beta.58', sha: 'c01f87288c' };

describe('a screen that throws while drawing never leaves the window blank', () => {
  beforeEach(() => {
    document.body.innerHTML = '<div id="root"></div>';
    vi.stubGlobal('IS_REACT_ACT_ENVIRONMENT', false);
    vi.spyOn(console, 'error').mockImplementation(() => undefined);
  });
  afterEach(() => {
    vi.unstubAllGlobals();
    vi.restoreAllMocks();
  });

  it('draws the app again after one failure and puts one page_error event in the app log', async () => {
    failing.value = true;
    const lines: string[] = [];
    const show = recoveringRoot(
      document.getElementById('root')!,
      (line) => {
        lines.push(line);
        failing.value = false;
      },
      DESKTOP,
    );
    show(<FlyView />);
    expect(await screen.findByText('Fly view')).toBeTruthy();
    expect(lines).toHaveLength(1);
    expect(lines[0]).toMatch(
      /^event=page_error kind=render reason="Error: motor 3 has no reading" stack="Error: motor 3 has no reading < at FlyView .*" components="at FlyView .*" screen=\{"tab":"Fly","flyView":"Map"\} head=desktop version=5\.6\.5-beta\.58 sha=c01f87288c uptime_s=\d+\.\d$/,
    );
  });

  it('shows the error with Reload and Copy details when the app fails again straight after coming back', async () => {
    failing.value = true;
    const lines: string[] = [];
    const copied = vi.fn(() => Promise.resolve());
    vi.stubGlobal('navigator', { clipboard: { writeText: copied } });
    const show = recoveringRoot(document.getElementById('root')!, (line) => lines.push(line), DESKTOP);
    show(<FlyView />);
    expect(await screen.findByText(CRASHED_TITLE)).toBeTruthy();
    expect(lines).toHaveLength(2);
    expect(screen.getByText(RELOAD_KEEPS_LINK)).toBeTruthy();
    expect(screen.getByText('Error: motor 3 has no reading')).toBeTruthy();
    expect(screen.getByRole('button', { name: 'Reload' })).toBeTruthy();
    await act(async () => fireEvent.click(screen.getByRole('button', { name: 'Copy details' })));
    expect(copied).toHaveBeenCalledWith(lines[1]);
    expect(screen.getByRole('button', { name: 'Copied' })).toBeTruthy();
  });

  it('does not promise a kept link in the browser, where a reload restarts the core', async () => {
    failing.value = true;
    const show = recoveringRoot(document.getElementById('root')!, () => undefined, { ...DESKTOP, desktop: false });
    show(<FlyView />);
    expect(await screen.findByText(CRASHED_TITLE)).toBeTruthy();
    expect(screen.getByText(RELOAD_RESTARTS)).toBeTruthy();
  });
});

describe('an error outside drawing is recorded too', () => {
  it('logs an uncaught error and an unhandled rejection with what the pilot was looking at', () => {
    const lines: string[] = [];
    recordPageErrors((line) => lines.push(line), DESKTOP);
    window.dispatchEvent(new ErrorEvent('error', { error: new TypeError('video is null'), message: 'TypeError: video is null' }));
    window.dispatchEvent(Object.assign(new Event('unhandledrejection'), { reason: 'The 3D view could not start.' }));
    expect(lines).toHaveLength(2);
    expect(lines[0]).toMatch(/^event=page_error kind=error reason="TypeError: video is null" stack="TypeError: video is null < .*" components="" screen=\{"tab":"Fly","flyView":"Map"\} head=desktop /);
    expect(lines[1]).toMatch(/^event=page_error kind=rejection reason="The 3D view could not start\." stack="" components="" screen=/);
  });

  it('keeps a quote inside the message from ending its field early', () => {
    expect(pageErrorLine('error', new Error('no "camera"'), undefined, { ...DESKTOP, desktop: false }, 1250)).toMatch(/^event=page_error kind=error reason="Error: no 'camera'" .* head=browser version=5\.6\.5-beta\.58 sha=c01f87288c uptime_s=1\.3$/);
  });
});
