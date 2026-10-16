import { useState } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { type Json, jsonObjects, jsonStrings, optBoolean, optInt, optJSONArray, optText } from '@/bridge/Json';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { same } from '@/kotlin/Std';
import { AlertDialog, IconButton, OutlinedButton, OutlinedTextField, Switch, Text, TextButton } from '@/m3';
import { MaterialIcon } from '@/res/Icon';
import { md } from '@/theme/Colors';

export const LOG_CATEGORIES_VIEW = 'view.logCategories';
export const LOG_SET_CATEGORY = 'appLog.setCategory';
export const LOG_RESET_CATEGORIES = 'appLog.resetCategories';
const INDENT_PER_DEPTH = 12;

export type LogCategory = { name: string; shortName: string; depth: number; enabled: boolean };

export type LogCategories = { active: readonly string[]; categories: readonly LogCategory[] };

export const logCategories = (view: Json | null): LogCategories | null =>
  view === null
    ? null
    : {
        active: jsonStrings(optJSONArray(view, 'active')),
        categories: jsonObjects(optJSONArray(view, 'categories')).map((category) => ({
          name: optText(category, 'name'),
          shortName: optText(category, 'shortName'),
          depth: optInt(category, 'depth'),
          enabled: optBoolean(category, 'enabled'),
        })),
      };

export const filteredCategories = (categories: readonly LogCategory[], search: string): readonly LogCategory[] =>
  categories.filter((category) => category.name.toLowerCase().includes(search.toLowerCase()));

const NAME_CHARACTER = /[\p{L}\p{N}_]/u;

export const parentOf = (categories: readonly LogCategory[], category: LogCategory): LogCategory | null =>
  categories.find(
    (it) =>
      it.depth === category.depth - 1 &&
      category.name.startsWith(it.name) &&
      category.name.endsWith(category.shortName) &&
      category.name.length > it.name.length + category.shortName.length &&
      ![...category.name.slice(it.name.length, category.name.length - category.shortName.length)].some((character) => NAME_CHARACTER.test(character)),
  ) ?? null;

export const parentNames = (categories: readonly LogCategory[]): ReadonlySet<string> =>
  new Set(
    categories.flatMap((category) => {
      const parent = parentOf(categories, category);
      return parent === null ? [] : [parent.name];
    }),
  );

export const shownInTree = (categories: readonly LogCategory[], expanded: ReadonlySet<string>): readonly LogCategory[] => {
  const childrenOf = (parent: LogCategory | null): readonly LogCategory[] =>
    categories.filter((category) => {
      const found = parentOf(categories, category);
      return parent === null ? found === null : found !== null && same(found, parent);
    });
  const walk = (category: LogCategory): readonly LogCategory[] => [category, ...(expanded.has(category.name) ? childrenOf(category).flatMap(walk) : [])];
  return childrenOf(null).flatMap(walk);
};

export const LoggingCategoriesDialog = ({ onDismiss }: { onDismiss: () => void }) => {
  const [read, setRead] = useState<LogCategories | null>(null);
  const [revision, setRevision] = useState(0);
  const [search, setSearch] = useState('');
  const [expanded, setExpanded] = useState<ReadonlySet<string>>(new Set());

  useLaunchedEffect([revision], async (scope) => {
    const categories = logCategories(await Qgc.get(LOG_CATEGORIES_VIEW));
    if (scope.isActive()) setRead(categories);
  });

  const act = (path: string, ...args: unknown[]) =>
    void (async () => {
      await Qgc.invoke(path, ...args);
      setRevision((it) => it + 1);
    })();

  const categories = read?.categories ?? [];
  const found = filteredCategories(categories, search);
  const parents = parentNames(categories);

  return (
    <AlertDialog
      onDismissRequest={onDismiss}
      title="Logging categories"
      text={
        <div className="flex max-h-[520px] flex-col gap-2 overflow-y-auto">
          <div className="flex items-center gap-2">
            <OutlinedTextField value={search} onValueChange={setSearch} placeholder="Filter categories…" className="flex-1" />
            <TextButton onClick={() => setSearch('')}>Clear</TextButton>
          </div>
          <Text typography="titleSmall" className="block">
            Active categories
          </Text>
          {(read?.active ?? []).map((name) => (
            <CategorySwitch key={name} label={name} checked depth={0} onChange={() => act(LOG_SET_CATEGORY, name, false)} />
          ))}
          <OutlinedButton onClick={() => act(LOG_RESET_CATEGORIES)} className="self-start">
            Reset all
          </OutlinedButton>
          {search === '' ? (
            <>
              <Text typography="titleSmall" className="block">
                Categories
              </Text>
              {shownInTree(categories, expanded).map((category) => {
                const opened = expanded.has(category.name);
                return (
                  <CategorySwitch
                    key={category.name}
                    label={category.shortName}
                    checked={category.enabled}
                    depth={category.depth}
                    expander={parents.has(category.name) ? opened : null}
                    onExpand={() => setExpanded(opened ? new Set([...expanded].filter((name) => name !== category.name)) : new Set([...expanded, category.name]))}
                    onChange={(checked) => act(LOG_SET_CATEGORY, category.name, checked)}
                  />
                );
              })}
            </>
          ) : (
            <>
              <Text typography="titleSmall" className="block">
                Search results
              </Text>
              {found.map((category) => (
                <CategorySwitch key={category.name} label={category.name} checked={category.enabled} depth={0} onChange={(checked) => act(LOG_SET_CATEGORY, category.name, checked)} />
              ))}
              {found.length === 0 ? <Text color={md('onSurfaceVariant')}>No matching categories</Text> : null}
            </>
          )}
        </div>
      }
      confirmButton={<TextButton onClick={onDismiss}>Close</TextButton>}
    />
  );
};

const CategorySwitch = ({
  label,
  checked,
  depth,
  expander = null,
  onExpand = () => undefined,
  onChange,
}: {
  label: string;
  checked: boolean;
  depth: number;
  expander?: boolean | null;
  onExpand?: () => void;
  onChange: (checked: boolean) => void;
}) => (
  <div className="flex w-full items-center" style={{ paddingLeft: depth * INDENT_PER_DEPTH }}>
    {expander !== null ? (
      <IconButton onClick={onExpand} className="size-8" label={expander ? 'Collapse' : 'Expand'}>
        <MaterialIcon name={expander ? 'keyboard_arrow_down' : 'keyboard_arrow_right'} filled />
      </IconButton>
    ) : null}
    <Text className="min-w-0 flex-1 break-words">
      {label}
    </Text>
    <Switch checked={checked} onCheckedChange={onChange} label={label} />
  </div>
);
