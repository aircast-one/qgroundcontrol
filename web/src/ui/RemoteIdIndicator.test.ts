import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { makeFact } from '@/bridge/Qgc';
import { remoteIdRows, remoteIdStatus, selfIdFacts } from './RemoteIdIndicator';

const setting = (name: string, value: string) =>
  makeFact({ path: `settings.remoteIDSettings.${name}`, name, description: '', units: '', valueString: value, value, enumStrings: [], enumIndex: -1, isBool: false, isString: true, readOnly: false });

describe('RemoteIdIndicatorTest', () => {
  it('rows follow the indicator page', () => {
    const status = remoteIdStatus(
      parseJson('{"shown":true,"state":"warning","comms":true,"armStatus":true,"gcsGps":false,"basicId":true,"operatorIdShown":false,"operatorId":false,"emergency":false,"emergencyHoldMs":800}'),
    )!;
    expect(remoteIdRows(status).map(([label]) => label)).toEqual(['RID COMMS', 'ARM STATUS', 'GCS GPS', 'BASIC ID']);
    expect(status.holdMs).toBe(800);
    const offline = { ...status, comms: false };
    expect(remoteIdRows(offline)).toEqual([['NOT CONNECTED', false]]);
    expect(remoteIdStatus(parseJson('{"shown":false}'))).toBeNull();
  });

  it("self id lists QGC's facts in order and greys the message fields while broadcast is off", () => {
    const page = [setting('selfIDFree', 'x'), setting('region', '0'), setting('selfIDEmergency', 'help'), setting('sendSelfID', 'false'), setting('selfIDType', '0')];
    const off = selfIdFacts(page);
    expect(off.map((fact) => fact.name)).toEqual(['sendSelfID', 'selfIDType', 'selfIDFree', 'selfIDEmergency']);
    expect(off.map((fact) => fact.acceptsWrite)).toEqual([true, false, false, true]);
    expect(
      off.slice(0, 2).map((fact) => fact.title),
      'RemoteIDIndicatorPage names the switch and the combo itself',
    ).toEqual(['Broadcast', 'Broadcast message']);
    expect(selfIdFacts(page.map((fact) => (fact.name === 'sendSelfID' ? makeFact({ ...fact, valueString: 'true', value: true }) : fact))).map((fact) => fact.acceptsWrite)).toEqual([true, true, true, true]);
  });
});
