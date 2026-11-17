// @vitest-environment jsdom
import { cleanup, fireEvent, render } from '@testing-library/react';
import { useState } from 'react';
import { afterEach, describe, expect, it } from 'vitest';
import { trackFocus, useFocusReturn } from './FocusReturn';

const Panel = () => {
  useFocusReturn();
  return <button type="button">inside</button>;
};

const App = () => {
  const [open, setOpen] = useState(false);
  const [menu, setMenu] = useState(false);
  return (
    <div>
      <button type="button" onClick={() => setMenu(true)}>
        Menu
      </button>
      {menu ? (
        <div role="dialog">
          <button
            type="button"
            onClick={() => {
              setMenu(false);
              setOpen(true);
            }}
          >
            Settings
          </button>
        </div>
      ) : null}
      {open ? (
        <div>
          <Panel />
          <button type="button" onClick={() => setOpen(false)}>
            Close
          </button>
        </div>
      ) : null}
    </div>
  );
};

describe('FocusReturnTest', () => {
  afterEach(cleanup);

  it('closing a page opened from a menu puts focus back on the menu button, not on the page body', () => {
    trackFocus();
    const view = render(<App />);
    const menu = view.getByText('Menu');
    menu.focus();
    fireEvent.click(menu);
    const item = view.getByText('Settings');
    item.focus();
    fireEvent.click(item);
    view.getByText('Close').focus();
    fireEvent.click(view.getByText('Close'));
    expect(document.activeElement).toBe(menu);
  });
});
