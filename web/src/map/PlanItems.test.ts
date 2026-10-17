import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { RALLY_ALTITUDE_DECIMALS, altitudeFieldText } from './AltitudeEntry';
import { rallyPoints } from './FenceBridge';
import type { MapHit } from './MissionEditing';
import { AT_END, HOME_ITEM } from './MissionKinds';
import { RETURN_COLOUR, START_COLOUR, TAKEOFF_COLOUR, WAYPOINT_COLOUR } from './MissionLayers';
import { KIND_STRUCTURE, KIND_TAKEOFF, MAV_CMD_NAV_RETURN_TO_LAUNCH, MissionItem, allMissionItems } from './PlanBridge';
import {
  NOT_READY_SEAL,
  NO_POSITION,
  addingAfterText,
  altitudeWithFrame,
  cameraText,
  holdText,
  insertAfter,
  itemCountText,
  itemDetail,
  itemRows,
  legText,
  missionItemIndex,
  movedText,
  photosText,
  rallyAltitudeIsEditable,
  rallyAltitudeLabel,
  rowAt,
  selectionAfterRemove,
  selectionSequence,
  sequenceLabel,
  sheetDetail,
  surveyTiles,
  worthListing,
} from './PlanItems';
import { selectionText } from './PlanSummary';
import { selectionSurvives } from './SelectionValidity';
import { SurveyStats, surveyStats } from './SurveyBridge';

const waypoint = (index: number): MapHit => ({ kind: 'Waypoint', index });

describe('PlanItemsTest', () => {
  const item = (index: number, sequence: number, name: string, altitude = 50.0, kind = 'waypoint', commandId = 16) =>
    MissionItem({ index, sequence, latitude: 41.0, longitude: 44.0, command: name, selected: false, altitude, kind, commandId, altitudeText: Number.isNaN(altitude) ? '' : `${Math.trunc(altitude)} m` });

  it("a survey's area and photos move from the sheet line into tiles", () => {
    const survey = { ...item(4, 4, 'Survey'), cameraShots: 1043 };
    const stats = SurveyStats({ areaText: '90049 m²', warning: '', intervalText: '2.0 s', distanceText: '4123 m' });

    expect(surveyTiles(survey, stats)).toEqual([
      ['AREA', '90049 m²'],
      ['DISTANCE', '4123 m'],
      ['PHOTOS', '1043'],
      ['INTERVAL', '2.0 s'],
    ]);
    expect(sheetDetail(survey, stats)).toBe('50 m');
    expect(surveyTiles(survey, null)).toEqual([]);
    expect(
      surveyTiles({ ...item(5, 5, 'Structure Scan', 50.0, KIND_STRUCTURE), cameraShots: 40 }, { ...stats, areaText: '' }).map((it) => it[0]),
      "StructureScanEditor's stats have no Distance row",
    ).toEqual(['PHOTOS', 'INTERVAL']);
    expect(sheetDetail(survey, null)).toBe('50 m · 1043 photos');
  });

  it('a row carries the number, the name, the altitude and the marker colour', () => {
    const rows = itemRows([item(1, 1, 'Waypoint', 49.6)]);

    expect(rows.map((it) => it.number)).toEqual(['1']);
    expect(rows.map((it) => it.name)).toEqual(['Waypoint']);
    expect(rows.map((it) => it.detail)).toEqual(['49 m']);
    expect(rows.map((it) => it.colour)).toEqual([WAYPOINT_COLOUR]);
  });

  it('the list is coloured the same way the map is, so a row and its marker match', () => {
    const rows = itemRows([item(0, 0, 'Mission Start', 50.0, 'settings'), item(1, 1, 'Takeoff', 50.0, 'takeoff', 22), item(2, 2, 'Return To Launch', 50.0, 'command', MAV_CMD_NAV_RETURN_TO_LAUNCH)]);

    expect(rows.map((it) => it.colour)).toEqual([START_COLOUR, TAKEOFF_COLOUR, RETURN_COLOUR]);
  });

  it('a pattern spanning heights shows the band, having no single altitude', () => {
    const survey = MissionItem({ index: 4, sequence: 4, latitude: 41.0, longitude: 44.0, command: 'Survey', selected: false, altitude: NaN, kind: 'survey', altitudeBandText: '585 m to 660 m' });

    expect(itemDetail(survey)).toBe('585 m to 660 m');
  });

  it('an item with its own altitude ignores any band', () => {
    const plain = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m', altitudeBandText: 'nonsense' });

    expect(itemDetail(plain)).toBe('50.0 m');
  });

  it('an item with no altitude says nothing rather than NaN', () => {
    expect(itemRows([item(1, 1, 'Waypoint', NaN)])[0]!.detail).toBe('');
  });

  it('an item the vehicle did not name is still reachable by its number', () => {
    expect(itemRows([item(3, 3, '')])[0]!.name).toBe('Item 3');
  });

  it('a command with no place by design is listed and says nothing about position', () => {
    const rows = itemRows([MissionItem({ index: 1, sequence: 1, latitude: NaN, longitude: NaN, command: 'Change Speed', selected: false, altitude: NaN, kind: 'command', commandId: 178, placed: false, specifiesCoordinate: false })]);

    expect(rows[0]!.detail).toBe('');
    expect(rows[0]!.placed).toBe(false);
  });

  it('an item that is supposed to have a place and has none still says so', () => {
    const rows = itemRows([MissionItem({ index: 1, sequence: 1, latitude: NaN, longitude: NaN, command: 'Waypoint', selected: false, altitude: NaN, kind: 'waypoint', commandId: 16, placed: false, specifiesCoordinate: true })]);

    expect(rows[0]!.detail).toBe(NO_POSITION);
  });

  it('an ArduPilot takeoff specifies an altitude and no place, so the altitude is what it says', () => {
    const takeoff = MissionItem({ index: 1, sequence: 1, latitude: NaN, longitude: NaN, command: 'Takeoff', selected: false, altitude: 50.0, kind: KIND_TAKEOFF, commandId: 22, placed: false, altitudeText: '50 m' });

    expect(itemRows([takeoff])[0]!.detail).toBe('50 m');
  });

  it('the altitude is whatever the core spelled, because the operator may not be in metres', () => {
    const feet = MissionItem({ index: 1, sequence: 1, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 75.0, kind: 'waypoint', commandId: 16, altitudeText: '246 ft' });

    expect(itemRows([feet])[0]!.detail).toBe('246 ft');
  });

  it('an item sitting after the land says so, because the map just draws no line to it', () => {
    const stranded = MissionItem({ index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, routed: false, kind: 'waypoint', commandId: 16, afterRouteEnds: true, altitudeText: '50 m' });

    expect(itemRows([stranded])[0]!.detail).toBe('50 m · after the route ends');
  });

  it('an item the route does not pass through is not accused of being stranded', () => {
    const roi = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Region Of Interest', selected: false, altitude: NaN, routed: false, kind: 'roi', commandId: 201 });

    expect(itemRows([roi])[0]!.detail).toBe('');
  });

  it('a row is found by the index the map selects with, not by position', () => {
    const rows = itemRows([item(4, 1, 'Waypoint'), item(9, 2, 'Land', 50.0, 'land', 21)]);

    expect(rowAt(rows, 9)?.name).toBe('Land');
    expect(rowAt(rows, 1)).toBeNull();
  });
});

describe('UnplacedSelectionTest', () => {
  const unplaced = MissionItem({ index: 1, sequence: 1, latitude: NaN, longitude: NaN, command: 'Takeoff', selected: false, altitude: NaN, kind: 'takeoff', commandId: 22, placed: false });

  it('selecting an item the map cannot draw is not thrown away on the next read', () => {
    expect(selectionSurvives(waypoint(1), [unplaced], [], [], [], [])).toBe(true);
  });

  it('a selection of an item that left the plan is still thrown away', () => {
    expect(selectionSurvives(waypoint(1), [], [], [], [], [])).toBe(false);
  });
});

describe('WorthListingTest', () => {
  const item = (index: number) => MissionItem({ index, sequence: index, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint' });

  it('a plan holding only its settings item has nothing to list', () => {
    expect(worthListing([item(HOME_ITEM)])).toBe(false);
  });

  it('one real item is enough to be worth listing', () => {
    expect(worthListing([item(HOME_ITEM), item(1)])).toBe(true);
  });

  it('no plan at all is nothing to list', () => {
    expect(worthListing([])).toBe(false);
  });
});

describe('InsertAfterTest', () => {
  const item = (index: number) => MissionItem({ index, sequence: index, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint' });

  const plan = [item(0), item(1), item(2)];

  it('with nothing selected an item goes on the end', () => {
    expect(insertAfter(null, plan)).toBe(AT_END);
  });

  it('a new item follows the one the operator selected, as it does in QGC', () => {
    expect(insertAfter(waypoint(1), plan)).toBe(2);
  });

  it('selecting the last item still means the end, not a place past it', () => {
    expect(insertAfter(waypoint(2), plan)).toBe(AT_END);
  });

  it('a selection the plan no longer holds does not name a position', () => {
    expect(insertAfter(waypoint(9), plan)).toBe(AT_END);
  });

  it('a selected pattern corner inserts after its pattern, and a non-mission selection appends', () => {
    expect(insertAfter({ kind: 'SurveyVertex', item: 1, vertex: 0 }, plan)).toBe(2);
    expect(insertAfter({ kind: 'Rally', index: 1 }, plan)).toBe(AT_END);
  });
});

describe('SelectionSequenceTest', () => {
  const plan = [
    MissionItem({ index: 0, sequence: 0, latitude: 41.0, longitude: 44.0, command: 'Mission Start', selected: false, altitude: 0.0, kind: 'settings' }),
    MissionItem({ index: 1, sequence: 1, latitude: 41.0, longitude: 44.0, command: 'Takeoff', selected: false, altitude: 50.0, kind: KIND_TAKEOFF }),
    MissionItem({ index: 2, sequence: 4, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint' }),
  ];

  it('the sequence is sent, not the index, because they are not the same number', () => {
    expect(selectionSequence(waypoint(2), plan)).toBe(4);
  });

  it('nothing selected selects nothing', () => {
    expect(selectionSequence(null, plan)).toBeNull();
  });

  it('a selection the plan no longer holds sends nothing', () => {
    expect(selectionSequence(waypoint(9), plan)).toBeNull();
  });

  it('a fence vertex is not a mission item and does not move the plan view', () => {
    expect(selectionSequence({ kind: 'FenceVertex', polygon: 0, vertex: 1 }, plan)).toBeNull();
  });
});

describe('AddingAfterTextTest', () => {
  const plan = [
    MissionItem({ index: 0, sequence: 0, latitude: 41.0, longitude: 44.0, command: 'Mission Start', selected: false, altitude: 0.0, kind: 'settings' }),
    MissionItem({ index: 2, sequence: 5, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint' }),
  ];

  it('the line names the sequence the operator sees on the marker', () => {
    expect(addingAfterText(waypoint(2), plan)).toBe('Tap the map to add after #5');
  });

  it('with nothing selected the insertion point is the end and needs no line', () => {
    expect(addingAfterText(null, plan)).toBeNull();
  });

  it('a fence vertex does not move the insertion point, so it says nothing', () => {
    expect(addingAfterText({ kind: 'FenceVertex', polygon: 0, vertex: 1 }, plan)).toBeNull();
  });

  it('a selection the plan no longer holds says nothing rather than a stale number', () => {
    expect(addingAfterText(waypoint(9), plan)).toBeNull();
  });
});

describe('LegTextTest', () => {
  const item = ({ distanceText = '449.4 m', azimuthText = '47', altitudeChangeText = '12.0 m', headingText = '', gradientText = '2 deg' } = {}) =>
    MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', commandId: 16, distanceText, azimuthText, altitudeChangeText, headingText, gradientText });

  it('each figure carries the label and order PlanToolBarIndicators gives it', () => {
    expect(legText(item({ headingText: '90' }))).toBe('Alt diff 12.0 m · Azimuth 47 · Heading 90 · Gradient 2 deg · Prev WP 449.4 m');
  });

  it('zeros show like QGC, which hides a stat only when it has no number', () => {
    expect(legText(item({ distanceText: '0.0 m', azimuthText: '0', altitudeChangeText: '0.0 m', gradientText: '' }))).toBe('Alt diff 0.0 m · Azimuth 0 · Prev WP 0.0 m');
  });

  it('an item the controller has not measured says nothing', () => {
    expect(legText(item({ distanceText: '', azimuthText: '', altitudeChangeText: '', gradientText: '' }))).toBeNull();
  });
});

describe('BlockedReasonTest', () => {
  it('an item that blocks the save says why, where the operator is looking', () => {
    const item = MissionItem({ index: 4, sequence: 4, latitude: 41.0, longitude: 44.0, command: 'Landing pattern', selected: false, altitude: NaN, kind: 'land', blockedReason: 'Landing point not set' });

    expect(itemDetail(item)).toBe('Landing point not set');
  });

  it('a reason joins whatever else the row already says', () => {
    const item = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m', blockedReason: 'Needs a value' });

    expect(itemDetail(item)).toBe('50.0 m · Needs a value');
  });

  it('an item with nothing wrong says nothing extra', () => {
    const item = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m' });

    expect(itemDetail(item)).toBe('50.0 m');
  });
});

describe('PhotosTextTest', () => {
  it('a pattern that takes photos says how many', () => {
    const survey = MissionItem({ index: 5, sequence: 5, latitude: 41.0, longitude: 44.0, command: 'Survey', selected: false, altitude: NaN, kind: 'survey', altitudeBandText: '0.0 m to 40.0 m', cameraShots: 340 });

    expect(itemDetail(survey)).toBe('0.0 m to 40.0 m · 340 photos');
  });

  it('one photo is not one photos', () => {
    expect(photosText(1)).toBe('1 photo');
    expect(photosText(2)).toBe('2 photos');
  });

  it('an item that takes none says nothing about photos', () => {
    expect(photosText(0)).toBeNull();
    expect(photosText(-1)).toBeNull();

    const plain = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m' });
    expect(itemDetail(plain)).toBe('50.0 m');
  });
});

describe('SpeedChangeRowTest', () => {
  it("an item that commands a speed says which, in the operator's unit", () => {
    const change = MissionItem({ index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: 'Change speed', selected: false, altitude: NaN, kind: 'command', speedChangeText: '12.0 m/s' });

    expect(itemDetail(change)).toBe('12.0 m/s');
  });

  it('a commanded speed reads alongside the altitude rather than instead of it', () => {
    const plain = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m', speedChangeText: '8.0 m/s' });

    expect(itemDetail(plain)).toBe('50.0 m · 8.0 m/s');
  });

  it('an item that both holds and changes speed says both, because neither ranks', () => {
    const both = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m', speedChangeText: '8.0 m/s', extraSeconds: 15.0 });

    expect(itemDetail(both)).toBe('50.0 m · 8.0 m/s · holds 15 s');
  });

  it('an item the core withheld a speed for says nothing about speed', () => {
    const plain = MissionItem({ index: 3, sequence: 3, latitude: 41.0, longitude: 44.0, command: 'Change speed', selected: false, altitude: NaN, kind: 'command' });

    expect(itemDetail(plain)).toBe('');
  });
});

describe('HoldTextTest', () => {
  it('an item that waits says how long', () => {
    const plain = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', altitudeText: '50.0 m', extraSeconds: 30.0 });

    expect(itemDetail(plain)).toBe('50.0 m · holds 30 s');
  });

  it('an item that does not wait says nothing about waiting', () => {
    expect(holdText(0.0)).toBeNull();
    expect(holdText(NaN)).toBeNull();
    expect(holdText(-1.0)).toBeNull();
  });
});

describe('SurveyStatsRowTest', () => {
  const survey = MissionItem({ index: 5, sequence: 5, latitude: 41.0, longitude: 44.0, command: 'Survey', selected: false, altitude: NaN, kind: 'survey', altitudeBandText: '0.0 m to 40.0 m', cameraShots: 171 });

  it('a survey carries its area beside its photo count', () => {
    expect(itemDetail(survey, SurveyStats({ areaText: '0.03 km²', warning: '' }))).toBe('0.0 m to 40.0 m · 0.03 km² · 171 photos');
  });

  it('a camera that cannot keep up says so, last, where it is read', () => {
    const stats = SurveyStats({ areaText: '0.03 km²', warning: 'The camera needs 2.00 s between shots but the survey asks for 1.50 s.' });

    expect(itemDetail(survey, stats).endsWith('The camera needs 2.00 s between shots but the survey asks for 1.50 s.')).toBe(true);
  });

  it('an item with no survey stats is unchanged', () => {
    expect(itemDetail(survey, null)).toBe('0.0 m to 40.0 m · 171 photos');
  });

  it('nothing to say is null rather than a line saying nothing', () => {
    expect(photosText(0), 'a waypoint taking no photographs must not draw a photo line').toBeNull();
    expect(photosText(-1)).toBeNull();
    expect(holdText(0.0), 'an item that does not hold must not draw a hold line').toBeNull();
    expect(holdText(NaN)).toBeNull();
    expect(holdText(-3.0)).toBeNull();
  });

  it('dragging the breach return point says so and survives only while the point exists', () => {
    expect(movedText({ kind: 'BreachReturn' }, [])).toBe('Moved the breach return point');
    expect(selectionSurvives({ kind: 'BreachReturn' }, [], [], [], [], [], [], true)).toBe(true);
    expect(selectionSurvives({ kind: 'BreachReturn' }, [], [], [], [], [])).toBe(false);
  });

  it('deleting keeps the selection at the same index or the new last item, as removeVisualItem does', () => {
    expect(selectionAfterRemove(2, 5)).toEqual(waypoint(2));
    expect(selectionAfterRemove(4, 5)).toEqual(waypoint(3));
    expect(selectionAfterRemove(1, 2)).toBeNull();
  });
});

describe('InsertAfterSelectionTest', () => {
  it('any selected mission item is current, as PlanView inserts after currentPlanViewVIIndex', () => {
    expect(missionItemIndex({ kind: 'SurveyVertex', item: 4, vertex: 2 })).toBe(4);
    expect(missionItemIndex({ kind: 'ShapeCentre', fence: false, owner: 5 })).toBe(5);
    expect(missionItemIndex({ kind: 'ShapeCentre', fence: true, owner: 5 }), 'a fence shape is not a mission item').toBeNull();
    expect(missionItemIndex({ kind: 'LandingPlace', index: 6, place: 0 })).toBe(6);
    expect(missionItemIndex({ kind: 'Rally', index: 1 })).toBeNull();
  });

  it('an item not ready to send shows a question mark seal like MissionItemEditor', () => {
    const item = MissionItem({ index: 2, sequence: 2, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', commandId: 16 });
    expect(itemRows([item])[0]!.number).toBe('2');
    expect(itemRows([{ ...item, readyForSave: false }])[0]!.number).toBe(NOT_READY_SEAL);
  });
});

describe('ItemCountTextTest', () => {
  it('singularForOne', () => {
    expect(itemCountText(1)).toBe('1 item');
  });

  it('pluralOtherwise', () => {
    expect(itemCountText(0)).toBe('0 items');
    expect(itemCountText(6)).toBe('6 items');
  });
});

describe('MovedNoticeTest', () => {
  const items = [MissionItem({ index: 4, sequence: 72, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint' })];

  it('a moved waypoint is named by the number on its marker', () => {
    expect(movedText(waypoint(4), items)).toBe('Moved #72');
  });

  it('an item the list no longer holds still says something happened', () => {
    expect(movedText(waypoint(99), items)).toBe('Moved an item');
  });

  it('every other kind of handle names what it moved', () => {
    expect(movedText({ kind: 'FenceVertex', polygon: 0, vertex: 2 }, items)).toBe('Moved a fence corner');
    expect(movedText({ kind: 'SurveyVertex', item: 1, vertex: 0 }, items)).toBe('Moved a survey corner');
    expect(movedText({ kind: 'Rally', index: 0 }, items)).toBe('Moved a rally point');
    expect(movedText({ kind: 'CircleCentre', index: 0 }, items)).toBe('Moved a fence circle');
    expect(movedText({ kind: 'Circle', index: 0 }, items)).toBe('Changed a fence radius');
  });
});

describe('SequenceLabelTest', () => {
  const item = (sequence: number, folded: number) => MissionItem({ index: sequence, sequence, latitude: 41.0, longitude: 44.0, command: 'Waypoint', selected: false, altitude: 50.0, kind: 'waypoint', foldedCommands: folded });

  it('an item that folds nothing is numbered plainly', () => {
    expect(sequenceLabel(item(2, 0))).toBe('2');
  });

  it('an item that folds a command owns the span, so the next number is not a hole', () => {
    expect(sequenceLabel(item(2, 1))).toBe('2–3');
  });

  it('a survey owns every sequence it generates', () => {
    expect(sequenceLabel(item(4, 212))).toBe('4–216');
  });

  it('the rows of a plan with a folded speed leave no gap between them', () => {
    expect(itemRows([item(0, 0), item(1, 0), item(2, 1), item(4, 0)]).map((it) => it.number)).toEqual(['0', '1', '2–3', '4']);
  });

  it('an item the core withheld the span for is numbered plainly rather than given a range', () => {
    const withheld = '{"kind":"object","items":[{"index":0,"sequence":2,"name":"Waypoint","kind":"waypoint","foldedCommands":null}]}';
    const absent = '{"kind":"object","items":[{"index":0,"sequence":2,"name":"Waypoint","kind":"waypoint"}]}';
    const folded = '{"kind":"object","items":[{"index":0,"sequence":2,"name":"Waypoint","kind":"waypoint","foldedCommands":1}]}';

    expect(sequenceLabel(allMissionItems(parseJson(withheld))[0]!)).toBe('2');
    expect(sequenceLabel(allMissionItems(parseJson(absent))[0]!)).toBe('2');
    expect(sequenceLabel(allMissionItems(parseJson(folded))[0]!)).toBe('2–3');
  });
});

describe('AltitudeFrameTest', () => {
  const item = (frameText: string, text = '50.0 m', band = '', editUnits = '') =>
    MissionItem({
      index: 2,
      sequence: 2,
      latitude: 41.0,
      longitude: 44.0,
      command: 'Waypoint',
      selected: false,
      altitude: 50.0,
      kind: 'waypoint',
      altitudeText: text,
      altitudeBandText: band,
      altitudeFrameText: frameText,
      altitudeEditUnits: editUnits,
    });

  it('a height above launch is the one an operator already assumes, so it is left bare', () => {
    expect(altitudeWithFrame(item(''))).toBe('50.0 m');
  });

  it('the two frames that are not the default say so', () => {
    expect(altitudeWithFrame(item('AMSL', '541 m'))).toBe('541 m AMSL');
    expect(altitudeWithFrame(item('AGL'))).toBe('50.0 m AGL');
  });

  it('a survey and a takeoff both reading fifty metres are no longer the same row', () => {
    const takeoff = item('');
    const survey = item('AGL');

    expect(itemDetail(takeoff)).toBe('50.0 m');
    expect(itemDetail(survey)).toBe('50.0 m AGL');
  });

  it('a band takes the frame too, rather than only single heights', () => {
    expect(altitudeWithFrame(item('AMSL', '', '0.0 m to 40.0 m'))).toBe('0.0 m to 40.0 m AMSL');
  });

  it('a frame the core withheld leaves the height unlabelled rather than guessing one', () => {
    expect(altitudeWithFrame(item(''))).toBe('50.0 m');
    expect(altitudeWithFrame(item('AMSL', ''))).toBeNull();
  });

  it('the summary chip carries the frame too, being the other place a height is spelled', () => {
    const amsl = item('AMSL', '541 m');

    expect(selectionText({ kind: 'Waypoint', index: 2 }, [amsl], [], [])).toBe('#2 at 541 m AMSL');
    expect(selectionText({ kind: 'Waypoint', index: 2 }, [item('')], [], [])).toBe('#2 at 50.0 m');
  });

  it('a frame neither head has heard of is shown, not swallowed into the default', () => {
    expect(altitudeWithFrame(item('SEABED'))).toBe('50.0 m SEABED');
  });
});

describe('CameraTextTest', () => {
  const stats = (json: string) => surveyStats(parseJson(json));

  it('a survey says how high above the surface, what a shot covers and how often it fires', () => {
    const view = stats('{"available":true,"areaText":"9.0 ha","warning":"","surfaceDistanceText":"60.0 m","footprintText":"12.5 × 8.0 m","intervalText":"2.4 s"}');

    expect(cameraText(view)).toBe('60.0 m above the surface · each shot covers 12.5 × 8.0 m · a shot every 2.4 s');
  });

  it("the core's em dash is an absence, not a figure to print", () => {
    const view = stats('{"available":true,"areaText":"—","warning":"","surfaceDistanceText":"—","footprintText":"—","intervalText":"—"}');

    expect(cameraText(view)).toBeNull();
    expect(view!.areaText).toBe('');
  });

  it('a survey the core answered partly says the part it knows', () => {
    const view = stats('{"available":true,"areaText":"9.0 ha","warning":"","footprintText":"12.5 × 8.0 m"}');

    expect(cameraText(view)).toBe('each shot covers 12.5 × 8.0 m');
  });

  it('no survey and no stats say nothing rather than an empty row', () => {
    expect(cameraText(null)).toBeNull();
    expect(cameraText(stats('{"available":false}'))).toBeNull();
  });

  it('an unavailable survey has no stats to show', () => {
    expect(surveyStats(null)).toBeNull();
    expect(
      surveyStats(parseJson('{"available":false,"areaText":"12 ha"}')),
      'available false means the core could not compute them; a stats block built anyway would show blanks as though they were measurements',
    ).toBeNull();
  });
});

describe('RallyAltitudeEditTest', () => {
  const imperial = parseJson(
    '{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":164.0,"altitudeUnits":"ft","altitudeText":"164 ft","altitudeMetres":50.0,"altitudePath":"plan.rallyPointController.points.0.textFieldFacts.2"}]}',
  );

  it("the edited number is the operator's unit, never the metres used for a drag", () => {
    const point = rallyPoints(imperial)[0]!;

    expect(point.altitude).toBeCloseTo(164.0, 9);
    expect(point.altitudeMetres, 'a drag writes a whole coordinate the bridge reads as metres, so the two numbers describe the same height and only one of them belongs in the field').toBeCloseTo(50.0, 9);
    expect(Math.abs(point.altitude - point.altitudeMetres)).toBeGreaterThan(1e-9);
    expect(altitudeFieldText(point.altitude, RALLY_ALTITUDE_DECIMALS)).toBe('164');
  });

  it('the label carries the served unit, not a hardcoded metre', () => {
    expect(rallyAltitudeLabel(rallyPoints(imperial)[0]!)).toBe('Alt ft');
  });

  it('the field writes to the fact path, which converts, not to the coordinate', () => {
    const point = rallyPoints(imperial)[0]!;

    expect(point.altitudePath).toBe('plan.rallyPointController.points.0.textFieldFacts.2');
    expect(
      point.altitudePath.endsWith('textFieldFacts.2'),
      'the fact takes the cooked number and QGC converts it; writing 164 into the coordinate instead would set the point to 164 metres',
    ).toBe(true);
  });

  it('a point whose altitude is not a fact offers no field', () => {
    const notAFact = parseJson('{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":null,"altitudeUnits":"","altitudeMetres":50.0,"altitudePath":""}]}');

    expect(rallyAltitudeIsEditable(rallyPoints(notAFact)[0]!)).toBe(false);
  });

  it('a core too old to serve the path offers no field rather than writing nowhere', () => {
    const old = parseJson('{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitudeMetres":50.0}]}');

    expect(rallyAltitudeIsEditable(rallyPoints(old)[0]!)).toBe(false);
  });

  it('an altitude with nowhere to write it offers no field', () => {
    const noPath = parseJson('{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":164.0,"altitudeUnits":"ft","altitudeMetres":50.0}]}');

    expect(rallyAltitudeIsEditable(rallyPoints(noPath)[0]!), 'the path is the write target; a field that renders without one edits nothing and reports success').toBe(false);
  });

  it('a served point is editable', () => {
    expect(rallyAltitudeIsEditable(rallyPoints(imperial)[0]!)).toBe(true);
  });

  it('a metric point labels itself in metres without a fallback', () => {
    const metric = parseJson(
      '{"kind":"object","class":"Fences","rallyPoints":[{"index":0,"latitude":47.397,"longitude":8.545,"altitude":50.0,"altitudeUnits":"m","altitudeMetres":50.0,"altitudePath":"plan.rallyPointController.points.0.textFieldFacts.2"}]}',
    );
    const point = rallyPoints(metric)[0]!;

    expect(rallyAltitudeLabel(point)).toBe('Alt m');
    expect(point.altitude).toBeCloseTo(point.altitudeMetres, 9);
  });
});
