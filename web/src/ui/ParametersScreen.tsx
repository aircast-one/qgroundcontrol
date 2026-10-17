import { type KeyboardEvent, useEffect, useMemo, useRef, useState } from 'react';
import { type Fact, Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { optArrayText } from '@/bridge/Json';
import { useLaunchedEffect, useProduceState } from '@/kotlin/Compose';
import { format, same } from '@/kotlin/Std';
import { DropdownMenu, DropdownMenuItem, FilterChip, OutlinedButton, Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { ParameterEditDialog } from '@/ui/ParameterLinks';
import { bitmaskSummary, factFromParameter } from '@/ui/ParameterForm';
import { ParameterToolsMenu } from '@/ui/ParameterTools';
import { RcToParamDialog } from '@/ui/RcToParamDialog';
import { enumLabel } from '@/ui/SettingsScreen';
import { SearchPill } from '@/ui/SetupRow';
import { SETUP, isPx4, parametersReady, setupReadiness } from '@/ui/SetupView';

const PARAMETER_MANAGER = 'vehicle.parameterManager';
const DEFAULT_COMPONENT = -1;

const AUTOPILOT_COMPONENT = 1;

const LAZY_MARGIN = '400px';

const notBlank = (text: string): boolean => text.trim() !== '';

const afterColon = (key: string): string => (key.includes(':') ? key.slice(key.indexOf(':') + 1) : key);

const intOrNull = (text: string): number | null => (/^[+-]?\d+$/.test(text) ? Number(text) : null);

const intAt = (value: unknown): number => {
  const number = typeof value === 'number' ? value : typeof value === 'string' && value.trim() !== '' ? Number(value) : NaN;
  return Number.isFinite(number) ? Math.trunc(number) : 0;
};

export const parameterPath = (key: string, quantity: string | null = null): string => {
  const component = intOrNull(key.includes(':') ? key.slice(0, key.indexOf(':')) : '');
  const name = component === null ? key : afterColon(key);
  return `${PARAMETER_MANAGER}.getParameter(${component ?? DEFAULT_COMPONENT},${name}${quantity === null ? '' : `,${quantity}`})`;
};

export const parameterKeys = (namesByComponent: ReadonlyMap<number, readonly string[]>): string[] =>
  [...namesByComponent].flatMap(([component, names]) => names.toSorted().map((it) => (component === AUTOPILOT_COMPONENT || component === DEFAULT_COMPONENT ? it : `${component}:${it}`)));

export const ParametersScreen = ({ className, initialSearch = '' }: { className?: string; initialSearch?: string }) => {
  const setupJson = useQgcPath(SETUP);
  const ready = useMemo(() => parametersReady(setupJson), [setupJson]);
  const px4 = useMemo(() => isPx4(setupReadiness(setupJson)), [setupJson]);
  const [search, setSearch] = useState(initialSearch);
  const [names, setNames] = useState<readonly string[]>([]);
  const [descriptions, setDescriptions] = useState<ReadonlyMap<string, readonly string[]>>(new Map());
  const [modified, setModified] = useState<ReadonlySet<string>>(new Set());
  const [modifiedChosen, setModifiedChosen] = useState(false);
  const [placement, setPlacement] = useState<ReadonlyMap<string, Placement>>(new Map());
  const [chosenCategory, setChosenCategory] = useState<string | null>(null);
  const [chosenGroup, setChosenGroup] = useState<string | null>(null);
  const [reads, setReads] = useState(0);

  useLaunchedEffect([ready, reads], async (scope) => {
    const next = ready ? await parameterNames() : [];
    if (scope.isActive()) setNames((previous) => (same(previous, next) ? previous : next));
  });

  useLaunchedEffect([names, reads], async (scope) => {
    if (names.length === 0) return;
    const summary = await parameterSummary(names);
    if (!scope.isActive()) return;
    setDescriptions(summary.descriptions);
    setModified(summary.modified);
    setPlacement(summary.placement);
  });

  const modifiedOnly = modifiedFilterOn(modifiedChosen, px4);
  const tree = useMemo(() => parameterTree(names, placement), [names, placement]);
  const browsing = search.trim() === '' && !modifiedOnly;
  const category = tree.find((it) => it.name === chosenCategory) ?? tree[0] ?? null;
  const group = category?.groups.find((it) => it === chosenGroup) ?? category?.groups[0] ?? null;
  const matches = useMemo(
    () =>
      names
        .filter((it) => parameterShown(it, descriptions.get(it) ?? [], search, modifiedOnly, modified))
        .filter((it) => !browsing || inGroup(it, placement, category?.name ?? null, group)),
    [names, descriptions, modified, search, modifiedOnly, placement, category, group, browsing],
  );

  return (
    <div className={`flex h-full w-full flex-col ${className ?? ''}`}>
      <SearchPill value={search} onValueChange={setSearch} placeholder="Search parameters" />
      {!ready ? (
        <Text className="block p-4">Waiting for parameters from the vehicle.</Text>
      ) : (
        <>
          <div className="flex w-full items-center px-4">
            <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="flex-1">
              {parameterCountLine(matches.length, modified.size)}
            </Text>
            {px4 ? (
              <div className="flex gap-2">
                <FilterChip selected={!modifiedOnly} onClick={() => setModifiedChosen(false)} label="All" />
                <FilterChip selected={modifiedOnly} onClick={() => setModifiedChosen(true)} label="Changed" />
              </div>
            ) : null}
            <ParameterToolsMenu onRefreshed={() => setReads((it) => it + 1)} />
          </div>
          {browsing && category !== null ? (
            <div className="flex w-full gap-2 px-4">
              <ChoiceButton
                shown={category.name}
                options={tree.map((it) => it.name)}
                onPick={(it) => {
                  setChosenCategory(it);
                  setChosenGroup(null);
                }}
              />
              {group !== null ? <ChoiceButton shown={group} options={category.groups} onPick={setChosenGroup} /> : null}
            </div>
          ) : null}
          <div className="min-h-0 w-full flex-1 overflow-auto">
            {matches.map((name) => (
              <LazyParameterRow key={name} name={name} offersRcToParam={px4} />
            ))}
          </div>
        </>
      )}
    </div>
  );
};

const NameRow = ({ name }: { name: string }) => (
  <Text typography="bodyMedium" className="block w-full px-4 py-5">
    {name}
  </Text>
);

const LazyParameterRow = ({ name, offersRcToParam }: { name: string; offersRcToParam: boolean }) => {
  const [shown, setShown] = useState(false);
  const holder = useRef<HTMLDivElement>(null);
  useEffect(() => {
    const node = holder.current;
    if (node === null) return undefined;
    const observer = new IntersectionObserver(([entry]) => setShown(entry?.isIntersecting ?? false), { rootMargin: LAZY_MARGIN });
    observer.observe(node);
    return () => observer.disconnect();
  }, []);
  return <div ref={holder}>{shown ? <ParameterRow name={name} offersRcToParam={offersRcToParam} /> : <NameRow name={name} />}</div>;
};

const ParameterRow = ({ name, offersRcToParam }: { name: string; offersRcToParam: boolean }) => {
  const [revision, setRevision] = useState(0);
  const [mapping, setMapping] = useState(false);
  const [forcing, setForcing] = useState(false);
  const live = useQgcPath(parameterPath(name));
  const fact = useProduceState<Fact | null>(null, [name, revision, live], async (set) => set(await parameterFact(name)));

  if (fact === null) return <NameRow name={name} />;
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.target !== event.currentTarget || (event.key !== 'Enter' && event.key !== ' ')) return;
    event.preventDefault();
    setForcing(true);
  };
  return (
    <>
      <div
        role="button"
        tabIndex={0}
        onClick={() => setForcing(true)}
        onKeyDown={onKeyDown}
        className="flex min-h-[72px] w-full cursor-pointer items-center gap-3 px-4 py-2 hover:bg-[color-mix(in_srgb,var(--md-on-surface)_8%,transparent)]"
      >
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="bodyLarge" maxLines={1} ellipsis>
            {fact.name}
          </Text>
          {notBlank(fact.description) ? (
            <Text typography="bodyMedium" color={md('onSurfaceVariant')} maxLines={1} ellipsis>
              {fact.description}
            </Text>
          ) : null}
        </div>
        <Text typography="labelMedium" color={fact.changedFromDefault ? md('primary') : md('onSurface')} maxLines={2} textAlign="end" className="max-w-[160px]">
          {parameterValueText(fact)}
        </Text>
        {offersRcToParam && !fact.readOnly && !name.includes(':') ? (
          <span className="flex shrink-0" onClick={(event) => event.stopPropagation()}>
            <TextButton onClick={() => setMapping(true)}>RC</TextButton>
          </span>
        ) : null}
      </div>
      {forcing ? (
        <ParameterEditDialog
          name={name}
          onDismiss={() => {
            setForcing(false);
            setRevision((it) => it + 1);
          }}
        />
      ) : null}
      {mapping ? <RcToParamDialog fact={fact} onDismiss={() => setMapping(false)} /> : null}
    </>
  );
};

export const modifiedFilterOn = (chosen: boolean, px4: boolean): boolean => chosen && px4;

export const parameterShown = (name: string, descriptions: readonly string[], search: string, modifiedOnly: boolean, modified: ReadonlySet<string>): boolean =>
  parameterMatches(name, descriptions, search) && (!modifiedOnly || modified.has(name));

const patternOf = (word: string): RegExp | null => {
  try {
    return new RegExp(word, 'i');
  } catch {
    return null;
  }
};

export const parameterMatches = (name: string, descriptions: readonly string[], search: string): boolean =>
  search
    .split(' ')
    .filter((it) => it !== '')
    .every((word) => {
      const pattern = patternOf(word);
      return [afterColon(name), ...descriptions].some((text) => (pattern === null ? text.toLowerCase().includes(word.toLowerCase()) : pattern.test(text)));
    });

export const parameterCountLine = (shown: number, changed: number): string =>
  [format('%,d parameter%s', shown, shown === 1 ? '' : 's'), changed > 0 ? format('%,d changed', changed) : null].filter((it): it is string => it !== null).join(' · ');

export const parameterValueText = (fact: Fact): string =>
  fact.isBitmask ? bitmaskSummary(fact) : [enumLabel(fact), fact.isEnum ? '' : fact.units].filter(notBlank).join(' ');

export const parameterSubtitle = (description: string, units: string): string => [description, units].filter(notBlank).join(' · ');

export const parameterNames = async (): Promise<string[]> => {
  const names = async (component: number): Promise<string[]> => {
    const list = await Qgc.invokeResult(`${PARAMETER_MANAGER}.parameterNames`, component);
    return Array.isArray(list) ? list.map((_, index) => optArrayText(list, index)) : [];
  };
  const listed = await Qgc.invokeResult(`${PARAMETER_MANAGER}.componentIds`);
  const components = Array.isArray(listed) ? listed.map(intAt) : [];
  const byComponent = components.length === 0
    ? new Map([[DEFAULT_COMPONENT, await names(DEFAULT_COMPONENT)]])
    : new Map(await Promise.all(components.map(async (component) => [component, await names(component)] as const)));
  return parameterKeys(byComponent);
};

export type Placement = readonly [string, string];

export type ParameterSummary = {
  descriptions: ReadonlyMap<string, readonly string[]>;
  modified: ReadonlySet<string>;
  placement: ReadonlyMap<string, Placement>;
};

export const ParameterSummary = (fields: { descriptions: ReadonlyMap<string, readonly string[]>; modified: ReadonlySet<string>; placement?: ReadonlyMap<string, Placement> }): ParameterSummary => ({
  placement: new Map(),
  ...fields,
});

export type ParameterCategory = { name: string; groups: readonly string[] };

const STANDARD_CATEGORY = 'Standard';
const DEFAULT_CATEGORY = 'Other';
const DEFAULT_GROUP = 'Misc';

const distinct = <T,>(items: readonly T[]): T[] => [...new Set(items)];

export const parameterTree = (names: readonly string[], placement: ReadonlyMap<string, Placement>): ParameterCategory[] => {
  const placed = names.flatMap((it) => {
    const at = placement.get(it);
    return at === undefined ? [] : [at];
  });
  const categories = distinct(placed.map(([category]) => category));
  const ordered = [
    ...categories.filter((it) => it === STANDARD_CATEGORY),
    ...categories.filter((it) => it !== STANDARD_CATEGORY && it !== DEFAULT_CATEGORY),
    ...categories.filter((it) => it === DEFAULT_CATEGORY),
  ];
  return ordered.map((category) => {
    const groups = distinct(placed.filter(([it]) => it === category).map(([, it]) => it));
    return { name: category, groups: [...groups.filter((it) => it !== DEFAULT_GROUP), ...groups.filter((it) => it === DEFAULT_GROUP)] };
  });
};

export const inGroup = (name: string, placement: ReadonlyMap<string, Placement>, category: string | null, group: string | null): boolean => {
  const at = placement.get(name);
  return category === null || group === null || (at !== undefined && at[0] === category && at[1] === group);
};

const parameterSummary = async (names: readonly string[]): Promise<ParameterSummary> => {
  const read = await Promise.all(names.map(async (name) => [name, await parameterFact(name)] as const));
  const facts = read.flatMap(([name, fact]) => (fact === null ? [] : [[name, fact] as const]));
  return ParameterSummary({
    descriptions: new Map(facts.map(([name, fact]) => [name, [fact.description, fact.longDescription].filter(notBlank)])),
    modified: new Set(facts.filter(([, fact]) => fact.changedFromDefault).map(([name]) => name)),
    placement: new Map(facts.map(([name, fact]) => [name, [fact.category, fact.group] as const])),
  });
};

export const parameterFact = async (name: string, quantity: string | null = null): Promise<Fact | null> =>
  factFromParameter(name, await Qgc.get(parameterPath(name, quantity)), quantity);

const ChoiceButton = ({ shown, options, onPick }: { shown: string; options: readonly string[]; onPick: (option: string) => void }) => {
  const [open, setOpen] = useState(false);
  return (
    <div className="relative">
      <OutlinedButton onClick={() => setOpen(true)}>
        <Text typography="labelLarge" maxLines={1} ellipsis>
          {shown}
        </Text>
      </OutlinedButton>
      <DropdownMenu expanded={open} onDismissRequest={() => setOpen(false)}>
        {options.map((option) => (
          <DropdownMenuItem
            key={option}
            text={option}
            onClick={() => {
              setOpen(false);
              onPick(option);
            }}
          />
        ))}
      </DropdownMenu>
    </div>
  );
};
