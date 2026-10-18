import { useMemo } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { Text, TextButton } from '@/m3';
import { md } from '@/theme/Colors';
import { LENS_FORGET, LENS_MEASURE, LENS_VIEW, lensReading, measuredText } from '@/ui/Lens';
import { SectionHeader } from '@/ui/SetupRow';

export const LensSection = ({ className }: { className?: string }) => {
  const json = useQgcPath(LENS_VIEW);
  const lens = useMemo(() => lensReading(json), [json]);
  if (lens === null) return null;
  const measured = measuredText(lens.measuredZooms);
  return (
    <div className={`flex flex-col ${className ?? ''}`}>
      <SectionHeader text="Lens" />
      <Text typography="bodyMedium" className="px-4 py-1">
        {lens.summary}
      </Text>
      {measured !== null ? (
        <Text typography="bodySmall" className="px-4">
          {measured}
        </Text>
      ) : null}
      {lens.measuring ? (
        <Text typography="bodySmall" className="px-4 py-1">
          {lens.progress}
        </Text>
      ) : null}
      {lens.error !== null ? (
        <Text typography="bodySmall" color={md('error')} className="px-4 py-1">
          {lens.error}
        </Text>
      ) : null}
      <div className="px-1">
        <TextButton enabled={lens.canMeasure} onClick={() => void Qgc.invoke(LENS_MEASURE)}>
          {lens.measuring ? 'Measuring…' : 'Measure the lens'}
        </TextButton>
      </div>
      {!lens.measuring && lens.reason !== null ? (
        <Text typography="bodySmall" color={md('onSurfaceVariant')} className="px-4">
          {lens.reason}
        </Text>
      ) : null}
      {lens.measuredZooms.length > 0 && !lens.measuring ? (
        <div className="px-1">
          <TextButton onClick={() => void Qgc.invoke(LENS_FORGET)}>Forget measurements</TextButton>
        </div>
      ) : null}
    </div>
  );
};
