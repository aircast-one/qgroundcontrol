import { useMemo } from 'react';
import { useQgcPath } from '@/bridge/QgcState';
import { Text } from '@/m3';
import { Color, argb, md } from '@/theme/Colors';
import { DETECTIONS, detectionTrouble, detections, visibleBoxes } from '@/ui/Detections';
import { useElementSize } from '@/ui/FlyShell';
import { VIDEO_VIEW, paintedRect, videoReading } from '@/ui/VideoView';

const BOX_COLOUR = argb(0xff4dd0e1);
const TARGET_COLOUR = argb(0xffffd54f);

export const DetectionOverlay = ({ className }: { className?: string }) => {
  const json = useQgcPath(DETECTIONS);
  const videoJson = useQgcPath(VIDEO_VIEW);
  const reading = useMemo(() => detections(json), [json]);
  const boxes = useMemo(() => visibleBoxes(reading), [reading]);
  const trouble = useMemo(() => detectionTrouble(reading), [reading]);
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
      {trouble !== null ? (
        <Text typography="labelSmall" color={md('onErrorContainer')} className="absolute start-0 bottom-0 px-1 py-0.5" style={{ background: md('errorContainer') }}>
          {`Detections: ${trouble}`}
        </Text>
      ) : null}
    </div>
  );
};
