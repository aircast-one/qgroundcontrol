import { useMemo } from 'react';
import { type Json, jsonObjects, optJSONArray, optString } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { md } from '@/theme/Colors';
import { SectionHeader } from './SetupRow';

export const LINK_STATUS_VIEW = 'view.linkStatus';

export const linkStatusRows = (view: Json | null): readonly (readonly [string, string])[] =>
  jsonObjects(optJSONArray(view, 'rows')).map((it) => [optString(it, 'label'), optString(it, 'value')] as const);

export const LinkStatusSection = () => {
  const json = useQgcPath(LINK_STATUS_VIEW);
  const rows = useMemo(() => linkStatusRows(json), [json]);
  return (
    <>
      <SectionHeader text="Link Status (Current Vehicle)" />
      {rows.length === 0 ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} className="block px-4">
          Not connected
        </Text>
      ) : null}
      {rows.map(([label, value], index) => (
        <div key={index} className="flex w-full items-center px-4 py-1.5">
          <Text className="flex-1">{label}</Text>
          <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
            {value}
          </Text>
        </div>
      ))}
    </>
  );
};
