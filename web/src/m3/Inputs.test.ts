// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { createElement } from 'react';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { OutlinedTextField, stepsFor } from './Inputs';

describe('stepsFor gives a slider the keyboard step its caller rounds to', () => {
  it('counts the stops between the ends', () => {
    expect(stepsFor(0, 500, 1)).toBe(499);
    expect(stepsFor(0, 359, 1)).toBe(358);
    expect(stepsFor(0, 10, 0.01)).toBe(999);
    expect(stepsFor(5, 5, 1)).toBe(0);
  });
});

describe('a text field that grows to several lines', () => {
  afterEach(cleanup);

  const growing = (onDone?: () => void) =>
    render(createElement(OutlinedTextField, { value: 'caster.example.com', onValueChange: () => undefined, singleLine: false, minLines: 1, ariaLabel: 'Host address', onDone }));

  it('is set by Enter like a one-line field, and Shift+Enter still breaks the line', () => {
    const done = vi.fn();
    growing(done);
    const box = screen.getByRole('textbox', { name: 'Host address' });
    expect(fireEvent.keyDown(box, { key: 'Enter', shiftKey: true })).toBe(true);
    expect(done).not.toHaveBeenCalled();
    expect(fireEvent.keyDown(box, { key: 'Enter' })).toBe(false);
    expect(done).toHaveBeenCalledOnce();
  });

  it('without a done action Enter breaks the line', () => {
    growing();
    expect(fireEvent.keyDown(screen.getByRole('textbox', { name: 'Host address' }), { key: 'Enter' })).toBe(true);
  });
});
