import { useRef, useState } from 'react';
import { type Json, optInt, optJSONObject, optString } from '@/bridge/Json';
import { Qgc } from '@/bridge/Qgc';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { Text } from '@/m3';

export const OPTICAL_FLOW_SCREEN = 'opticalFlow';
const OPTICAL_FLOW_VIEW = 'view.opticalFlow';
const OPTICAL_FLOW_POLL_MS = 500;

export type FlowFrame = { width: number; height: number; rgba: Uint8Array };

const decodeBase64 = (data: string): Uint8Array => {
  try {
    return Uint8Array.from(atob(data.replace(/\s/g, '')), (char) => char.charCodeAt(0));
  } catch {
    return new Uint8Array();
  }
};

export const flowFrame = (view: Json | null): FlowFrame | null => {
  const image = optJSONObject(view, 'image');
  return image === null ? null : { width: optInt(image, 'width'), height: optInt(image, 'height'), rgba: decodeBase64(optString(image, 'data')) };
};

const channel = (bytes: Uint8Array, at: number): number => (bytes[at] ?? 0) & 0xff;

export const argbPixels = (rgba: Uint8Array, count: number): Int32Array =>
  Int32Array.from({ length: count }, (_, pixel) => {
    const at = pixel * 4;
    return (channel(rgba, at + 3) << 24) | (channel(rgba, at) << 16) | (channel(rgba, at + 1) << 8) | channel(rgba, at + 2);
  });

const bitmapOf = (frame: FlowFrame): string | null => {
  if (frame.width <= 0 || frame.height <= 0) return null;
  const pixels = argbPixels(frame.rgba, frame.width * frame.height);
  const bytes = Uint8ClampedArray.from({ length: pixels.length * 4 }, (_, at) => {
    const pixel = pixels[at >> 2] ?? 0;
    const shift = [16, 8, 0, 24][at & 3] ?? 0;
    return (pixel >>> shift) & 0xff;
  });
  const canvas = Object.assign(document.createElement('canvas'), { width: frame.width, height: frame.height });
  const context = canvas.getContext('2d');
  if (context === null) return null;
  context.putImageData(new ImageData(bytes, frame.width, frame.height), 0, 0);
  return canvas.toDataURL();
};

export const OpticalFlowScreen = ({ className }: { className?: string }) => {
  const index = useRef(-1);
  const [image, setImage] = useState<string | null>(null);
  useLaunchedEffect([], async (scope) => {
    const loop = async (): Promise<void> => {
      if (!scope.isActive()) return;
      const view = await Qgc.get(OPTICAL_FLOW_VIEW);
      if (!scope.isActive()) return;
      const latest = optInt(view, 'index');
      if (latest !== index.current) {
        index.current = latest;
        const frame = flowFrame(view);
        setImage(frame === null ? null : bitmapOf(frame));
      }
      await delay(OPTICAL_FLOW_POLL_MS);
      return loop();
    };
    await loop();
  });
  return (
    <div className={`flex w-full flex-col items-center gap-3 p-4 ${className ?? ''}`}>
      <Text typography="titleMedium">Optical flow camera</Text>
      {image !== null ? <img src={image} alt="Optical flow camera image" className="aspect-[4/3] w-1/2 object-contain" /> : null}
    </div>
  );
};
