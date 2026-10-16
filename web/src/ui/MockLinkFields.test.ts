import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { LinkType, addableLinkTypes } from './LinksScreen';
import { MOCK_VEHICLES, MOCK_VIDEO_STREAMS, MockLinkChoices, mockLinkArguments, withMockVehicle } from './MockLinkFields';
import { settingsPages } from './SettingsScreen';

const json = (text: string): Json => parseJson(text) ?? {};

describe('MockLinkFieldsTest', () => {
  it('mock links are started from Add link, not a settings page of their own', () => {
    const view = json('{"pages":[{"title":"Mock Link","sections":[]}]}');
    expect(settingsPages(view)).toHaveLength(0);
    expect(addableLinkTypes(json('{"linkTypeIds":["serial","udp","tcp","mock"]}'))).toEqual([LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Mock]);
  });

  it('start sends the vehicle key and options in the order the core reads them', () => {
    const choices = MockLinkChoices({ sendStatusText: true, camera: true, proximity: true, freshParams: true, vehicle: 1, videoStream: 3 });
    expect(mockLinkArguments(choices)).toEqual(['apmCopter', true, true, false, true, true, 3]);
  });

  it('fresh firmware parameters only reach an ArduPilot vehicle', () => {
    const px4 = MockLinkChoices({ freshParams: true, vehicle: 0 });
    expect(mockLinkArguments(px4)[5]).toBe(false);
    expect(withMockVehicle(MockLinkChoices({ freshParams: true, vehicle: 1 }), 0).freshParams).toBe(false);
    expect(MOCK_VEHICLES.map((it) => it[0])).toEqual(['px4', 'apmCopter', 'apmPlane', 'apmSub', 'apmRover', 'generic']);
    expect(MOCK_VIDEO_STREAMS).toHaveLength(6);
  });
});
