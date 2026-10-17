import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { RAW_EDIT_NOTE, commandEditable, itemNote, startCategory, wizardLines, wizardModePath } from './ItemEditor';

describe('ItemCommandTest', () => {
  it('the editor note is the command description unless raw edit is on, like SimpleItemEditor', () => {
    const view = parseJson('{"commandDescription":"Travel to a position in 3D space."}');
    expect(itemNote(view, false)).toBe('Travel to a position in 3D space.');
    expect(itemNote(view, true)).toBe(RAW_EDIT_NOTE);
    expect(itemNote(parseJson('{"commandDescription":null}'), false)).toBeNull();
  });

  it('a takeoff item offers no command change, like MissionItemEditor', () => {
    expect(['{"simple":true,"takeoff":false}', '{"simple":true,"takeoff":true}', '{"simple":false}'].map((it) => commandEditable(parseJson(it)))).toEqual([true, false, false]);
  });

  it("the picker opens on the item's category, like MissionCommandDialog", () => {
    expect(startCategory(['Basic', 'Advanced'], 'Advanced')).toBe('Advanced');
    expect(startCategory(['Basic', 'Advanced'], 'Gone')).toBe('Basic');
    expect(startCategory(['Basic', 'Advanced'], null)).toBe('Basic');
  });

  it("a plane's new takeoff shows the climb-out step until Done, like SimpleItemEditor's wizard", () => {
    const lines = wizardLines(parseJson(`{"wizardMode":true,"wizardText":["Move 'T' Takeoff to the climbout location.","Ensure clear of obstacles and into the wind."]}`));
    expect(lines).toEqual(["Move 'T' Takeoff to the climbout location.", 'Ensure clear of obstacles and into the wind.']);
    expect(wizardLines(parseJson('{"wizardMode":false,"wizardText":["x"]}'))).toEqual([]);
    expect(wizardModePath(1)).toBe('plan.missionController.visualItems.1.wizardMode');
  });
});
