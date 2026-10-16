import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { LinkCell, linkCell, vehicleLinks } from './VehicleLinks';

const json = (text: string): Json => parseJson(text) ?? {};

describe('VehicleLinksTest', () => {
  const view = ({
    available = true,
    watching = true,
    primary = '"LTE"',
    contactLost = 'false',
    links = `[{"name":"LTE","primary":true,"commLost":false},
              {"name":"radio","primary":false,"commLost":false}]`,
  }: { available?: boolean; watching?: boolean; primary?: string; contactLost?: string; links?: string } = {}) =>
    json(`{"kind":"object","class":"VehicleLinks","available":${available},"watching":${watching},
       "primary":${primary},"contactLost":${contactLost},"reason":"","links":${links}}`);

  it('another view is not a links reading', () => {
    expect(vehicleLinks(null)).toBeNull();
    expect(vehicleLinks(json('{"kind":"object","class":"AdsbTraffic"}'))).toBeNull();
  });

  it('one link is the ordinary case and says nothing', () => {
    const single = view({ links: '[{"name":"LTE","primary":true,"commLost":false}]' });
    expect(linkCell(vehicleLinks(single))).toBeNull();
  });

  it('two healthy links say so, because redundancy is the thing being reported', () => {
    expect(linkCell(vehicleLinks(view()))).toEqual(LinkCell('2 links', false));
  });

  it('a silent radio is counted, not named - the strip has no room for a link name', () => {
    const degraded = view({ links: '[{"name":"LTE","primary":true,"commLost":false},{"name":"radio","primary":false,"commLost":true}]' });
    expect(linkCell(vehicleLinks(degraded))).toEqual(LinkCell('1 link lost', true));
  });

  it('every link silent is a different sentence from one of them silent', () => {
    const gone = view({ contactLost: 'true', links: '[{"name":"LTE","primary":true,"commLost":true},{"name":"radio","primary":false,"commLost":true}]' });
    expect(linkCell(vehicleLinks(gone))).toEqual(LinkCell('no link heard', true));
  });

  it('an unwatched vehicle reports no loss, and a null is not a false', () => {
    const unwatched = view({ watching: false, contactLost: 'null', links: '[{"name":"LTE","primary":true,"commLost":null},{"name":"radio","primary":false,"commLost":null}]' });
    const reading = vehicleLinks(unwatched)!;
    expect(reading.links[0]!.commLost).toBeNull();
    expect(linkCell(reading)).toEqual(LinkCell('2 links', false));
  });

  it('no vehicle is no cell', () => {
    expect(linkCell(vehicleLinks(view({ available: false, links: '[]' })))).toBeNull();
  });

  it('two of three silent is not one of them', () => {
    const worse = view({
      links: '[{"name":"LTE","primary":true,"commLost":false},{"name":"radio","primary":false,"commLost":true},{"name":"spare","primary":false,"commLost":true}]',
    });
    expect(linkCell(vehicleLinks(worse))).toEqual(LinkCell('2 links lost', true));
  });

  it('one radio going quiet says nothing here, and the no-contact banner is why', () => {
    const alone = json('{"kind":"object","class":"VehicleLinks","available":true,"links":[{"name":"Telemetry","primary":true,"commLost":true}]}');
    expect(linkCell(vehicleLinks(alone)), 'with one link there is no which').toBeNull();
  });
});
