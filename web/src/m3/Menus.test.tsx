// @vitest-environment jsdom
import { Profiler } from 'react';
import { act, cleanup, render, screen } from '@testing-library/react';
import { afterEach, describe, expect, it } from 'vitest';
import { DropdownMenu, DropdownMenuItem } from './Menus';

const marks = (item: HTMLElement): number => item.querySelectorAll('[aria-hidden="true"]').length;

describe('a choice menu shows which option is chosen', () => {
  afterEach(cleanup);

  it('marks the chosen option for screen readers with aria-current and for the eye with a mark, and leaves the others plain', async () => {
    render(
      <div>
        <DropdownMenu expanded onDismissRequest={() => undefined}>
          <DropdownMenuItem text="Multi-rotor" selected onClick={() => undefined} />
          <DropdownMenuItem text="Fixed wing" selected={false} onClick={() => undefined} />
        </DropdownMenu>
      </div>,
    );
    const chosen = await screen.findByRole('menuitem', { name: 'Multi-rotor' });
    const other = screen.getByRole('menuitem', { name: 'Fixed wing' });
    expect(chosen.getAttribute('aria-current')).toBe('true');
    expect(other.hasAttribute('aria-current')).toBe(false);
    expect(marks(chosen)).toBe(1);
    expect(marks(other)).toBe(0);
  });
});

describe('a menu stays still while the screen around it redraws', () => {
  afterEach(cleanup);

  it('commits once per redraw of its parent, never twice, so a fast-updating screen cannot spin it into a loop', () => {
    const commits: string[] = [];
    const tree = (tick: number) => (
      <div data-tick={tick}>
        <Profiler id="menu" onRender={(_id, phase) => commits.push(phase)}>
          <DropdownMenu expanded={false} onDismissRequest={() => undefined}>
            <DropdownMenuItem text="Multi-rotor" onClick={() => undefined} />
          </DropdownMenu>
        </Profiler>
      </div>
    );
    const view = render(tree(0));
    const settled = commits.length;
    act(() => view.rerender(tree(1)));
    expect(commits.length - settled).toBe(1);
  });
});
