import { createContext, useContext, useMemo, useState } from 'react';
import { VideoCommands } from '@/bridge/BridgeCommands';
import { useQgcPath } from '@/bridge/QgcState';
import { useLaunchedEffect } from '@/kotlin/Compose';
import { delay } from '@/kotlin/Std';
import { FilledTonalButton, Surface, Text, TextButton } from '@/m3';
import { Drawable } from '@/res/Icon';
import type { DrawableName } from '@/res/drawables';
import { md } from '@/theme/Colors';
import { type AppNavigationState, LocalAppNavigation } from '@/ui/AppNavigation';
import { pageLook, pageTitle } from '@/ui/SettingsScreen';
import { VIDEO_SOURCES_PAGE } from '@/ui/Cameras';
import { LOOKING_TITLE } from '@/ui/ConnectingCard';
import { FLY_STATE, flyState, useHasVehicle } from '@/ui/FlyState';
import { useOsdBackdrop } from '@/ui/Osd';
import { useRememberSyntheticAvailable, useRememberSyntheticChosen } from '@/ui/SyntheticView';
import type { VideoReading } from '@/ui/VideoView';

const PROLONGED_SECONDS = 8;
const PANEL_MAX_WIDTH = 360;
const videoSourcesPlace = (): string => `Settings › ${pageLook(VIDEO_SOURCES_PAGE).group.title} › ${pageTitle(VIDEO_SOURCES_PAGE)}`;

export const elapsedText = (seconds: number): string => {
  if (seconds < 60) return `${seconds} s`;
  if (seconds < 3600) return `${Math.trunc(seconds / 60)} min`;
  return `${Math.trunc(seconds / 3600)} h ${Math.trunc((seconds % 3600) / 60)} min`;
};

export const activeCameraStatus = (video: VideoReading): string | null => {
  const status = video.cameras.find((it) => it.slot === video.activeSource)?.status ?? null;
  return status === null || status.trim() === '' ? null : status;
};

export const noVideoDetail = (video: VideoReading, seconds: number): string => {
  if (video.noVideoReason.trim() !== '') return video.noVideoReason;
  if (video.streaming) return `Receiving data — waiting for video for ${elapsedText(seconds)}`;
  return `${video.noVideoText} for ${elapsedText(seconds)}`;
};

export type FlyWait = { readonly name: 'Aircraft' | 'Position'; readonly ordinal: number; readonly title: string; readonly detail: string; readonly short: string };

export const FlyWait = {
  Aircraft: { name: 'Aircraft', ordinal: 0, title: LOOKING_TITLE, detail: '', short: 'Not connected' },
  Position: { name: 'Position', ordinal: 1, title: "Waiting for the aircraft's position", detail: 'The synthetic view draws once the aircraft has a GPS fix and a home.', short: 'Waiting for position' },
} as const satisfies Record<string, FlyWait>;

export const FlyWaitEntries: readonly FlyWait[] = [FlyWait.Aircraft, FlyWait.Position];

export const flyWait = (connected: boolean, video: VideoReading | null, syntheticChosen: boolean, syntheticReady: boolean): FlyWait | null => {
  if (video?.decoding === true || !syntheticChosen) return null;
  if (!connected) return FlyWait.Aircraft;
  if (!syntheticReady) return FlyWait.Position;
  return null;
};

export type NoVideoAction = { readonly name: 'None' | 'SetUp' | 'Settings' | 'TurnOn'; readonly ordinal: number };

export const NoVideoAction = {
  None: { name: 'None', ordinal: 0 },
  SetUp: { name: 'SetUp', ordinal: 1 },
  Settings: { name: 'Settings', ordinal: 2 },
  TurnOn: { name: 'TurnOn', ordinal: 3 },
} as const satisfies Record<string, NoVideoAction>;

export const NoVideoActionEntries: readonly NoVideoAction[] = [NoVideoAction.None, NoVideoAction.SetUp, NoVideoAction.Settings, NoVideoAction.TurnOn];

export type NoVideoState = { title: string; detail: string; action: NoVideoAction };

export const unavailableVideoState = (video: VideoReading): NoVideoState | null => {
  if (video.available) return null;
  if (!video.streamEnabled) return { title: 'Video off', detail: 'It turns back on when you arm.', action: NoVideoAction.TurnOn };
  if (!video.sourceChosen) return { title: 'No video source', detail: `Add a camera in ${videoSourcesPlace()}.`, action: NoVideoAction.SetUp };
  if (!video.cameras.some((it) => it.configured)) return { title: 'No stream address', detail: `Enter the stream address in ${videoSourcesPlace()}.`, action: NoVideoAction.Settings };
  return { title: video.summary, detail: '', action: NoVideoAction.Settings };
};

export const whileArmed = (state: NoVideoState): NoVideoState => ({ ...state, detail: '', action: state.action === NoVideoAction.TurnOn ? state.action : NoVideoAction.None });

export type NoVideoButton = { label: string; onClick: () => void };

export const videoSourcesButton = (navigation: AppNavigationState): NoVideoButton => ({
  label: 'Video sources',
  onClick: () => {
    navigation.settingsPage.value = VIDEO_SOURCES_PAGE;
  },
});

const TURN_VIDEO_ON: NoVideoButton = { label: 'Turn video on', onClick: () => void VideoCommands.turnStreamOn() };

export type NoVideoSize = { readonly name: 'Full' | 'Pill' | 'Thumb'; readonly ordinal: number };

export const NoVideoSize = {
  Full: { name: 'Full', ordinal: 0 },
  Pill: { name: 'Pill', ordinal: 1 },
  Thumb: { name: 'Thumb', ordinal: 2 },
} as const satisfies Record<string, NoVideoSize>;

export const NoVideoSizeEntries: readonly NoVideoSize[] = [NoVideoSize.Full, NoVideoSize.Pill, NoVideoSize.Thumb];

export const LocalNoVideoSize = createContext<NoVideoSize>(NoVideoSize.Full);

const NoVideoLayout = ({
  title,
  detail = '',
  primary = null,
  secondary = null,
  compactTitle = title,
  icon = 'ic_videocam_off',
}: {
  title: string;
  detail?: string;
  primary?: NoVideoButton | null;
  secondary?: NoVideoButton | null;
  compactTitle?: string;
  icon?: DrawableName;
}) => {
  const size = useContext(LocalNoVideoSize);
  const pillColor = useOsdBackdrop(md('surfaceContainerHigh'));
  if (size === NoVideoSize.Thumb) {
    return (
      <div className="flex min-w-0 flex-col items-center gap-1">
        <Drawable name={icon} size={24} style={{ color: md('onSurfaceVariant') }} />
        <Text typography="labelMedium" color={md('onSurfaceVariant')} maxLines={1} ellipsis className="max-w-full">
          {compactTitle}
        </Text>
      </div>
    );
  }
  if (size === NoVideoSize.Pill) {
    return (
      <Surface shape="extraLarge" color={pillColor} className="max-w-full">
        <div className="flex items-center gap-2 ps-3 pe-1">
          <Drawable name={icon} size={20} className="shrink-0" style={{ color: md('onSurfaceVariant') }} />
          <Text typography="labelLarge" color={md('onSurface')} maxLines={1} ellipsis className="min-w-0 py-3">
            {compactTitle}
          </Text>
          {primary !== null ? (
            <TextButton onClick={primary.onClick} className="shrink-0 whitespace-nowrap">
              {primary.label}
            </TextButton>
          ) : null}
        </div>
      </Surface>
    );
  }
  return (
    <div className="flex flex-col items-center gap-2 px-6" style={{ maxWidth: PANEL_MAX_WIDTH }}>
      <Drawable name={icon} size={40} style={{ color: md('onSurfaceVariant') }} />
      <Text typography="titleMedium" color={md('onSurface')} textAlign="center">
        {title}
      </Text>
      {detail.trim() !== '' ? (
        <Text typography="bodyMedium" color={md('onSurfaceVariant')} textAlign="center">
          {detail}
        </Text>
      ) : null}
      {primary !== null || secondary !== null ? (
        <div className="flex gap-2 pt-2">
          {primary !== null ? <FilledTonalButton onClick={primary.onClick}>{primary.label}</FilledTonalButton> : null}
          {secondary !== null ? <TextButton onClick={secondary.onClick}>{secondary.label}</TextButton> : null}
        </div>
      ) : null}
    </div>
  );
};

export const NoVideoPanel = ({ video }: { video: VideoReading | null }) => {
  const size = useContext(LocalNoVideoSize);
  const wait = flyWait(useHasVehicle(), video, useRememberSyntheticChosen(), useRememberSyntheticAvailable());
  const videoSources = videoSourcesButton(useContext(LocalAppNavigation));
  const flyJson = useQgcPath(FLY_STATE);
  const armed = useMemo(() => flyState(flyJson)?.armed === true, [flyJson]);
  if (wait !== null) {
    if (wait === FlyWait.Aircraft && size !== NoVideoSize.Full) return null;
    return <NoVideoLayout title={wait.title} detail={wait.detail} compactTitle={wait.short} icon="ic_flight" />;
  }
  const state = video === null ? null : unavailableVideoState(video);
  const unavailable = state === null ? null : armed ? whileArmed(state) : state;
  if (unavailable !== null) {
    const primary =
      unavailable.action === NoVideoAction.None
        ? null
        : unavailable.action === NoVideoAction.SetUp
          ? { ...videoSources, label: 'Set up video' }
          : unavailable.action === NoVideoAction.Settings
            ? videoSources
            : TURN_VIDEO_ON;
    return <NoVideoLayout title={unavailable.title} detail={unavailable.detail} primary={primary} />;
  }
  return <NoVideoStreamPanel video={video} videoSources={videoSources} />;
};

const NoVideoStreamPanel = ({ video, videoSources }: { video: VideoReading | null; videoSources: NoVideoButton }) => {
  const [seconds, setSeconds] = useState(0);
  useLaunchedEffect([video !== null, video?.decoding], async (scope) => {
    setSeconds(0);
    const tick = async (): Promise<void> => {
      if (video === null) return;
      await delay(1000);
      if (!scope.isActive()) return;
      setSeconds((was) => was + 1);
      return tick();
    };
    await tick();
  });
  if (video === null) return <NoVideoLayout title="No video" />;
  if (seconds >= PROLONGED_SECONDS) {
    return (
      <NoVideoLayout
        title="No video signal"
        detail={noVideoDetail(video, seconds)}
        primary={{
          label: 'Retry',
          onClick: () => {
            setSeconds(0);
            void VideoCommands.restart();
          },
        }}
        secondary={videoSources}
        compactTitle="No video"
      />
    );
  }
  return <NoVideoLayout title={activeCameraStatus(video) ?? video.summary} />;
};
