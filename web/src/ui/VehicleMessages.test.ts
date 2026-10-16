import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { readinessSubtitle } from './ActiveVehicle';
import { FlyState } from './FlyState';
import {
  MessageSeverity,
  OLDEST_FIRST,
  VehicleMessage,
  armingBlocker,
  bannerMessages,
  bannerText,
  chipBlocker,
  levelOf,
  messageCountText,
  messageTime,
  severitySummary,
  unreadCount,
  unreadMessages,
  vehicleMessages,
} from './VehicleMessages';

const json = (text: string): Json => parseJson(text) ?? {};

describe('VehicleMessagesTest', () => {
  const message = (index: number, level: MessageSeverity, text: string) => VehicleMessage({ index, time: '12:00', severity: level.name.toLowerCase(), level, text });
  const oldestFirst = (...messages: VehicleMessage[]) => messages;

  it('the log summary counts errors and warnings apart from the rest', () => {
    const plain = (level: MessageSeverity) => VehicleMessage({ index: 0, time: '', severity: '', level, text: 'x' });
    expect(severitySummary([plain(MessageSeverity.Error), plain(MessageSeverity.Warning), plain(MessageSeverity.Warning), plain(MessageSeverity.Normal)])).toBe('1 error · 2 warnings · 1 message');
    expect(severitySummary([])).toBe('0 messages from the vehicle');
  });

  it('the chip carries the arming blocker as a short reason', () => {
    expect(chipBlocker('PreArm: GPS 1: not healthy')).toBe('GPS 1: not healthy');
    expect(chipBlocker('No GPS lock. This vehicle needs a position fix before it will arm.')).toBe('No GPS lock');
    expect(chipBlocker('Throttle too high')).toBe('Throttle too high');
    const fly = FlyState({
      connected: true,
      armed: false,
      contactLost: false,
      state: 'disarmed',
      stateText: 'Not ready',
      staleNotice: '',
      mode: 'Guided',
      rcSupported: false,
      rcSignalText: '',
      rcSignal: null,
      rcOverride: null,
      telemetry: null,
    });
    expect(readinessSubtitle(fly, 'PreArm: GPS 1: not healthy', 3)).toBe('Guided · GPS 1: not healthy +2');
    expect(readinessSubtitle(fly, 'Compass not calibrated', 1)).toBe('Guided · Compass not calibrated');
    expect(readinessSubtitle(fly, null, 0)).toBeNull();
  });

  it('the banner only interrupts for warnings and errors the chip is not already saying', () => {
    const plain = (level: MessageSeverity, text: string) => VehicleMessage({ index: 0, time: '', severity: '', level, text });
    const unread = [
      VehicleMessage({ index: 0, time: '', severity: '', level: MessageSeverity.Error, text: 'PreArm: GPS 1: not healthy', toldElsewhere: true }),
      plain(MessageSeverity.Normal, 'EKF3 IMU0 is using GPS'),
      plain(MessageSeverity.Warning, 'Battery low'),
    ];
    expect(bannerMessages(unread).map((it) => it.text)).toEqual(['Battery low']);
    expect(bannerMessages(unread.slice(0, 2))).toEqual([]);
    expect(bannerText(bannerMessages(unread))).toBe('Battery low');
  });

  it('a message time drops the milliseconds QGC stamps it with', () => {
    expect(messageTime('20:00:53.747')).toBe('20:00:53');
    expect(messageTime('yesterday')).toBe('yesterday');
  });

  const served = `
    {"kind":"object","class":"VehicleMessages","order":"oldestFirst","items":[
      {"index":0,"time":"1:2:3.4","component":null,"severity":"Error","level":"error","text":"Battery < 20% & \\"low\\""},
      {"index":1,"time":"1:2:4.0","component":190,"severity":"Notice","level":"warning","text":"heads up"},
      {"index":2,"time":"1:2:5.0","component":null,"severity":"","level":"normal","text":"plain"}]}`;

  it('messages come from the core with their level and time', () => {
    const messages = vehicleMessages(json(served));
    expect(messages.map((it) => it.text)).toEqual(['Battery < 20% & "low"', 'heads up', 'plain']);
    expect(messages.map((it) => it.level)).toEqual([MessageSeverity.Error, MessageSeverity.Warning, MessageSeverity.Normal]);
    expect(messages.map((it) => it.time)).toEqual(['1:2:3.4', '1:2:4.0', '1:2:5.0']);
    expect(messages.map((it) => it.index)).toEqual([0, 1, 2]);
  });

  it("the level comes from the core's token, not from a word that gets translated", () => {
    const german = json('{"class":"VehicleMessages","order":"oldestFirst","items":[{"index":0,"time":"1:2:3.4","severity":"Fehler","level":"error","text":"kaputt"}]}');
    expect(vehicleMessages(german).length).toBe(1);
    expect(vehicleMessages(german)[0]!.level).toBe(MessageSeverity.Error);
    expect(vehicleMessages(german)[0]!.severity).toBe('Fehler');
  });

  it('an unknown level is treated as ordinary rather than dropped', () => {
    expect(levelOf('something-new')).toBe(MessageSeverity.Normal);
    expect(levelOf('')).toBe(MessageSeverity.Normal);
  });

  it('nothing to say reads as an empty list', () => {
    expect(vehicleMessages(null)).toEqual([]);
    expect(vehicleMessages(json('{"kind":"null"}'))).toEqual([]);
    expect(vehicleMessages(json('{"class":"VehicleMessages","items":[]}'))).toEqual([]);
  });

  it('a message with no text is not shown as a blank row', () => {
    const blank = json('{"class":"VehicleMessages","items":[{"index":0,"text":"","level":"error"}]}');
    expect(vehicleMessages(blank)).toEqual([]);
  });

  it('an arming blocker is read only when the core sets one', () => {
    expect(armingBlocker(json('{"armingBlocker":"Throttle too high"}'))).toBe('Throttle too high');
    expect(armingBlocker(json('{"armingBlocker":null}'))).toBeNull();
    expect(armingBlocker(json('{"armingBlocker":""}'))).toBeNull();
    expect(armingBlocker(null)).toBeNull();
  });

  it('an error is named in the banner, not buried in a count', () => {
    const messages = oldestFirst(message(0, MessageSeverity.Normal, 'Armed'), message(1, MessageSeverity.Error, 'EKF variance'), message(2, MessageSeverity.Normal, 'Mode changed'));
    expect(bannerText(messages)).toBe('EKF variance');
  });

  it('the newest error wins, and the core serves its lists oldest first', () => {
    const messages = oldestFirst(message(0, MessageSeverity.Error, 'Compass variance'), message(1, MessageSeverity.Error, 'EKF variance'));
    expect(bannerText(messages)).toBe('EKF variance');
  });

  it("the log renders newest first, which is the order QGC's own log uses", () => {
    const messages = oldestFirst(message(0, MessageSeverity.Normal, 'oldest'), message(1, MessageSeverity.Normal, 'newest'));
    expect(messages.toReversed().map((it) => it.text)).toEqual(['newest', 'oldest']);
  });

  it('a view that says newest first is turned round rather than trusted to match', () => {
    const body = `{"order":"newestFirst","items":[
        {"index":0,"time":"12:01","severity":"","level":"normal","text":"newest"},
        {"index":1,"time":"12:00","severity":"","level":"normal","text":"oldest"}]}`;
    expect(vehicleMessages(json(body)).map((it) => it.text)).toEqual(['oldest', 'newest']);
  });

  it('the order the core actually serves is taken as given', () => {
    const body = `{"order":"${OLDEST_FIRST}","items":[
        {"index":0,"time":"12:00","severity":"","level":"normal","text":"oldest"},
        {"index":1,"time":"12:01","severity":"","level":"normal","text":"newest"}]}`;
    expect(vehicleMessages(json(body)).map((it) => it.text)).toEqual(['oldest', 'newest']);
  });

  it('a warning is named when nothing worse has happened', () => {
    expect(bannerText([message(0, MessageSeverity.Warning, 'Low battery')])).toBe('Low battery');
  });

  it('routine chatter does not interrupt the pilot', () => {
    expect(bannerMessages([message(0, MessageSeverity.Normal, 'Armed'), message(1, MessageSeverity.Normal, 'Disarmed')])).toEqual([]);
  });

  it('nothing to say is nothing shown', () => {
    expect(bannerText([])).toBeNull();
  });
});

describe('VehicleMessagesContractTest', () => {
  it('the decoder reads the key the core actually serves', () => {
    const recorded = json(`{"kind":"object","class":"VehicleMessages","count":1,
        "items":[{"index":0,"time":"1:2:3.4","component":null,"severity":"Error","level":"error","text":"real"}]}`);
    expect(vehicleMessages(recorded).map((it) => it.text)).toEqual(['real']);
    expect(vehicleMessages(json('{"class":"VehicleMessages","messages":[{"text":"x"}]}')), 'a fixture invented by the head must not pass where the recorded one fails').toEqual([]);
  });

  it('opening the log marks what was there as read, as resetAllMessages does', () => {
    const messages = [
      VehicleMessage({ index: 0, time: '', severity: 'error', level: MessageSeverity.Error, text: 'EKF variance' }),
      VehicleMessage({ index: 1, time: '', severity: 'info', level: MessageSeverity.Normal, text: 'Armed' }),
    ];
    expect(unreadMessages(messages, 2).length).toBe(2);
    expect(unreadMessages(messages, 0), 'an old error no longer turns the banner red').toEqual([]);
    const newer = [...messages, VehicleMessage({ index: 2, time: '', severity: 'warning', level: MessageSeverity.Warning, text: 'Low battery' })];
    expect(unreadMessages(newer, 1).map((it) => it.text)).toEqual(['Low battery']);
    expect(unreadCount(json('{"unread":1}'))).toBe(1);
    expect(unreadCount(null)).toBe(0);
    expect(messageCountText(2)).toBe('2 messages from the vehicle');
  });
});
