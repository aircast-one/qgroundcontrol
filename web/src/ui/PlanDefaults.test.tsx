// @vitest-environment jsdom
import { cleanup, fireEvent, render, screen } from '@testing-library/react';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
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

afterEach(() => {
  cleanup();
  vi.restoreAllMocks();
});

describe('PlanDefaultsDialog', () => {
  it('the estimate-only note sits under the mission flight speed, above the speeds it is about', () => {
    render(<PlanDefaultsDialog view={view} onDismiss={() => undefined} />);
    const note = screen.getByText(NOTE);
    const flightSpeed = screen.getByRole('switch', { name: 'Flight speed' });
    const hover = screen.getByText('Planning hover speed');
    expect(flightSpeed.compareDocumentPosition(note) & Node.DOCUMENT_POSITION_FOLLOWING, 'the Flight speed switch adds a speed command to the mission, so the note must not claim it').toBeTruthy();
    expect(note.compareDocumentPosition(hover) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('names each default as Settings › General › Planning defaults does', () => {
    render(<PlanDefaultsDialog view={view} onDismiss={() => undefined} />);
    expect(screen.getByText('Default waypoint altitude')).toBeTruthy();
    expect(screen.getByText('Planning hover speed')).toBeTruthy();
    expect(screen.queryByText('Default mission altitude')).toBeNull();
    expect(screen.queryByText('Offline editing hover speed')).toBeNull();
  });

  it('Enter in a typed mission flight speed sets it like the Set button', async () => {
    const written = vi.spyOn(Qgc, 'writeRefusal').mockResolvedValue(null);
    const specified = parseJson(JSON.stringify({ ...view, defaults: { ...(view as { defaults: object }).defaults, flightSpeed: { available: true, specified: true, value: 5.0, units: 'm/s', path: 'plan.speed', specifyPath: 'plan.specify' } } }));
    render(<PlanDefaultsDialog view={specified} onDismiss={() => undefined} />);
    const typed = screen.getByRole('textbox', { name: 'Speed m/s' });
    fireEvent.change(typed, { target: { value: '6' } });
    fireEvent.keyDown(typed, { key: 'Enter' });
    await vi.waitFor(() => expect(written).toHaveBeenCalledWith('plan.speed', 6));
  });
});
