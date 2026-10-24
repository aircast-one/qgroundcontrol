import { describe, expect, it } from 'vitest';
import { accepted, edited, reconciled, settled, shownValues } from '@/ui/DeviceDraft';

const before = { name: 'old', apn: 'internet' };

describe('DeviceDraft', () => {
  it('a field the user has not touched follows the device', () => {
    const drafts = edited({}, 'name', 'typed');
    expect(shownValues({ name: 'old', apn: 'telenor' }, reconciled(drafts, before, { name: 'old', apn: 'telenor' }))).toEqual({ name: 'typed', apn: 'telenor' });
  });

  it('a view that still carries the old value leaves the typed value alone', () => {
    const drafts = accepted(edited({}, 'name', 'new'), { name: 'new' });
    expect(reconciled(drafts, before, { ...before })).toEqual(drafts);
    expect(shownValues(before, drafts)).toMatchObject({ name: 'new' });
  });

  it('the draft is done once the device reports the saved value', () => {
    const drafts = accepted(edited({}, 'name', 'new'), { name: 'new' });
    expect(reconciled(drafts, before, { ...before, name: 'new' })).toEqual({});
  });

  it('what is typed after a save survives the device reporting the save', () => {
    const typedAfter = edited(accepted(edited({}, 'name', 'new'), { name: 'new' }), 'name', 'newer');
    expect(reconciled(typedAfter, before, { ...before, name: 'new' })).toEqual({ name: { value: 'newer', sent: false } });
  });

  it('a value typed back to what the device still shows is kept while a save may still land', () => {
    const drafts = edited({}, 'name', 'old');
    expect(reconciled(drafts, before, { ...before, apn: 'telenor' })).toEqual(drafts);
  });

  it('only what was sent and not edited since counts as sent', () => {
    const drafts = accepted(edited(edited({}, 'name', 'new'), 'apn', 'typed-later'), { name: 'new', apn: 'internet' });
    expect(drafts).toEqual({ name: { value: 'new', sent: true }, apn: { value: 'typed-later', sent: false } });
    expect(settled(drafts)).toEqual({ apn: { value: 'typed-later', sent: false } });
  });

  it('a second tap before the first one shows up is held through the first one arriving', () => {
    const first = accepted(edited({}, 'checked', true), { checked: true });
    const second = accepted(edited(first, 'checked', false), { checked: false });
    const throughFirst = reconciled(second, { checked: false }, { checked: true });
    expect(shownValues({ checked: true }, throughFirst)).toEqual({ checked: false });
    expect(reconciled(throughFirst, { checked: true }, { checked: false })).toEqual({});
  });

  it('rows compare by what they hold, not by identity', () => {
    const rows = [{ url: 'turn:relay.example.com:3478', username: 'drone', password: '' }];
    const drafts = accepted(edited({}, 'rows', rows), { rows });
    expect(reconciled(drafts, { rows: [{ url: '', username: '', password: '' }] }, { rows: rows.map((row) => ({ ...row })) })).toEqual({});
  });
});
