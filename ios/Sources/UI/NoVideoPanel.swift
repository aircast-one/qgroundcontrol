import SwiftUI

private let PROLONGED_SECONDS = 8
private let PANEL_MAX_WIDTH: CGFloat = 360
private let VIDEO_SOURCES_PLACE = "Settings \u{203A} \(pageLook(VIDEO_SOURCES_PAGE).group.title) \u{203A} \(pageTitle(VIDEO_SOURCES_PAGE))"

func elapsedText(_ seconds: Int) -> String {
    if seconds < 60 { return "\(seconds) s" }
    if seconds < 3600 { return "\(seconds / 60) min" }
    return "\(seconds / 3600) h \((seconds % 3600) / 60) min"
}

func activeCameraStatus(_ video: VideoReading) -> String? {
    video.cameras.first { $0.slot == video.activeSource }.flatMap { $0.status.isBlank ? nil : $0.status }
}

func noVideoDetail(_ video: VideoReading, _ seconds: Int) -> String {
    if !video.noVideoReason.isBlank { return video.noVideoReason }
    if video.streaming { return "Receiving data \u{2014} waiting for video for \(elapsedText(seconds))" }
    return "\(video.noVideoText) for \(elapsedText(seconds))"
}

enum NoVideoAction: Equatable {
    case None, SetUp, Settings, TurnOn
}

struct NoVideoState: Equatable {
    var title: String
    var detail: String
    var action: NoVideoAction
}

func unavailableVideoState(_ video: VideoReading) -> NoVideoState? {
    if video.available { return nil }
    if !video.streamEnabled { return NoVideoState(title: "Video off", detail: "It turns back on when you arm.", action: .TurnOn) }
    if !video.sourceChosen { return NoVideoState(title: "No video source", detail: "Add a camera in \(VIDEO_SOURCES_PLACE).", action: .SetUp) }
    if !video.cameras.contains(where: \.configured) {
        return NoVideoState(title: "No stream address", detail: "Enter the stream address in \(VIDEO_SOURCES_PLACE).", action: .Settings)
    }
    return NoVideoState(title: video.summary, detail: "", action: .Settings)
}

func whileArmed(_ state: NoVideoState) -> NoVideoState {
    NoVideoState(title: state.title, detail: "", action: state.action == .TurnOn ? .TurnOn : .None)
}

struct NoVideoButton {
    var label: String
    let onClick: () -> Void
}

func videoSourcesButton(_ navigation: AppNavigationState) -> NoVideoButton {
    NoVideoButton(label: "Video sources") { navigation.settingsPage = VIDEO_SOURCES_PAGE }
}

private let TURN_VIDEO_ON = NoVideoButton(label: "Turn video on") { offMain { VideoCommands.turnStreamOn() } }

enum NoVideoSize {
    case Full, Pill, Thumb
}

extension EnvironmentValues {
    @Entry var noVideoSize: NoVideoSize = .Full
}

private struct NoVideoLayout: View {
    let title: String
    var detail: String = ""
    var primary: NoVideoButton? = nil
    var secondary: NoVideoButton? = nil
    var compactTitle: String? = nil
    @Environment(\.theme) private var theme
    @Environment(\.noVideoSize) private var size
    @Environment(\.flyOsd) private var osd

    var body: some View {
        switch size {
        case .Thumb:
            VStack(spacing: 4) {
                Image(.videocamOff).font(.system(size: 22)).foregroundStyle(theme.colors.onSurfaceVariant)
                Text(compactTitle ?? title).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
            }
        case .Pill:
            HStack(spacing: 8) {
                Image(.videocamOff).font(.system(size: 18)).foregroundStyle(theme.colors.onSurfaceVariant)
                Text(compactTitle ?? title).font(.labelLarge).foregroundStyle(theme.colors.onSurface).lineLimit(1).padding(.vertical, 12)
                if let primary {
                    Button(action: primary.onClick) {
                        Text(primary.label).lineLimit(1).fixedSize().padding(.horizontal, 12).frame(minWidth: 58, minHeight: 40)
                    }
                    .buttonStyle(.borderless)
                }
            }
            .padding(.leading, 12)
            .padding(.trailing, primary == nil ? 16 : 4)
            .background(osdBackdrop(theme.colors.surfaceContainerHigh, osd), in: Capsule())
        case .Full:
            VStack(spacing: 8) {
                Image(.videocamOff).font(.system(size: 36)).foregroundStyle(theme.colors.onSurfaceVariant)
                Text(title).font(.titleMedium).foregroundStyle(theme.colors.onSurface).multilineTextAlignment(.center)
                if !detail.isBlank {
                    Text(detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).multilineTextAlignment(.center)
                }
                if primary != nil || secondary != nil {
                    HStack(spacing: 8) {
                        if let primary { Button(primary.label, action: primary.onClick).buttonStyle(.bordered) }
                        if let secondary { Button(secondary.label, action: secondary.onClick).buttonStyle(.borderless) }
                    }
                    .padding(.top, 8)
                }
            }
            .padding(.horizontal, 24)
            .frame(maxWidth: PANEL_MAX_WIDTH)
        }
    }
}

struct NoVideoPanel: View {
    let video: VideoReading?
    @Environment(AppNavigationState.self) private var navigation
    @QgcPath(FLY_STATE) private var flyJson

    var body: some View {
        let videoSources = videoSourcesButton(navigation)
        let armed = flyState(flyJson)?.armed == true
        if let unavailable = video.flatMap(unavailableVideoState).map({ armed ? whileArmed($0) : $0 }) {
            NoVideoLayout(title: unavailable.title, detail: unavailable.detail, primary: primary(unavailable.action, videoSources))
        } else {
            NoVideoStreamPanel(video: video, videoSources: videoSources)
        }
    }

    private func primary(_ action: NoVideoAction, _ videoSources: NoVideoButton) -> NoVideoButton? {
        switch action {
        case .None: nil
        case .SetUp: NoVideoButton(label: "Set up video", onClick: videoSources.onClick)
        case .Settings: videoSources
        case .TurnOn: TURN_VIDEO_ON
        }
    }
}

private struct StreamWait: Equatable {
    let present: Bool
    let decoding: Bool?
}

private struct NoVideoStreamPanel: View {
    let video: VideoReading?
    let videoSources: NoVideoButton
    @State private var seconds = 0

    var body: some View {
        content.task(id: StreamWait(present: video != nil, decoding: video?.decoding)) {
            seconds = 0
            while video != nil && !Task.isCancelled {
                try? await Task.sleep(for: .seconds(1))
                if !Task.isCancelled { seconds += 1 }
            }
        }
    }

    @ViewBuilder private var content: some View {
        if let video {
            if seconds >= PROLONGED_SECONDS {
                NoVideoLayout(
                    title: "No video signal",
                    detail: noVideoDetail(video, seconds),
                    primary: NoVideoButton(label: "Retry") {
                        seconds = 0
                        offMain { VideoCommands.restart() }
                    },
                    secondary: videoSources,
                    compactTitle: "No video"
                )
            } else {
                NoVideoLayout(title: activeCameraStatus(video) ?? video.summary)
            }
        } else {
            NoVideoLayout(title: "No video")
        }
    }
}
