import { VideoCommands } from '@/bridge/BridgeCommands';
import { isNull, optInt } from '@/bridge/Json';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { VIDEO_VIEW } from '@/ui/VideoView';

const DEVICE_CAMERA = 'deviceCamera';
const FULL_TURN = 360;

export const deviceCameraRotation = (sensorDegrees: number, front: boolean, displayDegrees: number): number =>
  front ? (sensorDegrees + displayDegrees) % FULL_TURN : (sensorDegrees - displayDegrees + FULL_TURN) % FULL_TURN;

const cameraGranted = async (): Promise<boolean> => {
  try {
    return (await navigator.permissions.query({ name: 'camera' })).state === 'granted';
  } catch {
    return false;
  }
};

const requestCamera = async (): Promise<boolean> => {
  try {
    const stream = await navigator.mediaDevices.getUserMedia({ video: true });
    stream.getTracks().map((track) => track.stop());
    return true;
  } catch {
    return false;
  }
};

export const DeviceCameraHost = () => {
  const videoJson = useQgcPath(VIDEO_VIEW);
  const camera = videoJson === null || isNull(videoJson, DEVICE_CAMERA) ? null : optInt(videoJson, DEVICE_CAMERA);

  useLaunchedEffect([camera], async () => {
    if (camera === null || (await cameraGranted())) return;
    if (await requestCamera()) void VideoCommands.restart();
  });

  return null;
};
