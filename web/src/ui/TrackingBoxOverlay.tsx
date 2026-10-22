import { useMemo, useRef } from 'react';
import { Qgc } from '@/bridge/Qgc';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { argb } from '@/theme/Colors';
import { CAMERA_START_TRACKING, CAMERA_VIEW, type TrackingRequest, trackingCanAim, trackingPointObject, trackingReading, trackingRectObject, trackingRequest } from '@/ui/CameraControl';
import { type Offset, useElementSize } from '@/ui/FlyShell';
import { AUTO_INTERCEPT, autoIntercept } from '@/ui/GuidedActions';
import { usePointerGestures } from '@/ui/VideoGestures';
import { VIDEO_VIEW, paintedRect, videoReading } from '@/ui/VideoView';

const TRACK_COLOUR = argb(0xff00e5ff);

export const TrackingBoxOverlay = ({ className }: { className?: string }) => {
  const cameraJson = useQgcPath(CAMERA_VIEW);
  const videoJson = useQgcPath(VIDEO_VIEW);
  const interceptJson = useQgcPath(AUTO_INTERCEPT);
  const hint = useMemo(() => autoIntercept(interceptJson)?.hint ?? null, [interceptJson]);
  const tracking = useMemo(() => trackingReading(cameraJson), [cameraJson]);
  const source = useMemo(() => videoReading(videoJson)?.sourceSize ?? null, [videoJson]);
  const box = tracking?.box ?? null;
  const { ref, size } = useElementSize();
  const picture = paintedRect(size.width, size.height, source);
  const drag = useRef<{ pressed: Offset; latest: Offset }>({ pressed: { x: 0, y: 0 }, latest: { x: 0, y: 0 } });
  const aim = usePointerGestures(
    tracking !== null && trackingCanAim(tracking)
      ? {
          onDragStart: (at) => {
            drag.current = { pressed: at, latest: at };
          },
          onDrag: (at) => {
            drag.current = { ...drag.current, latest: at };
          },
          onDragEnd: () => {
            const { pressed, latest } = drag.current;
            sendTracking(trackingRequest(pressed.x, pressed.y, latest.x, latest.y, picture));
          },
        }
      : null,
  );

  if (tracking === null) return null;

  return (
    <div ref={ref} className={`pointer-events-none ${className ?? 'relative'}`}>
      <div {...aim} className={`absolute inset-0 ${trackingCanAim(tracking) ? 'pointer-events-auto touch-none' : ''}`} />
      {box !== null ? (
        <svg width={size.width} height={size.height} className="absolute inset-0 block overflow-visible">
          <rect
            x={picture.left + box.x * picture.width}
            y={picture.top + box.y * picture.height}
            width={box.width * picture.width}
            height={box.height * picture.height}
            fill="none"
            stroke={TRACK_COLOUR}
            strokeWidth={4}
          />
        </svg>
      ) : null}
      {hint !== null ? (
        <div className="absolute inset-x-0 top-3 flex justify-center">
          <span className="rounded-full bg-black/60 px-3 py-1">
            <Text typography="labelMedium" color="#ffffff">
              {hint}
            </Text>
          </span>
        </div>
      ) : null}
    </div>
  );
};

const sendTracking = (request: TrackingRequest | null) => {
  if (request === null) return;
  if (request.kind === 'Box') void Qgc.invoke(CAMERA_START_TRACKING, trackingRectObject(request.rect));
  else void Qgc.invoke(CAMERA_START_TRACKING, trackingPointObject(request), request.radius);
};
