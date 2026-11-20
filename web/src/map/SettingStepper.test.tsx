// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { SettingStepper } from './SettingStepper';

beforeEach(() => {
  Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }) });
});

afterEach(cleanup);

describe('SettingStepper', () => {
  it('Enter in the typed value sets it like the Set button', () => {
    const onSet = vi.fn();
    render(<SettingStepper label="Altitude" value={50} unit="m" step={1} onSet={onSet} />);
    fireEvent.click(screen.getByRole('button', { name: 'Altitude: 50 m' }));
    const field = screen.getByRole('textbox', { name: 'Altitude in m' });
    fireEvent.change(field, { target: { value: '120' } });
    fireEvent.keyDown(field, { key: 'Enter' });
    expect(onSet).toHaveBeenCalledWith(120);
    expect(screen.queryByRole('textbox')).toBeNull();
  });
});
