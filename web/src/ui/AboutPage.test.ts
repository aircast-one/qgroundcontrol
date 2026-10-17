import { describe, expect, it } from 'vitest';
import releaseWorkflow from '../../../.github/workflows/aircast-release.yml?raw';
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

describe('ReleaseVersionTest', () => {
  it('the web build a release ships is named after the release tag, not dev', () => {
    const webJob = releaseWorkflow.slice(releaseWorkflow.indexOf('\n  build-web:'));
    const buildStep = webJob.slice(webJob.indexOf('- name: Build web'), webJob.indexOf('npm run build'));
    expect(buildStep).toContain('RELEASE_TAG: ${{ needs.setup.outputs.version }}');
    expect(buildStep).toContain('export VITE_APP_VERSION="${RELEASE_TAG#aircast-v}"');
  });
});
