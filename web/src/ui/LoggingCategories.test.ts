import { describe, expect, it } from 'vitest';
import { filteredCategories, logCategories, parentNames, parentOf, shownInTree } from './LoggingCategories';

const served = {
  active: ['groundstation::hub'],
  categories: [
    { name: 'groundstation', shortName: 'groundstation', depth: 0, enabled: false },
    { name: 'groundstation::hub', shortName: 'hub', depth: 1, enabled: true },
    { name: 'ureq', shortName: 'ureq', depth: 0, enabled: false },
  ],
};

describe('LoggingCategoriesTest', () => {
  it('the tree reads with its depths and the active list', () => {
    const read = logCategories(served)!;
    expect(read.active).toEqual(['groundstation::hub']);
    expect(read.categories.map((it) => it.depth)).toEqual([0, 1, 0]);
    expect(read.categories.map((it) => it.enabled)).toEqual([false, true, false]);
  });

  it('searching matches anywhere in the full name, ignoring case', () => {
    const read = logCategories(served)!;
    expect(filteredCategories(read.categories, 'HUB').map((it) => it.name)).toEqual(['groundstation::hub']);
    expect(filteredCategories(read.categories, '  ').map((it) => it.name), 'QSortFilterProxyModel::setFilterFixedString matches the text as typed').toEqual([]);
  });

  it('the tree starts collapsed, lists children under their parent and opens one level per expanded parent like TreeView', () => {
    const read = logCategories({
      active: [],
      categories: [
        { name: 'a', shortName: 'a', depth: 0, enabled: false },
        { name: 'a1', shortName: 'a1', depth: 0, enabled: false },
        { name: 'a::b', shortName: 'b', depth: 1, enabled: false },
        { name: 'a::b::c', shortName: 'c', depth: 2, enabled: false },
        { name: 'a1::x', shortName: 'x', depth: 1, enabled: false },
      ],
    })!;
    expect(parentNames(read.categories)).toEqual(new Set(['a', 'a::b', 'a1']));
    expect(parentOf(read.categories, read.categories.at(-1)!)?.name).toBe('a1');
    expect(shownInTree(read.categories, new Set()).map((it) => it.name)).toEqual(['a', 'a1']);
    expect(shownInTree(read.categories, new Set(['a'])).map((it) => it.name)).toEqual(['a', 'a::b', 'a1']);
    expect(shownInTree(read.categories, new Set(['a::b'])).map((it) => it.name), 'a child stays hidden while its parent is collapsed').toEqual(['a', 'a1']);
    expect(shownInTree(read.categories, new Set(['a', 'a::b'])).map((it) => it.name)).toEqual(['a', 'a::b', 'a::b::c', 'a1']);
  });
});
