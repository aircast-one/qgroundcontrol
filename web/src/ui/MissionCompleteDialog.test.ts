import { describe, expect, it } from 'vitest';
import { type Json, parseJson } from '@/bridge/Json';
import { imagesTakenText, missionComplete } from './MissionCompleteDialog';

const json = (text: string): Json => parseJson(text) ?? {};

describe('MissionCompleteDialogTest', () => {
  it('an open notice carries what the dialog offers', () => {
    const notice = missionComplete(json('{"open":true,"id":3,"imagesTaken":12,"resumeFromWaypoint":4,"batteryWarning":true}'))!;
    expect(notice.id).toBe(3);
    expect(notice.resumeFromWaypoint).toBe(4);
    expect(notice.batteryWarning).toBe(true);
    expect(imagesTakenText(notice.imagesTaken)).toBe('12 Images Taken');
  });

  it('a closed notice or no photos shows nothing', () => {
    expect(missionComplete(json('{"open":false,"id":3}'))).toBeNull();
    expect(missionComplete(json('{"open":true,"id":1,"resumeFromWaypoint":null}'))!.resumeFromWaypoint).toBeNull();
    expect(imagesTakenText(0)).toBeNull();
  });

  it('a stopped mission says where it stopped and names where it resumes', () => {
    const notice = missionComplete(json('{"open":true,"id":2,"imagesTaken":0,"resumeFromWaypoint":48,"batteryWarning":true,"interrupted":true,"title":"Mission stopped at survey 2 of 3","detail":"2 drops are still to come.","resumeLabel":"Resume from the survey (2 of 3), at its waypoint 14 of 40"}'))!;
    expect([notice.title, notice.resumeLabel, notice.resumeFromWaypoint], 'the button reads the core label, which names the plan row as the title does').toEqual(['Mission stopped at survey 2 of 3', 'Resume from the survey (2 of 3), at its waypoint 14 of 40', 48]);
    expect(missionComplete(json('{"open":true,"id":3,"imagesTaken":0,"resumeFromWaypoint":null,"batteryWarning":false}'))!.title).toBe('Flight plan complete');
  });
});
