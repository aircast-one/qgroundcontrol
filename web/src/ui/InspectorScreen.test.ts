import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import {
  InspectorMessage,
  inspectorChoices,
  inspectorEmptyText,
  inspectorMessages,
  inspectorRateChoices,
  inspectorShown,
  inspectorSystemText,
  messageIndexIn,
  openMessageIn,
  parseInspectorFields,
  selectedPathFor,
} from './InspectorScreen';

describe('InspectorScreenTest', () => {
  const path = 'mavlinkInspector.activeSystem.messages.3';

  const view = (selectedPath: string) =>
    parseJson(`{"messages":[{"path":"${selectedPath}","selected":true}],
      "fields":[
        {"name":"roll","type":"float","value":"0.001"},
        {"name":"pitch","type":"float","value":"0.002"}
      ]}`);

  it('fields keep their declared order', () => {
    const fields = parseInspectorFields(view(path), path);
    expect(fields.map((it) => it.name)).toEqual(['roll', 'pitch']);
    expect(fields[0]).toEqual({ name: 'roll', type: 'float', value: '0.001' });
  });

  it('fields of the previous selection are not drawn under the new message', () => {
    expect(parseInspectorFields(view('mavlinkInspector.activeSystem.messages.1'), path)).toEqual([]);
    expect(parseInspectorFields(null, path)).toEqual([]);
  });
});

describe('InspectorOpenMessageTest', () => {
  const message = (index: number, name: string) =>
    InspectorMessage({ index, id: index, name, rateText: '1.0 Hz', count: 1, path: `mavlinkInspector.activeSystem.messages.${index}`, compId: 1, title: name });

  it('the open message follows its name when the model is rebuilt at other indices', () => {
    const before = [message(0, 'HEARTBEAT'), message(1, 'ATTITUDE')];
    const after = [message(0, 'SYS_STATUS'), message(1, 'HEARTBEAT'), message(2, 'ATTITUDE')];

    expect(openMessageIn(before, 'mavlinkInspector.activeSystem.messages.1')?.index).toBe(1);
    expect(openMessageIn(after, 'mavlinkInspector.activeSystem.messages.2')?.index).toBe(2);
  });

  it('a message that leaves the model closes rather than showing another one', () => {
    expect(openMessageIn([message(0, 'HEARTBEAT')], 'ATTITUDE')).toBeNull();
  });

  it('nothing open resolves to nothing', () => {
    expect(openMessageIn([message(0, 'HEARTBEAT')], null)).toBeNull();
  });

  it("the table reads the core's rate text and path", () => {
    const messages = inspectorMessages(
      parseJson(`{"messages":[
        {"index":1,"id":30,"name":"ATTITUDE","rateText":"10.0 Hz","count":420,
         "path":"mavlinkInspector.activeSystem.messages.1"},
        {"index":0,"id":0,"name":"HEARTBEAT","rateText":"1.0 Hz","count":42,
         "path":"mavlinkInspector.activeSystem.messages.0"}]}`),
    );

    expect(messages.map((it) => it.name)).toEqual(['ATTITUDE', 'HEARTBEAT']);
    expect(messages.map((it) => it.rateText)).toEqual(['10.0 Hz', '1.0 Hz']);
    expect(messages.map((it) => it.index)).toEqual([1, 0]);
  });

  it('no messages is an empty table, not a crash', () => {
    expect(inspectorMessages(null)).toEqual([]);
    expect(inspectorMessages(parseJson('{}'))).toEqual([]);
  });

  it('two components sending the same message are distinct rows', () => {
    const messages = inspectorMessages(
      parseJson(`{"messages":[
        {"index":0,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
         "compId":100,"path":"mavlinkInspector.systems.1.messages.0"},
        {"index":1,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
         "compId":101,"path":"mavlinkInspector.systems.1.messages.1"}]}`),
    );

    expect(messages.length).toBe(2);
    expect(new Set(messages.map((it) => it.path)).size).toBe(2);
  });

  it("the row title is the core's, falling back to the name when absent", () => {
    const served = inspectorMessages(
      parseJson(`{"messages":[
        {"index":0,"id":271,"name":"CAMERA_CAPTURE_STATUS","rateText":"1.0 Hz","count":9,
         "compId":100,"path":"p0","title":"CAMERA_CAPTURE_STATUS (comp 100)"},
        {"index":1,"id":30,"name":"ATTITUDE","rateText":"5.0 Hz","count":99,
         "compId":1,"path":"p1"}]}`),
    );

    expect(served.find((it) => it.path === 'p0')?.title).toBe('CAMERA_CAPTURE_STATUS (comp 100)');
    expect(served.find((it) => it.path === 'p1')?.title).toBe('ATTITUDE');
  });

  it('the fields and the selection follow the served path, not a rebuilt one', () => {
    const path = 'mavlinkInspector.systems.0.messages.3';

    expect(selectedPathFor(path)).toBe('mavlinkInspector.systems.0.selected');
    expect(messageIndexIn(path)).toBe(3);
  });

  it('a path the core words differently still yields its own system', () => {
    expect(selectedPathFor('mavlinkInspector.systems.2.messages.11')).toBe('mavlinkInspector.systems.2.selected');
    expect(messageIndexIn('mavlinkInspector.systems.2.messages.11')).toBe(11);
  });
});

describe('InspectorRateTest', () => {
  it('no view offers no rates', () => {
    expect(inspectorRateChoices(null)).toEqual([]);
  });

  it('rate choices come from the core with their titles', () => {
    const choices = inspectorRateChoices(parseJson('{"rateChoices":[{"rate":-1,"title":"Disabled"},{"rate":0,"title":"Default"},{"rate":10,"title":"10 Hz"}]}'));
    expect(choices.length).toBe(3);
    expect(choices[0]?.rate).toBe(-1);
    expect(choices[0]?.title).toBe('Disabled');
    expect(choices[2]?.title).toBe('10 Hz');
  });

  it('a choice with no title is not offered', () => {
    const choices = inspectorRateChoices(parseJson('{"rateChoices":[{"rate":5,"title":""},{"rate":6,"title":"6 Hz"}]}'));
    expect(choices.length).toBe(1);
    expect(choices[0]?.rate).toBe(6);
  });

  it('the target rate title is decoded onto the message', () => {
    expect(inspectorMessages(parseJson('{"messages":[{"index":0,"id":30,"name":"ATTITUDE","targetRateTitle":"10 Hz"}]}'))[0]?.targetRateTitle).toBe('10 Hz');
  });
});

describe('InspectorSystemTest', () => {
  it('the inspector names the system its messages came from', () => {
    expect(inspectorSystemText(parseJson('{"kind":"object","class":"MavlinkInspector","available":true,"systemId":1,"messages":[]}'))).toBe('System 1');
  });

  it('no system yet is no line rather than System 0', () => {
    expect(inspectorSystemText(parseJson('{"kind":"object","class":"MavlinkInspector","available":true,"systemId":0}'))).toBeNull();
    expect(inspectorSystemText(parseJson('{"kind":"object","class":"MavlinkInspector","available":false,"systemId":3}'))).toBeNull();
    expect(inspectorSystemText(null)).toBeNull();
  });

  it("the empty state is the core's sentence, not the head's", () => {
    const noVehicle = parseJson('{"kind":"object","class":"Inspector","available":false,"messages":[],"emptyText":"Connect a vehicle to inspect its MAVLink traffic."}');
    expect(inspectorEmptyText(noVehicle)).toBe('Connect a vehicle to inspect its MAVLink traffic.');
  });

  it('a vehicle with rows has nothing to say instead of them', () => {
    const flowing = parseJson('{"kind":"object","class":"Inspector","available":true,"systemId":1,"messages":[{"name":"HEARTBEAT"}],"emptyText":""}');
    expect(inspectorEmptyText(flowing)).toBeNull();
  });

  it('a vehicle that has sent nothing yet says so, and the head does not invent it', () => {
    const waiting = parseJson('{"kind":"object","class":"Inspector","available":true,"systemId":1,"messages":[],"emptyText":"Waiting for this vehicle\'s first message…"}');
    expect(inspectorEmptyText(waiting)).toBe("Waiting for this vehicle's first message…");
  });

  it('the component filter narrows the list the way the inspector combo does', () => {
    const heartbeat = InspectorMessage({ index: 0, id: 0, name: 'HEARTBEAT', rateText: '', count: 1, path: 'p0', compId: 1, title: 'HEARTBEAT' });
    const camera = InspectorMessage({ index: 1, id: 262, name: 'CAMERA_CAPTURE_STATUS', rateText: '', count: 1, path: 'p1', compId: 100, title: 'CAMERA_CAPTURE_STATUS' });
    expect(inspectorShown([heartbeat, camera], '', 100)).toEqual([camera]);
    expect(inspectorShown([heartbeat, camera], '', null)).toEqual([heartbeat, camera]);
    expect(inspectorChoices(parseJson('{"systems":[{"id":2,"title":"System 2"}]}'), 'systems')).toEqual([{ id: 2, title: 'System 2' }]);
  });
});
