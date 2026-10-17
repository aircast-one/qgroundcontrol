import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { surveyAreaFeatures, surveyTransectFeatures } from './MissionLayers';
import { SHAPE_AREA, SHAPE_LINE, SurveyBridge } from './SurveyBridge';

const point = (latitude: number, longitude: number) => `{"latitude":${latitude},"longitude":${longitude}}`;

const threeCorners = [point(41.0, 44.0), point(41.0, 44.1), point(41.1, 44.1)];

const survey = ({
  corners = threeCorners,
  transects = [point(41.0, 44.0), point(41.0, 44.1)],
  shots = 12,
  kind = 'survey',
  shape = 'area',
  property = 'surveyAreaPolygon',
}: { corners?: readonly string[]; transects?: readonly string[]; shots?: number; kind?: string; shape?: string; property?: string } = {}) =>
  `{"kind":"${kind}","cameraShots":${shots},` +
  `"geometry":{"shape":"${shape}","property":"${property}",` +
  `"vertices":[${corners.join(',')}],` +
  `"transects":[${transects.join(',')}]}}`;

const plan = (...items: string[]) => parseJson(`{"kind":"object","items":[${items.join(',')}]}`);

const found = (json: Json | null) => SurveyBridge.surveysFrom(json);

describe('SurveyBridgeTest', () => {
  it('a survey carries its area transects and shot count', async () => {
    const surveys = await found(plan('{"kind":"settings"}', survey()));

    expect(surveys.length).toBe(1);
    expect(surveys[0]?.index).toBe(1);
    expect(surveys[0]?.area.length).toBe(3);
    expect(surveys[0]?.transects.length).toBe(2);
    expect(surveys[0]?.cameraShots).toBe(12);
  });

  it('items that are not surveys are ignored', async () => {
    expect((await found(plan('{"kind":"waypoint"}', '{"kind":"takeoff"}'))).length).toBe(0);
  });

  it('a corridor scan is a line, not an area, and says which property holds it', async () => {
    const corridor = await found(plan('{"kind":"settings"}', survey({ kind: 'corridor', shape: 'line', property: 'corridorPolyline' })));

    expect(corridor.length).toBe(1);
    expect(corridor[0]?.shape).toBe(SHAPE_LINE);
    expect(corridor[0]?.property).toBe('corridorPolyline');
    expect(corridor[0]?.kind).toBe('corridor');
  });

  it('a structure scan is an area drawn from its own property', async () => {
    const structure = await found(plan(survey({ kind: 'structure', property: 'structurePolygon' })));

    expect(structure[0]?.shape).toBe(SHAPE_AREA);
    expect(structure[0]?.property).toBe('structurePolygon');
  });

  it('geometry without a property cannot be dragged, so it is not shown', async () => {
    const noProperty = `{"kind":"corridor","geometry":{"shape":"line","vertices":[${threeCorners.join(',')}],"transects":[]}}`;

    expect((await found(plan(noProperty))).length).toBe(0);
  });

  it('a survey the core gave no geometry is not shown', async () => {
    expect((await found(plan('{"kind":"survey","cameraShots":3}'))).length).toBe(0);
  });

  it('transects that have not arrived yet leave the area drawable', async () => {
    const surveys = await found(plan(survey({ transects: [] })));

    expect(surveys[0]?.area.length).toBe(3);
    expect(surveys[0]?.transects.length).toBe(0);
  });

  it('unusable points are dropped from the area and transects', async () => {
    const surveys = await found(plan(survey({ transects: [point(41.0, 44.0), point(0.0, 0.0), point(41.2, 44.2)] })));

    expect(surveys[0]?.transects.length).toBe(2);
  });

  it('an area needs three corners before it is drawn', async () => {
    const thin = survey({ corners: [point(41.0, 44.0), point(41.0, 44.1)] });

    expect(surveyAreaFeatures(await found(plan(thin))).features.length).toBe(0);
  });

  it('transects need two points before they are drawn', async () => {
    const single = survey({ transects: [point(41.0, 44.0)] });

    expect(surveyTransectFeatures(await found(plan(single))).features.length).toBe(0);
  });

  it('an absent plan yields nothing', async () => {
    expect((await found(null)).length).toBe(0);
  });
});
