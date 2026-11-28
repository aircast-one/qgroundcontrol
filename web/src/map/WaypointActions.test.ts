import { describe, expect, it } from 'vitest';
import type { CameraExtras } from './ItemCamera';
import { legLabels } from './MissionLayers';
import { MissionItem } from './PlanBridge';
import { actionSteps, activeActions, addActionsHint, cameraIcon, dropText, offeredActions, orderedActions, startingChoice, waypointDrop, waypointYaw } from './WaypointActions';

describe('WaypointActionsTest', () => {
  const labels = ['No change', 'Take photo', 'Take photos (time)', 'Take photos (distance)', 'Stop taking photos', 'Start recording video', 'Stop recording video'];
  const idleCamera: CameraExtras = { intervalTime: null, intervalDistance: null, distanceUnits: 'm', modeSupported: true, commandsMode: false, mode: 0, commandsGimbal: false, pitch: 0.0, yaw: 0.0, pitchRange: null, yawRange: null };
  const noHold = { seconds: 0.0, units: 's', path: 'h' };
  const noTurn = { degrees: null, units: 'deg', path: 'y' };

  it('a plain waypoint has no actions and offers every one it can carry', () => {
    expect(activeActions(noHold, noTurn, { labels, chosen: 0 }, idleCamera, null)).toEqual([]);
    expect(offeredActions(noHold, noTurn, { labels, chosen: 0 }, idleCamera, null).map((it) => it.title)).toEqual(['Hover', 'Photo', 'Video', 'Tilt camera', 'Camera mode', 'Turn aircraft']);
  });

  it('a plane, which flies through its waypoints, is not offered a hover or told it can hover', () => {
    const offers = offeredActions(null, null, { labels, chosen: 0 }, idleCamera, null);
    expect(offers.map((it) => it.title)).not.toContain('Hover');
    expect(addActionsHint(offers)).toBe('Tap Add action to take a photo or film here.');
    expect(addActionsHint(offeredActions(noHold, noTurn, { labels, chosen: 0 }, idleCamera, null))).toBe('Tap Add action to take a photo, film, hover or turn here.');
  });

  it('each action set on the waypoint shows once with its value and leaves the offers', () => {
    const active = activeActions(
      { seconds: 5.0, units: 's', path: 'h' },
      { degrees: 90.0, units: 'deg', path: 'y' },
      { labels, chosen: 2 },
      { ...idleCamera, intervalTime: 10.0, commandsGimbal: true, pitch: -45.0, commandsMode: true, mode: 1 },
      null,
    );
    expect(active.map((it) => `${it.title} ${it.value}`)).toEqual(['Hover 5 s', 'Take photos (time) every 10 s', 'Gimbal -45° / 0°', 'Camera mode Video', 'Turn aircraft 90°']);
    expect(offeredActions({ seconds: 5.0, units: 's', path: 'h' }, { degrees: 90.0, units: 'deg', path: 'y' }, { labels, chosen: 2 }, { ...idleCamera, commandsGimbal: true, commandsMode: true }, null)).toEqual([]);
  });

  it('photo and video start from their first action, and the editor offers the rest', () => {
    expect([startingChoice(labels, 'photo'), startingChoice(labels, 'video')]).toEqual([1, 5]);
    expect(startingChoice(['No change'], 'photo')).toBeNull();
  });

  it('video actions read as video and stopping photos reads as stop', () => {
    expect(['Take photo', 'Start recording video', 'Stop recording video', 'Stop taking photos'].map(cameraIcon)).toEqual(['plan_photo', 'plan_video', 'plan_video_off', 'plan_stop']);
  });

  it('an unset heading reads as no turn and an item without one offers none', () => {
    expect(waypointYaw({ yaw: { value: null, units: 'deg', path: 'p' } })).toEqual({ degrees: null, units: 'deg', path: 'p' });
    expect(waypointYaw({ yaw: { value: 45.0, units: 'deg', path: 'p' } })).toEqual({ degrees: 45.0, units: 'deg', path: 'p' });
    expect(waypointYaw({ yaw: null })).toBeNull();
  });

  it('each leg is labelled at its middle with the distance the core measured', () => {
    const home = MissionItem({ index: 0, sequence: 0, latitude: 41.0, longitude: 44.0, command: 'Home', selected: true, altitude: NaN });
    const first = MissionItem({ index: 1, sequence: 1, latitude: 41.0, longitude: 44.002, command: 'Waypoint', selected: true, altitude: 50.0, distance: 168.0, distanceText: '168 m' });
    const onTop = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.002, command: 'Waypoint', selected: true, altitude: 50.0, distance: 0.0, distanceText: '0 m' });
    const legs = legLabels([home, first, onTop]);
    expect(legs.map((it) => it[1])).toEqual(['168 m']);
    expect(legs.length).toBe(1);
    expect(legs[0]?.[0].longitude).toBeCloseTo(44.001, 9);
    expect(legs[0]?.[0].latitude).toBeCloseTo(41.0, 9);
  });

  it('a leg past an ROI is labelled between the waypoints it joins, not from the ROI', () => {
    const first = MissionItem({ index: 1, sequence: 1, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: true, altitude: 50.0 });
    const roi = MissionItem({ index: 2, sequence: 2, latitude: 42.0, longitude: 45.0, command: 'ROI', selected: true, altitude: NaN, routed: false });
    const next = MissionItem({ index: 3, sequence: 3, latitude: 41.0, longitude: 44.002, command: 'Waypoint', selected: true, altitude: 50.0, distance: 168.0, distanceText: '168 m' });
    const legs = legLabels([first, roi, next]);
    expect(legs.length).toBe(1);
    expect(legs[0]?.[0].longitude).toBeCloseTo(44.001, 9);
  });

  const dropFact = (fields: object) => waypointDrop({ drop: { index: 5, wait: 0, waitOffered: true, servo: true, output: 4, pwm: 2000, bay: null, ...fields } });

  it('a drop reads its servo, its PWM and its bay from its own mission item, and says how long the drone waits first', () => {
    expect(dropFact({ wait: 10 })).toEqual({ index: 5, wait: 10, waitOffered: true, servo: true, output: 4, pwm: 2000, bay: null });
    expect([dropFact({ wait: 10 }), dropFact({ output: 10, pwm: 1900, bay: 2 }), dropFact({ servo: false, output: null, pwm: null })].map((it) => (it === null ? '' : dropText(it)))).toEqual([
      'Servo 4 to 2000 · after 10 s',
      'Bay 2 · Servo 10 to 1900',
      'Gripper',
    ]);
    expect(activeActions(noHold, noTurn, null, null, dropFact({ wait: 10 })).map((it) => `${it.title} ${it.value}`)).toEqual(['Drop payload Servo 4 to 2000 · after 10 s']);
  });

  it('a waypoint offers a drop until one follows it, then shows it as an action', () => {
    const free = dropFact({ index: null, output: null, pwm: null });
    const taken = dropFact({});
    expect(waypointDrop({})).toBeNull();
    expect(free?.index).toBeNull();
    expect(dropFact({ waitOffered: false })?.waitOffered, 'a plane is not offered a wait').toBe(false);
    expect(offeredActions(noHold, noTurn, null, null, free).map((it) => it.title)).toContain('Drop payload');
    expect(offeredActions(noHold, noTurn, null, null, taken).map((it) => it.title)).not.toContain('Drop payload');
    expect(activeActions(noHold, noTurn, null, null, taken).map((it) => it.title)).toEqual(['Drop payload']);
    expect(offeredActions(noHold, noTurn, null, null, null).map((it) => it.title)).not.toContain('Drop payload');
  });

  it('the actions are listed in the order the core says they are flown, with waits and other commands in their places', () => {
    const steps = actionSteps({
      actions: [
        { kind: 'Gimbal', title: 'Gimbal', seconds: null, item: null, earlier: false, later: true },
        { kind: 'Camera', title: 'Camera', seconds: null, item: null, earlier: true, later: true },
        { kind: 'Wait', title: 'Wait', seconds: 10, item: 4, earlier: true, later: true },
        { kind: 'Command', title: 'Set servo', seconds: null, item: 5, earlier: true, later: true },
        { kind: 'Drop', title: 'Drop', seconds: null, item: 6, earlier: true, later: false },
      ],
    });
    const active = activeActions(noHold, { degrees: 90.0, units: 'deg', path: 'y' }, { labels, chosen: 1 }, { ...idleCamera, commandsGimbal: true, yaw: 60.0 }, dropFact({ index: 6, wait: 10 }));
    const ordered = orderedActions(steps, active);
    expect(ordered.map((it) => `${it.title} ${it.value}`.trim())).toEqual(['Gimbal 0° / 60°', 'Take photo', 'Wait 10 s', 'Set servo', 'Drop payload Servo 4 to 2000 · after 10 s', 'Turn aircraft 90°']);
    expect(ordered.map((it) => [it.place, it.earlier, it.later]), 'only what the core can move has a place, and the ends cannot leave the list').toEqual([
      [0, false, true],
      [1, true, true],
      [2, true, true],
      [3, true, true],
      [4, true, false],
      [null, false, false],
    ]);
    expect(ordered.map((it) => it.action?.kind.name ?? null), 'a wait and a plain command have no editor of their own').toEqual(['Gimbal', 'Camera', null, null, 'Drop', 'Turn']);
    expect(ordered[2]?.item).toBe(4);
    expect(orderedActions([], active).map((it) => it.title), 'with no order from the core the actions still show').toEqual(active.map((it) => it.title));
    expect(actionSteps(null)).toEqual([]);
  });
});
