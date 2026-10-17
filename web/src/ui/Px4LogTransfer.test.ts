import { describe, expect, it } from 'vitest';
import { type Json, optText, parseJson } from '@/bridge/Json';
import { LogFile, type MavlinkLog, WIND_SPEEDS, logListIdle, mavlinkLog, unsavedTexts, uploadEmail } from './Px4LogTransfer';

const read = (text: string): MavlinkLog => {
  const log = mavlinkLog(parseJson(text));
  if (log === null) throw new Error('the view did not parse');
  return log;
};

const settings = (text: string): Json => parseJson(text) ?? {};

describe('Px4LogTransferTest', () => {
  it('the page reads the logging state, settings and saved files', () => {
    const log = read(
      '{"vehiclePx4":true,"logRunning":false,"canStartLog":true,"persistence":true,"settings":{"emailAddress":"a@b.c","windSpeed":"5"},"files":[{"name":"001-x.ulg","size":2048,"uploaded":true}],"uploading":false,"uploadingFile":null,"message":null}',
    );
    expect(log.canStart).toBe(true);
    expect(log.files).toEqual([LogFile({ name: '001-x.ulg', size: 2048, uploaded: true })]);
    expect(optText(log.settings, 'emailAddress')).toBe('a@b.c');
    expect(WIND_SPEEDS.find(([, raw]) => raw === optText(log.settings, 'windSpeed'))?.[0]).toBe('Breeze');
    expect(log.uploadProgress).toBe(0);
  });

  it("the upload progress is the core's sent fraction", () => {
    expect(read('{"uploading":true,"uploadingFile":"001-x","uploadProgress":0.4}').uploadProgress).toBe(0.4);
  });

  it('the saved file buttons wait while logging or uploading like PX4LogTransferSettings _idle', () => {
    const idle = (running: boolean, uploading: boolean) => logListIdle(read(`{"logRunning":${running},"uploading":${uploading}}`));
    expect(idle(false, false)).toBe(true);
    expect(idle(true, false)).toBe(false);
    expect(idle(false, true)).toBe(false);
  });

  it('upload uses the email typed but not yet saved and saves it first like saveItems', () => {
    const saved = settings('{"emailAddress":"","description":"d"}');
    const drafts = new Map([
      ['emailAddress', 'a@b.c'],
      ['description', 'd'],
    ]);
    expect(uploadEmail(saved, drafts)).toBe('a@b.c');
    expect(unsavedTexts(saved, drafts)).toEqual([['emailAddress', 'a@b.c']]);
  });

  it('upload falls back to the saved email when nothing was typed', () => {
    const saved = settings('{"emailAddress":"a@b.c"}');
    expect(uploadEmail(saved, new Map())).toBe('a@b.c');
    expect(unsavedTexts(saved, new Map())).toEqual([]);
  });
});
