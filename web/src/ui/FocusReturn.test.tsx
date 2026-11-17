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

const Opener = ({ kept, disabled }: { kept: boolean; disabled: boolean }) => {
  const [open, setOpen] = useState(false);
  const [used, setUsed] = useState(false);
  return (
    <div>
      {kept ? (
        <button
          type="button"
          disabled={disabled && used}
          onClick={() => {
            setUsed(true);
            setOpen(true);
          }}
        >
          Open
        </button>
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

  it('when the opener is gone, closing leaves focus where it fell and does not throw', () => {
    trackFocus();
    const view = render(<Opener kept disabled={false} />);
    const opener = view.getByText('Open');
    opener.focus();
    fireEvent.click(opener);
    view.rerender(<Opener kept={false} disabled={false} />);
    view.getByText('Close').focus();
    expect(() => fireEvent.click(view.getByText('Close'))).not.toThrow();
    expect(opener.isConnected).toBe(false);
    expect(document.activeElement).toBe(document.body);
  });

  it('when the opener is disabled, focus does not land on it', () => {
    trackFocus();
    const view = render(<Opener kept disabled />);
    const opener = view.getByText('Open');
    opener.focus();
    fireEvent.click(opener);
    view.getByText('Close').focus();
    fireEvent.click(view.getByText('Close'));
    expect(opener).toHaveProperty('disabled', true);
    expect(document.activeElement).not.toBe(opener);
  });
});
