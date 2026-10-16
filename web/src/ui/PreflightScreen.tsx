import { useMemo, useRef, useState } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { Checkbox, HorizontalDivider, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';
import {
  CheckMark,
  GROUP_COLLAPSE_DELAY_MS,
  PREFLIGHT,
  type Preflight,
  checkMark,
  checkStatusText,
  checklistHeading,
  checklistIsComplete,
  collapsedAfterPass,
  groupEnabled,
  groupHeading,
  groupPassed,
  preflight,
  preflightSummary,
} from './Preflight';

const EMPTY: ReadonlySet<string> = new Set();

const toggled = (set: ReadonlySet<string>, name: string, on: boolean): ReadonlySet<string> => (on ? new Set([...set, name]) : new Set([...set].filter((it) => it !== name)));

export const PreflightScreen = ({ className, ticked = EMPTY, onTicked = () => {} }: { className?: string; ticked?: ReadonlySet<string>; onTicked?: (ticked: ReadonlySet<string>) => void }) => {
  const json = useQgcPath(PREFLIGHT);
  const checks = useMemo(() => preflight(json), [json]);

  if (checks === null || checks.groups.length === 0) {
    return (
      <Text className={`block w-full p-4 ${className ?? ''}`}>Connect a vehicle to run its preflight checks.</Text>
    );
  }
  return <PreflightChecks checks={checks} className={className} ticked={ticked} onTicked={onTicked} />;
};

const PreflightChecks = ({ checks, className, ticked, onTicked }: { checks: Preflight; className?: string; ticked: ReadonlySet<string>; onTicked: (ticked: ReadonlySet<string>) => void }) => {
  const [collapsed, setCollapsed] = useState<ReadonlySet<string>>(EMPTY);
  const passedNames = checks.groups.filter((it) => groupPassed(it, ticked)).map((it) => it.name);
  const passedKey = passedNames.join('\n');
  const passedGroups = useMemo<ReadonlySet<string>>(() => new Set(passedNames), [passedKey]);
  const passedBefore = useRef<ReadonlySet<string>>(EMPTY);
  useLaunchedEffect([passedGroups], async (scope) => {
    await delay(GROUP_COLLAPSE_DELAY_MS);
    if (!scope.isActive()) return;
    setCollapsed((current) => collapsedAfterPass(current, passedBefore.current, passedGroups));
    passedBefore.current = passedGroups;
  });

  return (
    <div className={`flex size-full min-h-0 flex-col ${className ?? ''}`}>
      <div className="flex w-full items-center pr-2 pl-4">
        <Text typography="titleMedium" className="min-w-0 flex-1">
          {checklistHeading(checklistIsComplete(checks, ticked))}
        </Text>
        <TextButton
          onClick={() => {
            onTicked(EMPTY);
            setCollapsed(EMPTY);
          }}
          enabled={ticked.size > 0}
        >
          Reset
        </TextButton>
      </div>
      <Text typography="titleSmall" color={checks.blocked.length === 0 ? md('onSurface') : md('error')} className="px-4 py-2">
        {preflightSummary(checks, ticked)}
      </Text>
      {checks.airframe.trim() !== '' ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-4">
          {checks.airframe}
        </Text>
      ) : null}
      <div className="pt-2">
        <HorizontalDivider />
      </div>
      <div className="min-h-0 flex-1 overflow-auto">
        {checks.groups.map((group, groupIndex) => {
          const open = groupEnabled(checks.groups, groupIndex, ticked);
          const folded = collapsed.has(group.name);
          return (
            <div key={`group:${group.name}`} className="flex flex-col">
              <button
                type="button"
                aria-expanded={!folded}
                onClick={() => setCollapsed(toggled(collapsed, group.name, !folded))}
                className="w-full pt-4 pb-1 pl-4 text-left"
              >
                <Text typography="labelLarge" color={open ? md('primary') : md('onSurfaceVariant')}>
                  {groupHeading(group, ticked)}
                </Text>
              </button>
              {folded
                ? null
                : group.checks.map((check) => {
                    const isTicked = ticked.has(check.name);
                    const mark = checkMark(check);
                    return (
                      <div key={`${group.name}:${check.name}`} className="flex w-full items-center gap-1 px-3 py-1.5">
                        {mark === CheckMark.TICKABLE ? (
                          <div className="flex size-12 shrink-0 items-center justify-center">
                            <Checkbox checked={isTicked} enabled={open} onCheckedChange={(on) => onTicked(toggled(ticked, check.name, on))} label={check.prompt.trim() === '' ? check.name : check.prompt} />
                          </div>
                        ) : mark === CheckMark.PASSED ? (
                          <div className="flex size-12 shrink-0 items-center justify-center">
                            <MaterialIcon name="check" filled label="Passing" style={{ color: md('onSurfaceVariant') }} />
                          </div>
                        ) : (
                          <div className="flex size-12 shrink-0 items-center justify-center">
                            <MaterialIcon
                              name="warning"
                              filled
                              label={check.blocked ? 'Will stop the flight' : 'Needs attention'}
                              style={{ color: check.blocked ? md('error') : md('onSurfaceVariant') }}
                            />
                          </div>
                        )}
                        <div className="flex min-w-0 flex-1 flex-col">
                          <Text typography="bodyMedium">{check.prompt.trim() === '' ? check.name : check.prompt}</Text>
                          <Text typography="labelSmall" color={check.blocked ? md('error') : md('onSurfaceVariant')}>
                            {checkStatusText(check, isTicked)}
                          </Text>
                        </div>
                      </div>
                    );
                  })}
            </div>
          );
        })}
      </div>
    </div>
  );
};
