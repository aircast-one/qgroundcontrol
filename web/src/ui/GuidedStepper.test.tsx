// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { GuidedStepper } from './GuidedValuePanel';

afterEach(cleanup);

describe('GuidedStepper', () => {
  it('names its buttons and its typed field by what they set, not by a bare minus, plus or number', () => {
    const onValue = vi.fn();
    render(<GuidedStepper value={3} label="Height above launch" unit="m" minimum={3} maximum={500} onValue={onValue} />);
    fireEvent.click(screen.getByRole('button', { name: 'Increase Height above launch' }));
    expect(onValue).toHaveBeenCalledWith(4);
    expect(screen.getByRole('button', { name: 'Decrease Height above launch' })).toBeTruthy();
    fireEvent.click(screen.getByRole('button', { name: 'Height above launch: 3.0 m, type a value' }));
    expect(screen.getByRole('textbox', { name: 'Height above launch' })).toBeTruthy();
  });
});
