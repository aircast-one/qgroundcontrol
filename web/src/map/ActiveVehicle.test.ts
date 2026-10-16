import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import {
  CHOOSER_TITLE,
  VehicleChoice,
  activeChanged,
  activeVehicleTitle,
  handoverNotice,
  linkDistinguishes,
  lostVehicles,
  lostVehiclesText,
  rememberedChoices,
  uploadHeading,
  vehicleChoiceLine,
  vehicleChoices,
} from './VehiclePicker';
import type { UploadGate } from './VehicleSync';

const json = (text: string): Json => parseJson(text) ?? {};

const two = json(`
  {"count":2,"activeId":1,"ambiguous":true,"vehicles":[
    {"id":1,"name":"Quadrotor 1","link":"TCP 127.0.0.1:5771","active":true,
     "armed":false,"flying":false,"flightMode":"Stabilize","contactLost":false},
    {"id":2,"name":"Quadrotor 2","link":"TCP 127.0.0.1:5772","active":false,
     "armed":true,"flying":true,"flightMode":"Guided","contactLost":true}
  ]}
`);

const one = json(`
  {"count":1,"activeId":1,"ambiguous":false,"vehicles":[
    {"id":1,"name":"Quadrotor 1","link":"TCP 127.0.0.1:5771","active":true,
     "armed":false,"flying":false,"flightMode":"Stabilize","contactLost":false}
  ]}
`);

const single = <T,>(items: readonly T[], match: (item: T) => boolean = () => true): T => {
  const found = items.filter(match);
  if (found.length !== 1) throw new Error(`expected exactly one, found ${found.length}`);
  return found[0] as T;
};

const gate = (heading: string): UploadGate => ({
  canSend: true,
  canProceed: true,
  pausesFirst: false,
  heading,
  refusal: '',
  proceedTitle: 'Upload',
});

const listOfVehicles = (ids: readonly number[], activeId: number) =>
  vehicleChoices(json(`{"ambiguous":${ids.length > 1},"vehicles":[${ids.map((it) => `{"id":${it},"name":"Quadrotor ${it}","active":${it === activeId}}`).join(',')}]}`));

describe('ActiveVehicleTest', () => {
  it('the header names the aircraft only when more than one is connected', () => {
    expect(
      activeVehicleTitle(vehicleChoices(one), 'Stabilize · Disarmed'),
      'one vehicle needs no name: the header is about the only thing it can be about',
    ).toBe('Stabilize · Disarmed');
    expect(
      activeVehicleTitle(vehicleChoices(two), 'Stabilize · Disarmed'),
      'with two connected, Arm and Takeoff act on one of them and the header is the only thing that says which',
    ).toBe('Quadrotor 1 · Stabilize · Disarmed');
  });

  it('a vehicle out of contact says so instead of what it was last doing', () => {
    const lost = single(vehicleChoices(two).choices, (it) => it.id === 2);
    expect(vehicleChoiceLine(lost)).toBe('No contact · TCP 127.0.0.1:5772');
    expect(lost.contactLost).toBe(true);
  });

  it('one radio carrying the whole fleet prints its name under nobody', () => {
    const first = VehicleChoice({ id: 1, name: 'Quadrotor 1', state: 'Stabilize · Disarmed', link: 'SiK', active: true, contactLost: false, latitude: 0.0, longitude: 0.0 });
    const second = { ...first, id: 2, name: 'Quadrotor 2', active: false };
    expect(
      linkDistinguishes([first, second]),
      'the link is drawn to tell two vehicles apart; the same string under every name is the noise it was added to cut, and it pushes the state text off a narrow row',
    ).toBe(false);
    expect(vehicleChoiceLine(first, linkDistinguishes([first, second]))).toBe('Stabilize · Disarmed');
  });

  it('two radios keep their names, because that is the question being asked', () => {
    const a = VehicleChoice({ id: 1, name: 'A', state: 'Disarmed', link: 'TCP 5771', active: true, contactLost: false, latitude: 0.0, longitude: 0.0 });
    const b = { ...a, id: 2, name: 'B', link: 'TCP 5772', active: false, contactLost: true };
    expect(linkDistinguishes([a, b])).toBe(true);
    expect(vehicleChoiceLine(b, true)).toBe('No contact · TCP 5772');
  });

  it('a vehicle in contact says what it is doing and which link carries it', () => {
    const live = single(vehicleChoices(two).choices, (it) => it.id === 1);
    expect(vehicleChoiceLine(live)).toBe('Stabilize · Disarmed · TCP 127.0.0.1:5771');
    expect(live.active).toBe(true);
  });

  it('a vehicle that has not reported a mode still says whether it is armed', () => {
    const early = json(`{"ambiguous":true,"vehicles":[{"id":4,"name":"Quadrotor 4","active":false,
      "contactLost":false,"flightMode":"","armed":true,"flying":false}]}`);

    expect(
      single(vehicleChoices(early).choices).state,
      'the mode arrives after the heartbeat does, so there is a window where the only thing known is that the propellers are live - losing that word to a blank mode would drop the half that matters',
    ).toBe('Armed');
  });

  it('flying outranks armed, because an armed vehicle on the ground is a different thing', () => {
    const flying = single(vehicleChoices(two).choices, (it) => it.id === 2);
    expect(flying.state).toBe('Guided · Flying');
  });

  it('a contactLost the core could not answer is not read as a loss', () => {
    const unknown = json('{"ambiguous":true,"vehicles":[{"id":3,"name":"Rover 3","active":false,"contactLost":null}]}');
    expect(
      single(vehicleChoices(unknown).choices).contactLost,
      'null means the vehicle does not track contact loss at all, which is not the same as having lost it',
    ).toBe(false);
  });

  it('an empty view offers nothing rather than an empty name', () => {
    expect(vehicleChoices(null).choices.length).toBe(0);
    expect(vehicleChoices(null).ambiguous).toBe(false);
    expect(activeVehicleTitle(vehicleChoices(null), 'No vehicle')).toBe('No vehicle');
  });

  it('a vehicle the core did not name still has something to tap', () => {
    const nameless = json('{"ambiguous":true,"vehicles":[{"id":7,"active":false}]}');
    expect(single(vehicleChoices(nameless).choices).name).toBe('Vehicle 7');
  });

  it('a vehicle that stopped answering is named even while another is being flown', () => {
    const lost = lostVehicles(vehicleChoices(two));
    expect(lost.map((it) => it.id)).toEqual([2]);
    expect(lostVehiclesText(lost), 'QGC announces this by voice and nowhere else, so the screen is the only place an operator can see it').toBe('Quadrotor 2 is not answering');
  });

  it('the vehicle being flown is not counted among the silent ones', () => {
    const activeLost = json(`{"ambiguous":true,"vehicles":[
      {"id":1,"name":"Quadrotor 1","active":true,"contactLost":true},
      {"id":2,"name":"Quadrotor 2","active":false,"contactLost":false}]}`);
    expect(
      lostVehicles(vehicleChoices(activeLost)).length === 0,
      'the header already says Communication lost for the one in command, and saying it twice reads as two failures',
    ).toBe(true);
  });

  it('more than one silent vehicle is counted rather than listed', () => {
    const many = json(`{"ambiguous":true,"vehicles":[
      {"id":1,"name":"Quadrotor 1","active":true,"contactLost":false},
      {"id":2,"name":"Quadrotor 2","active":false,"contactLost":true},
      {"id":3,"name":"Quadrotor 3","active":false,"contactLost":true}]}`);
    expect(lostVehiclesText(lostVehicles(vehicleChoices(many)))).toBe('2 other vehicles are not answering');
  });

  it('nothing is said when every vehicle is answering', () => {
    expect(lostVehiclesText(lostVehicles(vehicleChoices(one)))).toBeNull();
  });

  it('the upload names the aircraft it is about to send to', () => {
    expect(uploadHeading(gate(''), vehicleChoices(two)), 'a mission is being written to one of two aircraft and the dialog covers the header that says which').toBe('Upload this plan to Quadrotor 1?');
    expect(uploadHeading(gate('Send the plan to the vehicle'), vehicleChoices(two))).toBe('Send the plan to the vehicle to Quadrotor 1');
  });

  it("one vehicle leaves the core's question exactly as it was", () => {
    expect(uploadHeading(gate(''), vehicleChoices(one))).toBe('Upload this plan?');
    expect(
      uploadHeading(gate('Replace the plan on the vehicle?'), vehicleChoices(one)),
      'the core writes this heading when a plan is being replaced, and a name on the only aircraft connected adds nothing',
    ).toBe('Replace the plan on the vehicle?');
  });

  it('the aircraft is handed over before the one that left is removed', () => {
    expect(
      handoverNotice(listOfVehicles([1, 2], 1), listOfVehicles([1, 2], 2), null),
      'QGC promotes the survivor FIRST and deletes the departing vehicle a moment later, so a rule keyed on the old one being gone never fires - measured on the emulator',
    ).toBe('Quadrotor 1 stopped answering. Now flying Quadrotor 2.');
    expect(handoverNotice(listOfVehicles([1, 2], 1), listOfVehicles([2], 2), null)).toBe('Quadrotor 1 is gone. Now flying Quadrotor 2.');
  });

  it('an operator switching vehicles is told nothing, having just done it', () => {
    expect(
      handoverNotice(listOfVehicles([1, 2], 1), listOfVehicles([1, 2], 2), 2),
      'the same list change as a handover, and only the head knows it asked for this one',
    ).toBeNull();
  });

  it('a request lasts exactly one change of vehicle', () => {
    expect(
      activeChanged(listOfVehicles([1, 2], 1), listOfVehicles([1, 2], 2)),
      'one transition is all a request can explain; keeping it past that silences the next handover, and a request the core accepted but the vehicle never honoured would silence every handover after it',
    ).toBe(true);
    expect(activeChanged(listOfVehicles([1, 2], 1), listOfVehicles([1, 2], 1))).toBe(false);
    expect(
      activeChanged(listOfVehicles([1, 2], 1), vehicleChoices(json('{"vehicles":[{"id":2,"name":"Quadrotor 2","active":false}]}'))),
      'the gap with nothing active is not a change, or the request would be spent before it landed',
    ).toBe(false);
    expect(activeChanged(null, listOfVehicles([1], 1))).toBe(false);
  });

  it('a handover back to a vehicle the operator once chose is still announced', () => {
    expect(
      handoverNotice(listOfVehicles([1, 2], 1), listOfVehicles([1, 2], 2), null),
      'asked for 2 an hour ago, flew 1, then 1 died - measured on the emulator, where a kept request swallowed the notice entirely',
    ).toBe('Quadrotor 1 stopped answering. Now flying Quadrotor 2.');
  });

  it('an old request does not silence the next handover', () => {
    expect(handoverNotice(listOfVehicles([1, 2], 2), listOfVehicles([1, 2], 1), 2)).toBe('Quadrotor 2 stopped answering. Now flying Quadrotor 1.');
  });

  it('the other vehicle leaving is not a handover', () => {
    expect(handoverNotice(listOfVehicles([1, 2], 1), listOfVehicles([1], 1), null), 'the one being flown did not change, and its silence is already on the header').toBeNull();
  });

  it('the last vehicle leaving says nothing, because No vehicle already does', () => {
    expect(handoverNotice(listOfVehicles([1], 1), vehicleChoices(null), null)).toBeNull();
  });

  it('the first reading announces nothing', () => {
    expect(handoverNotice(null, listOfVehicles([1, 2], 1), null)).toBeNull();
  });

  it('the gap between one vehicle leaving and the next being promoted is not a reading', () => {
    const flying = listOfVehicles([1, 2], 1);
    const gap = vehicleChoices(json('{"ambiguous":false,"vehicles":[{"id":2,"name":"Quadrotor 2","active":false}]}'));
    const promoted = listOfVehicles([2], 2);

    expect(
      rememberedChoices(flying, gap),
      "measured on the emulator: killing the active vehicle's link emits THREE times - two vehicles with 1 active, one vehicle with NO active, then 2 promoted. Keeping the middle one loses who was being flown and the handover goes unannounced",
    ).toEqual(flying);
    expect(rememberedChoices(flying, promoted)).toEqual(promoted);
    expect(handoverNotice(rememberedChoices(flying, gap), promoted, null), 'by the time the survivor is promoted the old vehicle has already left the list').toBe(
      'Quadrotor 1 is gone. Now flying Quadrotor 2.',
    );
  });

  it('losing every vehicle forgets the one that was being flown', () => {
    const flying = listOfVehicles([1], 1);
    expect(
      rememberedChoices(flying, vehicleChoices(null)),
      'No vehicle is already on the header, and the next aircraft to connect is a new flight rather than a handover from the one that went away',
    ).toBeNull();
  });

  it('the chooser is titled as the question it answers, not as a state', () => {
    expect(
      CHOOSER_TITLE,
      'it read Flying, which is a word for what the aircraft is doing - and the sheet opens just as readily on a disarmed one sitting on the ground',
    ).toBe('Fly which aircraft?');
  });
});
