// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { type Fact, Qgc, makeFact } from '@/bridge/Qgc';
import { ParameterEditDialog, ValueDetailsSheet } from '@/ui/ParameterLinks';
import { FactRow } from '@/ui/SettingsScreen';
import { type ChangeNotice, LocalChangeNotice, SliderValueRow, inlineSlider } from '@/ui/ValueRow';

const fact = makeFact({ path: 'parameter.RTL_ALT_M', name: 'RTL_ALT_M', description: 'RTL Altitude', units: 'm', valueString: '15', value: 15, enumStrings: [], enumIndex: -1, isBool: false, isString: false, readOnly: false });

const choice = (options: string[]) =>
  makeFact({ path: 'parameter.FS_GCS_ENABLE', name: 'FS_GCS_ENABLE', description: 'Ground station failsafe', units: '', valueString: '0', value: 0, enumStrings: options, enumValues: options.map((_, at) => String(at)), enumIndex: 0, isBool: false, isString: false, readOnly: false });

const noticeSpy = () => vi.fn<ChangeNotice['show']>();

const field = () => screen.getByRole('textbox', { name: 'RTL Altitude' });

const settle = () => act(() => Promise.resolve());

const settled = async () => {
  await settle();
  await settle();
  await settle();
};

const sheet = (shown: Fact, notice: ChangeNotice['show'] = noticeSpy(), dismissed = vi.fn()) => (
  <LocalChangeNotice.Provider value={{ show: notice }}>
    <ValueDetailsSheet fact={shown} title="RTL altitude" onWrite={() => undefined} onDismiss={dismissed} />
  </LocalChangeNotice.Provider>
);

const save = () => fireEvent.click(screen.getByRole('button', { name: 'Save' }));

beforeEach(() => {
  Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }) });
  Object.defineProperty(HTMLElement.prototype, 'setPointerCapture', { configurable: true, value: () => undefined });
  vi.spyOn(Qgc, 'writeRefusal').mockResolvedValue(null);
  vi.spyOn(Qgc, 'writeForcedRefusal').mockResolvedValue(null);
  vi.spyOn(Qgc, 'set').mockResolvedValue(true);
  vi.spyOn(Qgc, 'invokeResult').mockResolvedValue(null);
  vi.spyOn(Qgc, 'watch').mockImplementation(() => undefined);
  vi.spyOn(Qgc, 'unwatch').mockImplementation(() => undefined);
});

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('typed values reach the aircraft only on an explicit save', () => {
  it('closing the sheet with a half-typed value writes nothing', async () => {
    const view = render(sheet(makeFact({ ...fact, slider: null })));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '2' } });
    fireEvent.keyDown(field(), { key: 'Escape' });
    expect(field()).toHaveProperty('value', '15');
    fireEvent.change(field(), { target: { value: '2' } });
    view.unmount();
    await settled();
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
  });

  it('cancel discards the typed value and closes the sheet', async () => {
    const dismissed = vi.fn();
    render(sheet(fact, noticeSpy(), dismissed));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }));
    await settled();
    expect(dismissed).toHaveBeenCalledTimes(1);
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
  });

  it('save writes the typed value with an undo notice', async () => {
    const notice = noticeSpy();
    const dismissed = vi.fn();
    render(sheet(fact, notice, dismissed));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    save();
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledWith('parameter.RTL_ALT_M', '20');
    expect(notice.mock.calls.map(([message]) => message)).toEqual(['RTL altitude: 20 m']);
    expect(dismissed).toHaveBeenCalledTimes(1);
  });

  it('Enter in a sheet runs its Save, so the write is the same one Save makes', async () => {
    const dismissed = vi.fn();
    render(sheet(fact, noticeSpy(), dismissed));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    fireEvent.keyDown(field(), { key: 'Enter' });
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledExactlyOnceWith('parameter.RTL_ALT_M', '20');
    expect(dismissed).toHaveBeenCalledTimes(1);
  });

  it('a refused save keeps the sheet open and says why', async () => {
    vi.mocked(Qgc.writeRefusal).mockResolvedValue('The vehicle refused.');
    const dismissed = vi.fn();
    render(sheet(fact, noticeSpy(), dismissed));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    save();
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledWith('parameter.RTL_ALT_M', '20');
    expect(dismissed).not.toHaveBeenCalled();
    expect(screen.getByText('The vehicle refused.')).toBeTruthy();
  });

  it('a field in a list waits for Set or Enter and never writes on blur', async () => {
    const notice = noticeSpy();
    render(
      <LocalChangeNotice.Provider value={{ show: notice }}>
        <FactRow fact={fact} />
      </LocalChangeNotice.Provider>,
    );
    fireEvent.change(field(), { target: { value: '5000' } });
    fireEvent.blur(field());
    await settle();
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Set' }));
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledWith('parameter.RTL_ALT_M', '5000');
    expect(notice.mock.calls.map(([message]) => message)).toEqual(['RTL altitude: 5000 m']);
  });

  it("undo puts back the aircraft's latest value when it changed while the text was typed", async () => {
    const notice = noticeSpy();
    const view = render(sheet(fact, notice));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    view.rerender(sheet(makeFact({ ...fact, valueString: '16', value: 16 }), notice));
    save();
    await settled();
    const undo = notice.mock.calls[0]?.[1];
    expect(undo).toBeTypeOf('function');
    await act(async () => void (await undo?.()));
    expect(vi.mocked(Qgc.writeRefusal).mock.calls).toEqual([
      ['parameter.RTL_ALT_M', '20'],
      ['parameter.RTL_ALT_M', '16'],
    ]);
  });
});

describe('a settings row with a slider', () => {
  it('steps a whole metre from the arrow keys and announces the value with its unit', () => {
    const slider = inlineSlider(fact);
    if (slider === null) throw new Error('RTL altitude has a slider');
    render(<SliderValueRow fact={fact} label="Return altitude" slider={slider} title="Return altitude" onOpen={() => undefined} onWrite={() => undefined} />);
    const thumb = screen.getByRole('slider', { name: 'Return altitude' });
    expect([thumb.getAttribute('step'), thumb.getAttribute('aria-valuetext'), (thumb as HTMLInputElement).value]).toEqual(['1', '15 m', '15']);
    expect(screen.getByRole('button', { name: 'Return altitude: 15 m' })).toBeTruthy();
  });
});

describe('every control in a sheet waits for Save', () => {
  it('the steppers stay unwritten until Save, then write once', async () => {
    render(sheet(fact));
    const plus = screen.getByRole('button', { name: 'Increase RTL altitude' });
    fireEvent.pointerDown(plus, { button: 0, pointerId: 1 });
    fireEvent.pointerUp(plus, { button: 0, pointerId: 1 });
    await settled();
    expect(screen.getByTitle('Type a value').textContent).toBe('16 m');
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledExactlyOnceWith('parameter.RTL_ALT_M', 16);
  });

  it('Turn on stays unwritten until Save', async () => {
    const off = makeFact({ ...fact, path: 'parameter.MPC_XY_VEL_MAX', name: 'MPC_XY_VEL_MAX', description: 'Max speed', longDescription: 'Disabled if 0.', valueString: '0', value: 0 });
    render(sheet(off));
    fireEvent.click(screen.getByRole('button', { name: 'Turn on' }));
    await settled();
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledExactlyOnceWith('parameter.MPC_XY_VEL_MAX', 1);
  });

  it('the slider stays unwritten until Save', async () => {
    render(sheet(fact));
    const slider = screen.getByRole('slider', { name: 'RTL altitude' });
    fireEvent.focus(slider);
    fireEvent.keyDown(slider, { key: 'End' });
    await settled();
    expect(screen.getByTitle('Type a value').textContent).toBe('500 m');
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.writeRefusal).toHaveBeenCalledExactlyOnceWith('parameter.RTL_ALT_M', 500);
  });

  it('the slider starts at the value it holds, so 15 m below the useful band is not shown as 20', () => {
    render(sheet(fact));
    const slider = screen.getByRole('slider', { name: 'RTL altitude' });
    expect(slider.getAttribute('aria-valuenow')).toBe('15');
    fireEvent.keyDown(slider, { key: 'ArrowRight' });
    expect(screen.getByTitle('Type a value').textContent).toBe('16 m');
  });

  it('a step after typing replaces the typed value, and typing after a step replaces the step: one write either way', async () => {
    const first = render(sheet(fact));
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    const plus = () => screen.getByRole('button', { name: 'Increase RTL altitude' });
    fireEvent.pointerDown(plus(), { button: 0, pointerId: 1 });
    fireEvent.pointerUp(plus(), { button: 0, pointerId: 1 });
    save();
    await settled();
    expect(vi.mocked(Qgc.writeRefusal).mock.calls).toEqual([['parameter.RTL_ALT_M', 16]]);
    first.unmount();
    vi.mocked(Qgc.writeRefusal).mockClear();

    render(sheet(fact));
    fireEvent.pointerDown(plus(), { button: 0, pointerId: 1 });
    fireEvent.pointerUp(plus(), { button: 0, pointerId: 1 });
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    expect(screen.getByTitle('Type a value').textContent).toBe('15 m');
    save();
    await settled();
    expect(vi.mocked(Qgc.writeRefusal).mock.calls).toEqual([['parameter.RTL_ALT_M', '20']]);
  });

  it('a choice from the menu stays unwritten until Save', async () => {
    render(sheet(choice(['Disabled', 'Enabled always RTL', 'Enabled continue mission', 'Enabled always SmartRTL', 'Enabled always land'])));
    fireEvent.click(screen.getByRole('button', { name: 'Ground station failsafe, Disabled' }));
    fireEvent.click(await screen.findByRole('menuitem', { name: 'Enabled always land' }));
    await settled();
    expect(screen.getByRole('button', { name: 'Ground station failsafe, Enabled always land' })).toBeTruthy();
    expect(Qgc.set).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.set).toHaveBeenCalledExactlyOnceWith('parameter.FS_GCS_ENABLE.enumIndex', 4);
  });

  it('a segment stays unwritten until Save', async () => {
    render(sheet(choice(['Off', 'On'])));
    const on = () => screen.getAllByRole('radio', { name: 'On' });
    fireEvent.click(on()[0]!);
    await settled();
    expect(on().map((it) => it.getAttribute('aria-checked'))).toEqual(['true', 'true']);
    expect(Qgc.set).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.set).toHaveBeenCalledExactlyOnceWith('parameter.FS_GCS_ENABLE.enumIndex', 1);
  });

  it('a switch stays unwritten until Save', async () => {
    render(sheet(makeFact({ ...fact, path: 'parameter.ARMING_CHECK_ON', name: 'ARMING_CHECK_ON', description: 'Arming checks', units: '', valueString: '0', value: 0, isBool: true })));
    fireEvent.click(screen.getByRole('switch', { name: 'Arming checks' }));
    await settled();
    expect(screen.getByRole('switch', { name: 'Arming checks' })).toHaveProperty('ariaChecked', 'true');
    expect(Qgc.set).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.set).toHaveBeenCalledExactlyOnceWith('parameter.ARMING_CHECK_ON', true);
  });

  it('bitmask ticks stay unwritten until Save, and add up', async () => {
    render(sheet(makeFact({ ...fact, path: 'parameter.LOG_BITMASK', name: 'LOG_BITMASK', description: 'Logs', units: '', valueString: '1', value: 1, bitmaskStrings: ['Attitude', 'GPS', 'Motors'], bitmaskValues: [1, 2, 4] })));
    fireEvent.click(screen.getByRole('button', { name: 'Logs, Attitude' }));
    fireEvent.click(await screen.findByRole('checkbox', { name: 'GPS' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Motors' }));
    fireEvent.click(screen.getByRole('button', { name: 'Done' }));
    await settled();
    expect(screen.getByRole('button', { name: 'Logs, All' })).toBeTruthy();
    expect(Qgc.set).not.toHaveBeenCalled();
    save();
    await settled();
    expect(Qgc.set).toHaveBeenCalledExactlyOnceWith('parameter.LOG_BITMASK', '7');
  });
});

describe('the parameter editor', () => {
  const readOnly = {
    kind: 'fact',
    name: 'SYSID_SW_TYPE',
    shortDescription: 'Software type',
    valueString: '10',
    value: 10,
    readOnly: true,
    defaultValueAvailable: true,
    defaultValue: 0,
    defaultValueString: '0',
  };

  it('a forced value waits for the one Save, which Enter runs too', async () => {
    vi.spyOn(Qgc, 'get').mockResolvedValue(readOnly);
    const notice = noticeSpy();
    const dismissed = vi.fn();
    render(
      <LocalChangeNotice.Provider value={{ show: notice }}>
        <ParameterEditDialog name="SYSID_SW_TYPE" onDismiss={dismissed} />
      </LocalChangeNotice.Provider>,
    );
    fireEvent.click(await screen.findByRole('checkbox', { name: 'Advanced settings' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Force edit read-only param' }));
    fireEvent.change(screen.getByRole('textbox', { name: 'Value' }), { target: { value: '3' } });
    expect(screen.getAllByRole('button', { name: 'Save' })).toHaveLength(1);
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'Value' }), { key: 'Enter' });
    await settled();
    expect(Qgc.writeForcedRefusal).toHaveBeenCalledExactlyOnceWith(expect.stringContaining('SYSID_SW_TYPE'), 3);
    expect(dismissed).toHaveBeenCalledTimes(1);
  });

  it('a forced reset undoes with a forced write', async () => {
    vi.spyOn(Qgc, 'get').mockResolvedValue(readOnly);
    const notice = noticeSpy();
    render(
      <LocalChangeNotice.Provider value={{ show: notice }}>
        <ParameterEditDialog name="SYSID_SW_TYPE" onDismiss={() => undefined} />
      </LocalChangeNotice.Provider>,
    );
    fireEvent.click(await screen.findByRole('checkbox', { name: 'Advanced settings' }));
    fireEvent.click(screen.getByRole('checkbox', { name: 'Force edit read-only param' }));
    fireEvent.click(screen.getByRole('button', { name: 'Reset to default' }));
    await settled();
    const undo = notice.mock.calls[0]?.[1];
    await act(async () => void (await undo?.()));
    expect(vi.mocked(Qgc.writeForcedRefusal).mock.calls.map(([, value]) => value)).toEqual([0, '10']);
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
  });
});
