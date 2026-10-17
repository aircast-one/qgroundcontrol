import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  categoryNames,
  commandChoices,
  geoOf,
  gridNote,
  itemCommandPath,
  itemFactsPath,
  itemFields,
  landingNotes,
  positionForms,
  radioChoices,
  sectionStarts,
  speedSection,
  utmToGeoPath,
  vehicleAltitudeLabel,
  vehiclePositionOf,
  withLandingFrameUnits,
} from './ItemEditor';

const json = (text: string): Json => parseJson(text)!;

describe('ItemEditorTest', () => {
  it('structure scan note comes from the core and blank means none like StructureScanEditor', () => {
    expect(gridNote(json(`{"gridNote":"The polygon outlines the structure's surface, not the flight path."}`))).toBe("The polygon outlines the structure's surface, not the flight path.");
    expect(gridNote(json('{"gridNote":null}'))).toBeNull();
    expect(gridNote(null)).toBeNull();
  });

  it("vehicle position mode shows the vehicle's coordinate and frame altitude like EditPositionDialog", () => {
    const shown = vehiclePositionOf(json('{"valid":true,"latitude":47.3977419,"longitude":8.5455938}'), json('{"kind":"fact","value":12.3,"valueString":"12.3","units":"m"}'));
    expect(shown).toEqual({ latitude: '47.3977419', longitude: '8.5455938', altitude: '12.3 m' });
    expect(vehiclePositionOf(json('{"valid":true,"latitude":1.0,"longitude":2.0}'), null)?.altitude).toBe('');
    expect(vehiclePositionOf(json('{"valid":false}'), null)).toBeNull();
    expect([1, 2, 3, 4, 0].map(vehicleAltitudeLabel)).toEqual(['Alt (Rel)', 'Alt (AMSL)', 'Alt (AGL)', 'Alt (AGL)', null]);
  });

  it('commands and categories read from the command tree', () => {
    const commands = commandChoices(JSON.parse('[{"command":16,"friendlyName":"Waypoint","description":"Travel to a position in 3D space."},{"command":19,"friendlyName":"Loiter (time)","description":"d"}]'));
    expect(commands.map((it) => it.id)).toEqual([16, 19]);
    expect(commands[0]?.name).toBe('Waypoint');
    expect(categoryNames(JSON.parse('["Basic","Advanced"]'))).toEqual(['Basic', 'Advanced']);
    expect(commandChoices(null)).toHaveLength(0);
  });

  it("an item's command is written on its own path", () => {
    expect(itemCommandPath(3)).toBe('plan.missionController.visualItems.3.command');
    expect(itemFactsPath(3)).toBe('view.itemFacts(3)');
    expect(itemFields(json('{"fields":[{"name":"Hold","label":"Hold","control":"number","path":"p"}]}'))).toHaveLength(1);
  });

  it("landing altitudes carry the pattern's altitude frame like AltitudeFactTextField", () => {
    const fields = itemFields(
      json(`{"fields":[
            {"name":"finalApproachAltitude","label":"Altitude","control":"number","units":"m","path":"plan.missionController.visualItems.4.finalApproachAltitude"},
            {"name":"landingAltitude","label":"Altitude","control":"number","units":"m","path":"plan.missionController.visualItems.4.landingAltitude"},
            {"name":"loiterRadius","label":"Radius","control":"number","units":"m","path":"plan.missionController.visualItems.4.loiterRadius"}]}`),
    );
    expect(withLandingFrameUnits(fields, true).map((it) => it.units)).toEqual(['m Rel', 'm Rel', 'm']);
    expect(withLandingFrameUnits(fields, false).map((it) => it.units)).toEqual(['m AMSL', 'm AMSL', 'm']);
    expect(withLandingFrameUnits(fields, null).map((it) => it.units)).toEqual(['m', 'm', 'm']);
  });

  it('a section header sits above the first row of each landing editor section', () => {
    const view = json(`{"fields":[
            {"path":"a","section":"Final approach"},{"path":"b","section":"Final approach"},
            {"path":"c","section":"Landing point"},{"path":"d","section":"Landing point"},{"path":"e"}]}`);
    expect(sectionStarts(view)).toEqual(
      new Map([
        ['a', 'Final approach'],
        ['c', 'Landing point'],
      ]),
    );
    expect(sectionStarts(json('{"fields":[{"path":"a"}]}')).size).toBe(0);
  });

  it('distance and glide slope rows carry the radio button that picks between them', () => {
    const view = json(`{"fields":[
            {"path":"i.landingDistance","choice":{"path":"i.valueSetIsDistance","value":true,"selected":false}},
            {"path":"i.glideSlope","choice":{"path":"i.valueSetIsDistance","value":false,"selected":true}},{"path":"i.landingHeading"}]}`);
    expect(radioChoices(view)).toEqual(
      new Map([
        ['i.landingDistance', { path: 'i.valueSetIsDistance', value: true, selected: false }],
        ['i.glideSlope', { path: 'i.valueSetIsDistance', value: false, selected: true }],
      ]),
    );
  });

  it('the speed section is offered only where the item has one', () => {
    const speed = speedSection(json('{"speedSection":{"available":true,"specified":true,"value":8.5,"units":"m/s","path":"p","specifyPath":"s"}}'))!;
    expect(speed.specified).toBe(true);
    expect(speed.value).toBe(8.5);
    expect(speedSection(json('{"speedSection":{"available":false}}'))).toBeNull();
    expect(speedSection(json('{"speedSection":null}'))).toBeNull();
  });

  it('position forms seed the UTM and MGRS fields', () => {
    const forms = positionForms(json('{"utm":{"zone":32,"southern":false,"easting":464617.5,"northing":5247152.25},"mgrs":"32T MN 64617 47152"}'))!;
    expect(forms.zone).toBe('32');
    expect(forms.easting).toBe('464617.50');
    expect(forms.mgrs).toBe('32T MN 64617 47152');
    expect(utmToGeoPath('1', '2', '32', true)).toBe('view.utmToGeo(1,2,32,true)');
    expect(geoOf(json('{"valid":true,"latitude":47.0,"longitude":8.0}'))).toEqual([47.0, 8.0]);
    expect(geoOf(json('{"valid":false}'))).toBeNull();
  });

  it("a landing pattern's notes come from the core in order", () => {
    const view = json('{"landing":true,"landingNotes":["* Actual flight path will vary.","* Avoid tailwind on approach to land."]}');
    expect(landingNotes(view)).toEqual(['* Actual flight path will vary.', '* Avoid tailwind on approach to land.']);
    expect(landingNotes(json('{}'))).toEqual([]);
  });
});
