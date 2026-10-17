import { describe, expect, it } from 'vitest';
import type { CameraExtras } from './ItemCamera';
import { legLabels } from './MissionLayers';
import { MissionItem } from './PlanBridge';
import { activeActions, cameraIcon, offeredActions, startingChoice, waypointYaw } from './WaypointActions';

describe('WaypointActionsTest', () => {
  const labels = ['No change', 'Take photo', 'Take photos (time)', 'Take photos (distance)', 'Stop taking photos', 'Start recording video', 'Stop recording video'];
  const idleCamera: CameraExtras = { intervalTime: null, intervalDistance: null, distanceUnits: 'm', modeSupported: true, commandsMode: false, mode: 0, commandsGimbal: false, pitch: 0.0, yaw: 0.0, pitchRange: null, yawRange: null };
  const noHold = { seconds: 0.0, units: 's', path: 'h' };
  const noTurn = { degrees: null, units: 'deg', path: 'y' };

  it('a plain waypoint has no actions and offers every one it can carry', () => {
    expect(activeActions(noHold, noTurn, { labels, chosen: 0 }, idleCamera)).toEqual([]);
    expect(offeredActions(noHold, noTurn, { labels, chosen: 0 }, idleCamera).map((it) => it.title)).toEqual(['Hover', 'Photo', 'Video', 'Tilt camera', 'Camera mode', 'Turn aircraft']);
  });

  it('each action set on the waypoint shows once with its value and leaves the offers', () => {
    const active = activeActions(
      { seconds: 5.0, units: 's', path: 'h' },
      { degrees: 90.0, units: 'deg', path: 'y' },
      { labels, chosen: 2 },
      { ...idleCamera, intervalTime: 10.0, commandsGimbal: true, pitch: -45.0, commandsMode: true, mode: 1 },
    );
    expect(active.map((it) => `${it.title} ${it.value}`)).toEqual(['Hover 5 s', 'Take photos (time) every 10 s', 'Gimbal -45° / 0°', 'Camera mode Video', 'Turn aircraft 90°']);
    expect(offeredActions({ seconds: 5.0, units: 's', path: 'h' }, { degrees: 90.0, units: 'deg', path: 'y' }, { labels, chosen: 2 }, { ...idleCamera, commandsGimbal: true, commandsMode: true })).toEqual([]);
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
});
