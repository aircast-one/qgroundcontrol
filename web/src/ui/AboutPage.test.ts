import { describe, expect, it } from 'vitest';
import { PRIVACY_POLICY_LINK, aboutLinks, qgcVersion } from './AboutPage';

describe('QgcVersionTest', () => {
  it('versionCarriesTheBuildAbiBitnessLikeQgcVersion', () => {
    expect(qgcVersion('5.0.1', true)).toBe('5.0.1 64 bit');
    expect(qgcVersion('5.0.1', false)).toBe('5.0.1 32 bit');
  });
});

describe('SettingsViewTest', () => {
  it('about lists the privacy policy after the support links', () => {
    const guide = { name: 'QGroundControl User Guide', url: 'https://docs.qgroundcontrol.com', host: 'docs.qgroundcontrol.com' };
    expect(aboutLinks([guide])).toEqual([guide, PRIVACY_POLICY_LINK]);
    expect(aboutLinks([]).map((it) => it.url)).toEqual(['https://aircast.one/privacy/android']);
  });
});
