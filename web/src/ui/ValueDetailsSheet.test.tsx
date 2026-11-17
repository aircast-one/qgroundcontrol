// @vitest-environment jsdom
import { act, cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { Qgc, makeFact } from '@/bridge/Qgc';
import { ValueDetailsSheet } from '@/ui/ParameterLinks';
import { FactRow } from '@/ui/SettingsScreen';
import { type ChangeNotice, LocalChangeNotice } from '@/ui/ValueRow';

const fact = makeFact({ path: 'parameter.RTL_ALT_M', name: 'RTL_ALT_M', description: 'RTL Altitude', units: 'm', valueString: '15', value: 15, enumStrings: [], enumIndex: -1, isBool: false, isString: false, readOnly: false });

const notices = (): ChangeNotice & { shown: string[] } => {
  const shown: string[] = [];
  return { shown, show: (message) => void shown.push(message) };
};

const field = () => screen.getByRole('textbox', { name: 'RTL Altitude' });

const settle = () => act(() => Promise.resolve());

describe('typed values reach the aircraft only on an explicit save', () => {
  beforeEach(() => {
    vi.spyOn(Qgc, 'writeRefusal').mockResolvedValue(null);
    vi.spyOn(Qgc, 'invokeResult').mockResolvedValue(null);
  });

  afterEach(() => {
    cleanup();
    vi.restoreAllMocks();
  });

  it('closing the sheet with a half-typed value writes nothing', async () => {
    const dismissed = vi.fn();
    const view = render(<ValueDetailsSheet fact={makeFact({ ...fact, slider: null })} title="RTL altitude" onWrite={() => undefined} onDismiss={dismissed} />);
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '2' } });
    fireEvent.keyDown(field(), { key: 'Escape' });
    expect(field()).toHaveProperty('value', '15');
    fireEvent.change(field(), { target: { value: '2' } });
    view.unmount();
    await settle();
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
  });

  it('cancel discards the typed value and save writes it with an undo notice', async () => {
    const notice = notices();
    const dismissed = vi.fn();
    render(
      <LocalChangeNotice.Provider value={notice}>
        <ValueDetailsSheet fact={fact} title="RTL altitude" onWrite={() => undefined} onDismiss={dismissed} />
      </LocalChangeNotice.Provider>,
    );
    fireEvent.click(screen.getByTitle('Type a value'));
    fireEvent.change(field(), { target: { value: '20' } });
    fireEvent.click(screen.getByRole('button', { name: 'Save' }));
    await settle();
    await settle();
    expect(Qgc.writeRefusal).toHaveBeenCalledWith('parameter.RTL_ALT_M', '20');
    expect(notice.shown).toEqual(['RTL altitude: 20 m']);
    expect(dismissed).toHaveBeenCalledTimes(1);
  });

  it('a field in a list waits for Set or Enter and never writes on blur', async () => {
    const notice = notices();
    render(
      <LocalChangeNotice.Provider value={notice}>
        <FactRow fact={fact} />
      </LocalChangeNotice.Provider>,
    );
    fireEvent.change(field(), { target: { value: '5000' } });
    fireEvent.blur(field());
    await settle();
    expect(Qgc.writeRefusal).not.toHaveBeenCalled();
    fireEvent.click(screen.getByRole('button', { name: 'Set' }));
    await settle();
    await settle();
    expect(Qgc.writeRefusal).toHaveBeenCalledWith('parameter.RTL_ALT_M', '5000');
    expect(notice.shown).toEqual(['RTL altitude: 5000 m']);
  });
});
