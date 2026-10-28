import { useEffect, useMemo, useState } from 'react';
import { type Json } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { AlertDialog, Surface, Text, TextButton } from '@/m3';
import { Color, aircast, argb, md, withAlpha } from '@/theme/Colors';
import { CAMERA_START_PURSUIT, CAMERA_STOP_PURSUIT, type PursuitReading, pursuitReading } from '@/ui/CameraControl';
import { DETECTIONS, detectionTrouble, detections, visibleBoxes } from '@/ui/Detections';
import { useElementSize } from '@/ui/FlyShell';
import { osdShadow, useOsdBackdrop } from '@/ui/Osd';
import { VIDEO_VIEW, paintedRect, videoReading } from '@/ui/VideoView';

const BOX_COLOUR = argb(0xff4dd0e1);
const TARGET_COLOUR = argb(0xffffd54f);
const REFUSAL_MS = 4000;
const SCRIM_ALPHA = 0.55;
const DISABLED_CONTENT_ALPHA = 0.6;
const BORDER_ALPHA = 0.4;

const Pill = ({ onClick, color, contentColor, border, label, children }: { onClick?: () => void; color: string; contentColor: string; border?: string; label: string; children: string }) => (
  <Surface onClick={onClick} shape="full" color={color} contentColor={contentColor} border={border} className={`pointer-events-auto flex min-w-[140px] items-center justify-center px-6 py-3 ${osdShadow()}`} label={label}>
    <Text typography="titleMedium" maxLines={1}>
      {children}
    </Text>
  </Surface>
);

const OverlayNote = ({ background, color, children }: { background: string; color: string; children: string }) => (
  <Text typography="labelSmall" color={color} className={`rounded-full px-2.5 py-0.5 ${osdShadow()}`} style={{ background }}>
    {children}
  </Text>
);

const ChaseOsd = ({ reading }: { reading: PursuitReading }) => {
  const [confirming, setConfirming] = useState(false);
  const [refused, setRefused] = useState<string | null>(null);
  const scrim = useOsdBackdrop(withAlpha(Color.Black, SCRIM_ALPHA));
  const foreground = aircast('outdoorForeground');
  useEffect(() => {
    if (refused === null) return undefined;
    const timer = setTimeout(() => setRefused(null), REFUSAL_MS);
    return () => clearTimeout(timer);
  }, [refused]);

  if (reading.engaged)
    return <Pill onClick={() => void Qgc.invoke(CAMERA_STOP_PURSUIT)} color={md('error')} contentColor={md('onError')} label="Stop chasing the target">Chasing — Stop</Pill>;

  return (
    <div className="flex flex-col items-center gap-1">
      <Pill
        onClick={reading.ready ? () => setConfirming(true) : undefined}
        color={reading.ready ? md('primary') : scrim}
        contentColor={reading.ready ? md('onPrimary') : withAlpha(foreground, DISABLED_CONTENT_ALPHA)}
        border={reading.ready ? undefined : `1px solid ${withAlpha(foreground, BORDER_ALPHA)}`}
        label="Chase the locked target"
      >
        Chase
      </Pill>
      {!reading.ready && reading.reason.trim() !== '' ? (
        <OverlayNote background={scrim} color={foreground}>
          {reading.reason}
        </OverlayNote>
      ) : null}
      {refused !== null ? (
        <OverlayNote background={md('error')} color={md('onError')}>
          {refused}
        </OverlayNote>
      ) : null}
      {confirming ? (
        <AlertDialog
          onDismissRequest={() => setConfirming(false)}
          title="Chase the target?"
          text="The aircraft will fly itself toward the locked target. Keep the area clear and be ready to take manual control."
          confirmButton={
            <TextButton
              onClick={() => {
                setConfirming(false);
                void (async () => setRefused(await Qgc.refusalOf(CAMERA_START_PURSUIT)))();
              }}
            >
              <span style={{ color: md('error') }}>Chase</span>
            </TextButton>
          }
          dismissButton={<TextButton onClick={() => setConfirming(false)}>Cancel</TextButton>}
        />
      ) : null}
    </div>
  );
};

export const DetectionOverlay = ({ className }: { className?: string }) => {
  const json = useQgcPath(DETECTIONS);
  const videoJson = useQgcPath(VIDEO_VIEW);
  const reading = useMemo(() => detections(json), [json]);
  const boxes = useMemo(() => visibleBoxes(reading), [reading]);
  const trouble = useMemo(() => detectionTrouble(reading), [reading]);
  const pursuit = useMemo(() => pursuitReading(json as Json | null), [json]);
  const locked = useMemo(() => boxes.some((box) => box.target), [boxes]);
  const source = useMemo(() => videoReading(videoJson)?.sourceSize ?? null, [videoJson]);
  const { ref, size } = useElementSize();
  const picture = paintedRect(size.width, size.height, source);

  return (
    <div ref={ref} className={`pointer-events-none ${className ?? 'relative'}`}>
      <svg width={size.width} height={size.height} className="absolute inset-0 block overflow-visible">
        {boxes.map((box, index) => {
          const width = box.target ? 4 : 2;
          return (
            <rect
              key={index}
              x={picture.left + box.x * picture.width}
              y={picture.top + box.y * picture.height}
              width={box.w * picture.width}
              height={box.h * picture.height}
              fill="none"
              stroke={box.target ? TARGET_COLOUR : BOX_COLOUR}
              strokeWidth={width}
            />
          );
        })}
      </svg>
      {boxes.map((box, index) => {
        const caption = box.caption;
        return caption.trim() === '' ? null : (
          <Text
            key={index}
            typography="labelSmall"
            color={Color.Black}
            className="absolute px-[3px] whitespace-nowrap"
            style={{ left: picture.left + box.x * picture.width, top: picture.top + box.y * picture.height, background: box.target ? TARGET_COLOUR : BOX_COLOUR }}
          >
            {caption}
          </Text>
        );
      })}
      {pursuit !== null && locked ? (
        <div className="absolute inset-x-0 top-3 flex justify-center px-3">
          <ChaseOsd reading={pursuit} />
        </div>
      ) : null}
      {trouble !== null ? (
        <Text typography="labelSmall" color={md('onErrorContainer')} className="absolute start-0 bottom-0 px-1 py-0.5" style={{ background: md('errorContainer') }}>
          {`Detections: ${trouble}`}
        </Text>
      ) : null}
    </div>
  );
};
