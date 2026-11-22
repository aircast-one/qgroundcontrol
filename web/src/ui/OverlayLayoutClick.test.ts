// @vitest-environment jsdom
import { afterEach, describe, expect, it } from 'vitest';
import { endsLayoutEditing } from './OverlayLayout';

const add = (html: string): Element => {
  const holder = document.createElement('div');
  holder.innerHTML = html;
  document.body.append(holder);
  return holder;
};

describe('OverlayLayoutClickTest', () => {
  afterEach(() => {
    document.body.innerHTML = '';
  });

  it('a click on empty space ends layout editing', () => {
    const empty = add('<div><span>picture</span></div>').querySelector('span');
    expect(endsLayoutEditing(empty, 300, 300)).toBe(true);
    expect(endsLayoutEditing(null, 300, 300)).toBe(true);
  });

  it('a click on a layout item or a control that still works keeps editing', () => {
    const page = add('<div data-layout-item><span id="reading">3.0 m</span></div><button id="land"><span id="label">Land</span></button><div role="slider" id="slide"></div>');
    expect(endsLayoutEditing(page.querySelector('#reading'), 300, 300)).toBe(false);
    expect(endsLayoutEditing(page.querySelector('#label'), 300, 300)).toBe(false);
    expect(endsLayoutEditing(page.querySelector('#slide'), 300, 300)).toBe(false);
  });

  it('with a menu or sheet open, a click outside it only closes that, not the editing', () => {
    const page = add('<div role="menu"><div role="menuitem">Reset positions</div></div><span id="empty">picture</span>');
    expect(endsLayoutEditing(page.querySelector('#empty'), 300, 300)).toBe(false);
  });
});
