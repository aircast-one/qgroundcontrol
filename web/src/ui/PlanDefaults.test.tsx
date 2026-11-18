// @vitest-environment jsdom
import { cleanup, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { PlanDefaultsDialog } from './PlanDefaults';

const NOTE = 'The speeds below only estimate the mission time. They do not change how fast the aircraft flies.';

const control = (name: string, label: string, value: number, units: string) =>
  `{"kind":"object","class":"Control","control":"number","name":"${name}","label":"${label}","path":"settings.appSettings.${name}","value":${value},"valueString":"${value}","units":"${units}","enabled":true,"readOnly":false,"options":[],"bits":[]}`;

const view = parseJson(
  `{"kind":"object","class":"Plan","defaults":{"altitude":${control('defaultMissionItemAltitude', 'Default mission altitude', 50.0, 'm')},
   "flightSpeed":{"available":true,"specified":false,"value":5.0,"units":"m/s","path":"plan.missionController.visualItems.0.speedSection.flightSpeed","specifyPath":"plan.missionController.visualItems.0.speedSection.specifyFlightSpeed"},
   "hover":${control('offlineEditingHoverSpeed', 'Offline editing hover speed', 5.0, 'm/s')},
   "speedNote":"${NOTE}"}}`,
);

beforeEach(() => {
  Object.defineProperty(window, 'matchMedia', { configurable: true, value: () => ({ matches: false, addEventListener: () => undefined, removeEventListener: () => undefined }) });
});

afterEach(cleanup);

describe('PlanDefaultsDialog', () => {
  it('the estimate-only note sits under the mission flight speed, above the speeds it is about', () => {
    render(<PlanDefaultsDialog view={view} onDismiss={() => undefined} />);
    const note = screen.getByText(NOTE);
    const flightSpeed = screen.getByRole('switch', { name: 'Flight speed' });
    const hover = screen.getByText('Offline editing hover speed');
    expect(flightSpeed.compareDocumentPosition(note) & Node.DOCUMENT_POSITION_FOLLOWING, 'the Flight speed switch adds a speed command to the mission, so the note must not claim it').toBeTruthy();
    expect(note.compareDocumentPosition(hover) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });
});
