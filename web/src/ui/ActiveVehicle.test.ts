import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  MvAction,
  activeVehicleId,
  fleetActionLine,
  fleetHeading,
  fleetIsDestructive,
  fleetPanelShown,
  loadingProgress,
  mvActions,
  mvReasonFor,
  panelToggleShown,
  rowCompassAlpha,
} from './ActiveVehicle';

const json = (text: string): Json => parseJson(text) ?? {};

const single = <T,>(items: readonly T[]): T => {
  if (items.length !== 1) throw new Error(`expected exactly one, found ${items.length}`);
  return items[0] as T;
};

describe('LoadProgressTest', () => {
  it('the chip shows load progress until the initial connect completes', () => {
    expect(loadingProgress(false, 0.4)).toBe(0.4);
    expect(loadingProgress(false, null)).toBe(0);
    expect(loadingProgress(false, 3)).toBe(1);
    expect(loadingProgress(true, 1.0)).toBeNull();
    expect(loadingProgress(null, null)).toBeNull();
  });
});

describe('FleetActionsTest', () => {
  const served = (...entries: readonly string[]) => json(`{"kind":"object","class":"Vehicles","actions":[${entries.join(',')}]}`);

  const action = (id: string, offer: string, reason = '') => `{"id":"${id}","title":"${id}","prompt":"do ${id}","offer":"${offer}","reason":"${reason}"}`;

  it('an absent actions list reads as no fleet actions', () => {
    expect(mvActions(json('{"kind":"object"}'))).toEqual([]);
    expect(mvActions(null)).toEqual([]);
  });

  it('every served action is offered in the order the core listed them', () => {
    const actions = mvActions(served(action('mvArm', 'ready'), action('mvDisarm', 'blocked', 'No selected vehicle is armed.')));
    expect(actions.map((it) => it.id)).toEqual(['mvArm', 'mvDisarm']);
    expect(actions[0]?.ready).toBe(true);
    expect(actions.at(-1)?.ready).toBe(false);
  });

  it('an entry without an id is not offered', () => {
    expect(mvActions(served('{"title":"Arm","offer":"ready","prompt":"","reason":""}'))).toEqual([]);
  });

  it('a reason is shown only while the action is blocked', () => {
    const blocked = single(mvActions(served(action('mvPause', 'blocked', 'No selected vehicle supports being paused.'))));
    expect(mvReasonFor(blocked)).toBe('No selected vehicle supports being paused.');
    const ready = single(mvActions(served(action('mvPause', 'ready', 'No selected vehicle supports being paused.'))));
    expect(mvReasonFor(ready)).toBeNull();
  });

  it('an empty reason on a blocked action says nothing rather than an empty line', () => {
    expect(mvReasonFor(single(mvActions(served(action('mvArm', 'blocked')))))).toBeNull();
  });

  it('the heading lists the selected vehicle ids like FlyViewTopRightPanel', () => {
    expect(fleetHeading([])).toBe('Vehicles Selected: -');
    expect(fleetHeading([1, 3])).toBe('Vehicles Selected: 1, 3');
  });

  it('the panel shows for two or more vehicles with the setting on', () => {
    expect(fleetPanelShown(1, true)).toBe(false);
    expect(fleetPanelShown(2, false)).toBe(false);
    expect(fleetPanelShown(2, true)).toBe(true);
  });

  it('a ready action says what it does', () => {
    const ready = single(mvActions(served(action('mvArm', 'ready'))));
    expect(fleetActionLine(ready)).toBe('do mvArm');
  });

  it('a blocked action shows the refusal in place of the prompt', () => {
    const blocked = single(mvActions(served(action('mvDisarm', 'blocked', 'No selected vehicle is armed.'))));
    expect(fleetActionLine(blocked)).toBe('No selected vehicle is armed.');
  });

  it('an action the core sent no prompt for falls back to its title', () => {
    expect(fleetActionLine(MvAction({ id: 'mvArm', title: 'Arm', confirmTitle: 'Arm (MV)', prompt: '', offer: 'ready', reason: '' }))).toBe('Arm');
  });

  it('pause is the one fleet action that is not destructive', () => {
    const destructive = ['mvArm', 'mvDisarm', 'mvStartMission', 'mvPause']
      .map((it) => MvAction({ id: it, title: it, confirmTitle: it, prompt: '', offer: 'ready', reason: '' }))
      .filter(fleetIsDestructive)
      .map((it) => it.id);
    expect(destructive).toEqual(['mvArm', 'mvDisarm', 'mvStartMission']);
  });

  it("the active vehicle's id comes off the view that already names it", () => {
    expect(activeVehicleId(json('{"kind":"object","activeId":7}'))).toBe(7);
  });

  it('no vehicle is no id, and never zero', () => {
    expect(activeVehicleId(null)).toBeNull();
    expect(activeVehicleId(json('{"kind":"object","activeId":null}'))).toBeNull();
    expect(activeVehicleId(json('{"kind":"object"}'))).toBeNull();
    expect(activeVehicleId(json('{"kind":"object","activeId":0}'))).toBeNull();
  });
});

describe('MultiVehiclePanelToggleTest', () => {
  it("the toggle follows the fact's visible flag, as MultiVehicleSelector does", () => {
    expect(panelToggleShown(json('{"visible":false}'))).toBe(false);
    expect(panelToggleShown(json('{"visible":true}'))).toBe(true);
    expect(panelToggleShown(null)).toBe(true);
  });
});

describe('VehicleRowCompassTest', () => {
  it("a disarmed vehicle's compass is dimmed like MultiVehicleList's", () => {
    expect(rowCompassAlpha(true)).toBe(1);
    expect(rowCompassAlpha(false)).toBe(0.5);
  });
});
