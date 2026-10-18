import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import {
  BETA_WARNING,
  DEFAULT_FIRMWARE_SOURCE,
  DEV_WARNING,
  FIRMWARE_FROM_FILE,
  DEVICE_PORT,
  FIRMWARE_SOURCES,
  FirmwareJob,
  FirmwarePort,
  bootloaderOffered,
  cancelledOnLeave,
  firmwareChoice,
  firmwareFileAccepted,
  firmwareHeading,
  firmwareJob,
  firmwarePhaseText,
  firmwarePorts,
  firmwareSources,
  firmwareWarning,
  flashingLabel,
  preselectedPort,
  rememberedSource,
  sourceAfterAdvanced,
  sourceLabel,
  sourceSettings,
} from './FirmwareScreen';

describe('FirmwareScreenTest', () => {
  it('theCoresFirmwareViewsReadAsTheScreenShowsThem', () => {
    const ports = parseJson('{"kind":"object","class":"FirmwarePorts","ports":[{"port":"/dev/bus/usb/001/002","description":"PX4 BL FMU v5.x","bootloader":true},{"port":"","description":"x","bootloader":false}]}');
    expect(firmwarePorts(ports)).toEqual([FirmwarePort({ port: '/dev/bus/usb/001/002', description: 'PX4 BL FMU v5.x', bootloader: true })]);
    const job = parseJson('{"kind":"object","class":"FirmwareUpgrade","phase":"programming","busy":true,"cancellable":false,"progress":0.25,"messages":["Erasing previous program...","Erase complete"],"error":null}');
    expect(firmwareJob(job)).toEqual(FirmwareJob({ phase: 'programming', busy: true, cancellable: false, progress: 0.25, messages: ['Erasing previous program...', 'Erase complete'], error: '' }));
    expect(firmwareJob(parseJson('{"kind":"null"}'))).toBeNull();
    expect(firmwarePhaseText('connecting')).toBe('Waiting for the bootloader');
    expect(firmwarePhaseText('idle')).toBe('');
    const choosing = parseJson('{"class":"FirmwareUpgrade","phase":"choosing","busy":true,"cancellable":true,"choices":[{"name":"CUAVv5 - 4.6.2","url":"https://f/c.apj"}]}');
    expect(firmwareJob(choosing)?.choices).toEqual([['CUAVv5 - 4.6.2', 'https://f/c.apj']]);
    expect(firmwarePhaseText('choosing')).toBe('Choose the firmware build');
  });

  it('aReleaseIsSentAsTheCoresTokenAndAFileAsItsPath', () => {
    expect(firmwareChoice('ardupilot:heli:dev', null)).toBe('ardupilot:heli:dev');
    expect(firmwareChoice(FIRMWARE_FROM_FILE, '/cache/firmware-fw.px4')).toBe('/cache/firmware-fw.px4');
    expect(firmwareChoice(FIRMWARE_FROM_FILE, null)).toBeNull();
    expect(FIRMWARE_SOURCES.some(([token, label]) => token === 'ardupilot:plane:stable' && label === 'ArduPilot Plane, stable')).toBe(true);
  });

  it('onlyImagesTheBootloaderTakesAreAccepted', () => {
    expect(firmwareFileAccepted('px4_fmu-v5_default.px4')).toBe(true);
    expect(firmwareFileAccepted('arducopter.APJ')).toBe(true);
    expect(firmwareFileAccepted('fw.bin')).toBe(true);
    expect(firmwareFileAccepted('notes.txt')).toBe(false);
  });

  it("beta and developer builds carry QGC's warnings, like FirmwareUpgrade", () => {
    expect(firmwareWarning('px4:beta')).toBe(BETA_WARNING);
    expect(firmwareWarning('ardupilot:copter:dev')).toBe(DEV_WARNING);
    expect(firmwareWarning('px4:stable')).toBeNull();
  });

  it('without advanced settings only the standard builds are offered, as FirmwareUpgrade shows', () => {
    const plain = firmwareSources(false).map(([token]) => token);
    expect(plain.every((it) => it.endsWith(':stable'))).toBe(true);
    expect(plain).toContain('px4:stable');
    expect(plain).not.toContain(FIRMWARE_FROM_FILE);
    expect(firmwareSources(true).map(([token]) => token)).toContain(FIRMWARE_FROM_FILE);
    expect(firmwareSources(true).map(([token]) => token)).toContain('px4:dev');
    expect(firmwareWarning('px4:dev')).toBe(DEV_WARNING);
    expect(sourceAfterAdvanced('ardupilot:plane:dev', false)).toBe('ardupilot:plane:stable');
    expect(sourceAfterAdvanced(FIRMWARE_FROM_FILE, false)).toBe(DEFAULT_FIRMWARE_SOURCE);
    expect(sourceAfterAdvanced('px4:beta', true)).toBe('px4:beta');
  });

  it('the bootloader button needs advanced settings and an ArduPilot vehicle', () => {
    expect(bootloaderOffered(true, true)).toBe(true);
    expect(bootloaderOffered(false, true)).toBe(false);
    expect(bootloaderOffered(true, false)).toBe(false);
  });

  it('the last stack and ArduPilot vehicle are remembered like FirmwareUpgrade qml', () => {
    expect(rememberedSource(12, 2)).toBe('px4:stable');
    expect(rememberedSource(3, 2)).toBe('ardupilot:plane:stable');
    expect(rememberedSource(3, null)).toBe('ardupilot:copter:stable');
    expect(sourceSettings('ardupilot:sub:beta')).toEqual([
      ['defaultFirmwareType', 3],
      ['apmVehicleType', 4],
    ]);
    expect(sourceSettings('px4:dev')).toEqual([['defaultFirmwareType', 12]]);
    expect(sourceSettings('sik:stable')).toEqual([]);
  });

  it('flashing names the board like FirmwareUpgrade qml', () => {
    expect(flashingLabel([FirmwarePort({ port: '/dev/bus/usb/1', description: 'Pixhawk 6C', bootloader: true })], '/dev/bus/usb/1')).toBe('Flashing - Pixhawk 6C');
    expect(flashingLabel([], '/dev/x')).toBe('Flashing - /dev/x');
  });

  it('a recognised selection is kept, else the first Pixhawk, then a SiK radio, like _preselectIndex', () => {
    const radio = FirmwarePort({ port: 'r', description: 'SiK', bootloader: false, boardType: 'SiK Radio' });
    const fmu = FirmwarePort({ port: 'p', description: 'Pixhawk 6C', bootloader: false, boardType: 'Pixhawk' });
    const other = FirmwarePort({ port: 'o', description: 'FTDI', bootloader: false });
    expect(preselectedPort([other, radio, fmu], '')).toBe('p');
    expect(preselectedPort([other, radio], 'o')).toBe('r');
    expect(preselectedPort([radio, fmu], 'r')).toBe('r');
    expect(preselectedPort([other], '')).toBeNull();
  });

  it('PX4 builds carry the release name like updatePX4VersionDisplay', () => {
    expect(sourceLabel('px4:stable', 'PX4 Pro, stable', 'v1.17.0 - Stable Release', '')).toBe('PX4 Pro v1.17.0 - Stable Release');
    expect(sourceLabel('px4:beta', 'PX4 Pro, beta', 'v1.15.4', '')).toBe('PX4 Pro, beta');
    expect(sourceLabel('ardupilot:copter:stable', 'ArduPilot Copter, stable', 'v1.15.4', 'x')).toBe('ArduPilot Copter, stable');
  });

  it('leaving the page drops a board search but never interrupts a write', () => {
    expect(['connecting', 'choosing', 'erasing', 'programming', 'verifying', 'idle'].map(cancelledOnLeave)).toEqual([true, true, false, false, false, false]);
    expect(cancelledOnLeave(null)).toBe(false);
  });
  it('a page served by an Aircast device offers the flight controller on it before any USB board', () => {
    const usb = { port: 'webserial-0', bootloader: false, description: 'Pixhawk', boardType: 'Pixhawk' };
    expect(firmwareHeading([])).toBe('No USB serial devices attached');
    expect(firmwareHeading([usb])).toBe('Flash a board over USB');
    expect(firmwareHeading([usb, { ...usb, port: DEVICE_PORT, description: 'Flight controller on the Aircast device' }])).toBe('Flash the flight controller on this Aircast device, or a board over USB');
  });
});
