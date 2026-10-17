import { useState } from 'react';
import { settingControl, useQgcString } from '@/bridge/QgcState';
import { useLaunchedEffect, useProduceState } from '@/kotlin/Compose';
import { openDocument } from '@/kotlin/Platform';
import { useValue } from '@/kotlin/State';
import { Button, Checkbox, LinearProgressIndicator, OutlinedTextField, Surface, Text, TextButton } from '@/m3';
import { argb, md, withAlpha } from '@/theme/Colors';
import { GeoTagRun, geoTagOutputText, parsedOffset, shownOffset } from '@/ui/GeoTagRun';
import { type DownloadedLog, downloadedLogs, openDocumentTree, treeName } from '@/ui/GeoTagStaging';
import { GEOTAG_TAGGED, type GeoTagImage, geoTagButton, geoTagCoordinate, geoTagError, geoTagImageText, geoTagStep, geoTagSummary } from '@/ui/GeoTagView';

const GEOTAG_BLUE = argb(0xff2196f3);
const GEOTAG_GREEN = argb(0xff00c853);
const GEOTAG_ORANGE = argb(0xffff9800);

export const GeoTagScreen = ({ className }: { className?: string }) => {
  const logSavePath = useQgcString(settingControl('settings.appSettings.logSavePath'));
  const state = useValue(GeoTagRun.state);
  const note = useValue(GeoTagRun.note);
  const busy = useValue(GeoTagRun.busy);
  const imageTree = useValue(GeoTagRun.imageTree);
  const outputTree = useValue(GeoTagRun.outputTree);
  const [offsetText, setOffsetText] = useState<string | null>(null);
  const logs = useProduceState<readonly DownloadedLog[]>([], [logSavePath], async (set) => set(await downloadedLogs(logSavePath)));

  useLaunchedEffect([], () => GeoTagRun.refresh());

  const pickLog = async () => {
    const chosen = await openDocument(['*/*']);
    if (chosen !== null) GeoTagRun.pickLog(chosen);
  };
  const pickImages = async () => {
    const tree = await openDocumentTree();
    if (tree !== null) GeoTagRun.pickImages(tree);
  };
  const pickOutput = async () => {
    const tree = await openDocumentTree();
    if (tree !== null) GeoTagRun.pickOutput(tree);
  };

  const current = state;
  if (current === null) return null;
  const editable = !current.inProgress && !busy;
  return (
    <div className={`flex w-full flex-col gap-3 overflow-y-auto p-5 ${className ?? ''}`}>
      <Text typography="bodyMedium" color={md('onSurfaceVariant')}>
        Tag images from a survey mission with GPS coordinates from your flight log.
      </Text>
      {current.inProgress ? (
        <>
          <Text typography="bodyMedium">Geotagging in progress...</Text>
          <LinearProgressIndicator progress={current.progress / 100.0} className="w-full" />
        </>
      ) : null}
      {[geoTagError(current), note]
        .filter((it) => it !== null)
        .map((message, index) => (
          <Text key={index} typography="bodyMedium" color={md('error')}>
            {message}
          </Text>
        ))}
      {(() => {
        const summary = geoTagSummary(current);
        return summary !== null ? (
          <Text typography="bodyMedium" color={md('primary')}>
            {summary}
          </Text>
        ) : null;
      })()}

      <GeoTagStepRow
        mark={geoTagStep(current.logFile.trim() !== '', 1)}
        title="Flight log"
        detail={current.logFile.trim() === '' ? 'No file selected' : current.logFile.slice(current.logFile.lastIndexOf('/') + 1)}
        enabled={editable}
        onBrowse={() => void pickLog()}
      />
      {logs.length > 0 && editable ? (
        <div className="flex flex-col items-start pl-10">
          {logs.map((log) => (
            <TextButton key={log.absolutePath} onClick={() => GeoTagRun.pickDownloadedLog(log.absolutePath)}>
              {log.name}
            </TextButton>
          ))}
        </div>
      ) : null}
      <GeoTagStepRow
        mark={geoTagStep(current.imageDirectory.trim() !== '', 2)}
        title="Photos folder"
        detail={imageTree !== null ? treeName(imageTree) : 'No folder selected'}
        enabled={editable}
        onBrowse={() => void pickImages()}
      />
      <GeoTagStepRow
        mark="3"
        title="Output folder (optional)"
        detail={geoTagOutputText(outputTree !== null ? treeName(outputTree) : null, imageTree !== null ? treeName(imageTree) : null)}
        enabled={editable}
        onBrowse={() => void pickOutput()}
      />

      <Text typography="titleSmall">Advanced options</Text>
      <OutlinedTextField
        value={offsetText ?? shownOffset(current.timeOffsetSecs)}
        onValueChange={(typed) => {
          setOffsetText(typed);
          const parsed = parsedOffset(typed);
          if (parsed !== null) GeoTagRun.set('timeOffsetSecs', parsed);
        }}
        label="Time offset (seconds)"
        supportingText="Adjust if camera clock differs from flight log"
        singleLine
        enabled={editable}
        className="w-full"
      />
      <div className="flex items-center gap-3">
        <Checkbox checked={current.previewMode} enabled={editable} onCheckedChange={(it) => GeoTagRun.set('previewMode', it)} label="Preview mode (don't write files)" />
        <div className="flex flex-col">
          <Text typography="bodyMedium">Preview mode (don't write files)</Text>
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            Verify time offset before committing
          </Text>
        </div>
      </div>
      <Button
        enabled={current.inProgress || (!busy && current.logFile.trim() !== '' && current.imageDirectory.trim() !== '')}
        onClick={() => (current.inProgress ? GeoTagRun.cancel() : GeoTagRun.start())}
        className="w-full"
      >
        {geoTagButton(current)}
      </Button>
      {current.images.length > 0 ? (
        <>
          <Text typography="titleSmall">{`Images (${current.images.length})`}</Text>
          <GeoTagLegend />
          {current.images.map((image, index) => (
            <GeoTagImageRow key={`${index}:${image.fileName}`} image={image} />
          ))}
        </>
      ) : null}
    </div>
  );
};

const useGeoTagStatusColour = (status: number): string => {
  switch (status) {
    case 1:
      return GEOTAG_BLUE;
    case GEOTAG_TAGGED:
      return GEOTAG_GREEN;
    case 3:
      return GEOTAG_ORANGE;
    case 4:
      return md('error');
    default:
      return withAlpha(md('onSurface'), 0.5);
  }
};

export const GEOTAG_LEGEND: readonly (readonly [number, string])[] = [
  [0, 'Pending'],
  [1, 'Processing'],
  [GEOTAG_TAGGED, 'Tagged'],
  [3, 'Skipped'],
  [4, 'Failed'],
];

const LegendEntry = ({ status, label }: { status: number; label: string }) => (
  <div className="flex items-center gap-1">
    <span className="block size-2.5 rounded-[2px]" style={{ background: useGeoTagStatusColour(status) }} />
    <Text typography="bodySmall">{label}</Text>
  </div>
);

const GeoTagLegend = () => (
  <div className="flex flex-wrap gap-x-3">
    {GEOTAG_LEGEND.map(([status, label]) => (
      <LegendEntry key={status} status={status} label={label} />
    ))}
  </div>
);

const GeoTagImageRow = ({ image }: { image: GeoTagImage }) => {
  const colour = useGeoTagStatusColour(image.status);
  const coordinate = geoTagCoordinate(image);
  return (
    <div className="flex items-center gap-3">
      <span className="block size-3 shrink-0 rounded-[2px]" style={{ background: colour }} />
      <div className="flex min-w-0 flex-1 flex-col">
        <Text typography="bodyMedium" maxLines={1} ellipsis>
          {image.fileName}
        </Text>
        {coordinate !== null ? (
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {coordinate}
          </Text>
        ) : null}
      </div>
      <Text typography="bodySmall" color={image.status === 3 || image.status === 4 ? colour : md('onSurface')}>
        {geoTagImageText(image)}
      </Text>
    </div>
  );
};

const GeoTagStepRow = ({ mark, title, detail, enabled, onBrowse }: { mark: string; title: string; detail: string; enabled: boolean; onBrowse: () => void }) => {
  const done = ![...mark].every((it) => /\p{Nd}/u.test(it));
  return (
    <Surface onClick={onBrowse} enabled={enabled} shape="medium" color={md('surfaceContainer')} className="w-full">
      <div className="flex items-center gap-4 p-4">
        <span className="grid size-8 shrink-0 place-items-center rounded-full" style={{ background: done ? md('primary') : md('secondaryContainer') }}>
          <Text typography="labelLarge" color={done ? md('onPrimary') : md('onSecondaryContainer')}>
            {mark}
          </Text>
        </span>
        <div className="flex min-w-0 flex-1 flex-col">
          <Text typography="titleSmall">{title}</Text>
          <Text typography="bodySmall" color={md('onSurfaceVariant')}>
            {detail}
          </Text>
        </div>
      </div>
    </Surface>
  );
};
