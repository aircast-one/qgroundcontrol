import { describe, expect, it } from 'vitest';
import { parseJson } from '@/bridge/Json';
import { NO_ACTIONS_FILE, chosenFile, mavlinkActions } from './MavlinkActions';

describe('MavlinkActionsTest', () => {
  it('the_files_and_fly_view_actions_read_from_the_core', () => {
    expect(mavlinkActions(parseJson('{}'))).toBeNull();
    const read = mavlinkActions(parseJson('{"files":["fly.json"],"flyViewFile":"fly.json","joystickFile":"","flyViewPath":"a","joystickPath":"b","actions":[{"label":"Lights","description":"toggle"}]}'));
    expect(read?.actions).toEqual([{ label: 'Lights', description: 'toggle' }]);
    expect(chosenFile(NO_ACTIONS_FILE)).toBe('');
    expect(chosenFile('fly.json')).toBe('fly.json');
  });
});
