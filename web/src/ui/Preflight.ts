import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';

export const PREFLIGHT = 'view.preflight';

export const CHECKLIST_POPUP_DELAY_MS = 1000;

export type PreflightCheck = {
  readonly name: string;
  readonly prompt: string;
  readonly verdict: string;
  readonly reason: string;
  readonly blocked: boolean;
};

export type PreflightGroup = { readonly name: string; readonly checks: readonly PreflightCheck[] };

export type Preflight = {
  readonly airframe: string;
  readonly total: number;
  readonly blocked: readonly string[];
  readonly groups: readonly PreflightGroup[];
};

export const preflight = (view: Json | null): Preflight | null => {
  const groups = optJSONArray(view, 'groups');
  if (view === null || groups === null) return null;
  return {
    airframe: optText(view, 'airframe'),
    total: optInt(view, 'total'),
    blocked: jsonStrings(optJSONArray(view, 'blocked')),
    groups: jsonObjects(groups).map((group) => ({
      name: optText(group, 'name'),
      checks: jsonObjects(optJSONArray(group, 'checks')).map((it) => ({
        name: optText(it, 'name'),
        prompt: optText(it, 'prompt'),
        verdict: optText(it, 'verdict'),
        reason: optText(it, 'reason'),
        blocked: optBoolean(it, 'blocked'),
      })),
    })),
  };
};

export const checklistOffered = (armed: boolean): string | null => (armed ? 'The checks are for before the flight' : null);

export const preflightOffered = (view: Json | null): boolean => (view === null ? true : optBoolean(view, 'offered', true));

export const checkNeedsTicking = (check: PreflightCheck): boolean => check.verdict === 'manual' || check.verdict === 'overridable';

const manualNames = (preflight: Preflight): ReadonlySet<string> =>
  new Set(preflight.groups.flatMap((it) => it.checks).filter(checkNeedsTicking).map((it) => it.name));

export const preflightSummary = (preflight: Preflight | null, ticked: ReadonlySet<string>): string => {
  if (preflight === null) return 'Connect a vehicle to run its preflight checks.';
  const checks = preflight.groups.flatMap((it) => it.checks);
  const manual = manualNames(preflight);
  const outstanding = [...manual].filter((it) => !ticked.has(it)).length;
  const warnings = checks.filter((it) => it.verdict === 'overridable').length;
  if (preflight.blocked.length > 0) {
    const left = outstanding > 0 ? ` · ${outstanding} left to check` : '';
    return `${preflight.blocked.length} of ${preflight.total} will stop the flight${left}.`;
  }
  if (outstanding > 0) return `${outstanding} of ${manual.size} left to check.`;
  if (warnings > 0) return `All ${preflight.total} checks done · ${warnings} warning${warnings === 1 ? '' : 's'}.`;
  return `All ${preflight.total} checks done.`;
};

export const groupPassed = (group: PreflightGroup, ticked: ReadonlySet<string>): boolean =>
  group.checks.every((it) => !it.blocked && (!checkNeedsTicking(it) || ticked.has(it.name)));

export const groupEnabled = (groups: readonly PreflightGroup[], index: number, ticked: ReadonlySet<string>): boolean =>
  groups.slice(0, index).every((it) => groupPassed(it, ticked));

export const groupHeading = (group: PreflightGroup, ticked: ReadonlySet<string>): string => (groupPassed(group, ticked) ? `${group.name} (passed)` : group.name);

export const GROUP_COLLAPSE_DELAY_MS = 750;

export const collapsedAfterPass = (collapsed: ReadonlySet<string>, passedBefore: ReadonlySet<string>, passedNow: ReadonlySet<string>): ReadonlySet<string> =>
  new Set([...collapsed, ...[...passedNow].filter((it) => !passedBefore.has(it))]);

export const checklistHeading = (passed: boolean): string => `Pre-flight checklist ${passed ? '(passed)' : 'in progress'}`;

export const checklistIsComplete = (preflight: Preflight | null, ticked: ReadonlySet<string>): boolean =>
  preflight !== null && preflight.blocked.length === 0 && [...manualNames(preflight)].every((it) => ticked.has(it));

export const checklistPopupIsDue = (hasVehicle: boolean, useChecklist: boolean, enforceChecklist: boolean, complete: boolean, deciding = false): boolean =>
  hasVehicle && useChecklist && enforceChecklist && !complete && !deciding;

export type CheckMark = { readonly name: 'TICKABLE' | 'PASSED' | 'ATTENTION'; readonly ordinal: number };

export const CheckMark = {
  TICKABLE: { name: 'TICKABLE', ordinal: 0 },
  PASSED: { name: 'PASSED', ordinal: 1 },
  ATTENTION: { name: 'ATTENTION', ordinal: 2 },
} as const satisfies Record<string, CheckMark>;

export const CheckMarkEntries: readonly CheckMark[] = [CheckMark.TICKABLE, CheckMark.PASSED, CheckMark.ATTENTION];

export const checkMark = (check: PreflightCheck): CheckMark =>
  checkNeedsTicking(check) ? CheckMark.TICKABLE : check.verdict === 'passing' ? CheckMark.PASSED : CheckMark.ATTENTION;

const orWhenBlank = (text: string, fallback: string): string => (text.trim() === '' ? fallback : text);

export const checkStatusText = (check: PreflightCheck, ticked: boolean): string => {
  switch (check.verdict) {
    case 'passing':
      return 'Passing';
    case 'failing':
      return orWhenBlank(check.reason, 'Failing');
    case 'overridable':
      return ticked ? 'Checked' : orWhenBlank(check.reason, 'Needs attention');
    default:
      return ticked ? 'Checked' : 'Check it';
  }
};
