import SwiftUI

private let PREVIEW_ASPECT: CGFloat = 16.0 / 9.0
private let PREVIEW_CORNER: CGFloat = 12

// Plays a camera's WHEP stream on a native video channel the core hands out for view.videoPreview(<endpoint>),
// the same surface the fly view draws its main and picture-in-picture video on.
struct WhepPreview: View {
    let endpoint: String
    let watch: String
    let name: String
    @QgcPath private var previewJson: JSON?
    @Environment(\.openURL) private var openURL

    init(endpoint: String, watch: String, name: String) {
        self.endpoint = endpoint
        self.watch = watch
        self.name = name
        _previewJson = QgcPath(videoPreviewPath(endpoint))
    }

    private func open() {
        if let target = URL(string: watch) { openURL(target) }
    }

    var body: some View {
        let preview = videoPreview(previewJson)
        Button(action: open) {
            Color.black
                .aspectRatio(PREVIEW_ASPECT, contentMode: .fit)
                .frame(maxWidth: .infinity)
                .overlay {
                    if let channel = preview?.channel {
                        VideoChannelSurface(channel: channel).id(channel)
                    }
                }
                .overlay {
                    if preview?.playing != true {
                        VStack(spacing: 8) {
                            if preview?.trying != false {
                                ProgressView().tint(Color.white)
                            }
                            Text(preview?.text ?? "").font(.labelMedium).foregroundStyle(Color.white).multilineTextAlignment(.center)
                        }
                        .padding(.horizontal, 24)
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        .background(Color.black)
                    }
                }
                .overlay(alignment: .bottomTrailing) {
                    HStack(spacing: 4) {
                        Image(.openInNew).font(.system(size: 12))
                        Text("Open camera").font(.labelSmall)
                    }
                    .foregroundStyle(Color.white)
                    .padding(.horizontal, 8)
                    .padding(.vertical, 4)
                    .background(Color.black.opacity(0.6), in: RoundedRectangle(cornerRadius: 4))
                    .padding(8)
                }
                .clipShape(RoundedRectangle(cornerRadius: PREVIEW_CORNER))
                .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel("Open the \(name) camera")
    }
}

struct PreviewPlaceholder: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Color.black.opacity(0.8)
            .aspectRatio(PREVIEW_ASPECT, contentMode: .fit)
            .frame(maxWidth: .infinity)
            .overlay {
                VStack(spacing: 8) {
                    Image(.videocamOff).font(.system(size: 26)).foregroundStyle(theme.colors.onSurfaceVariant)
                    Text(text).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant).multilineTextAlignment(.center)
                }
                .padding(.horizontal, 24)
            }
            .clipShape(RoundedRectangle(cornerRadius: PREVIEW_CORNER))
    }
}

struct PartDisclosure<Content: View>: View {
    let part: SetupPart
    let section: String
    let opened: String
    @ViewBuilder let content: () -> Content
    @Environment(\.LocalDeviceScroll) private var scroll

    var body: some View {
        DeviceDisclosure(part.title, summary: part.summary, badge: part.badge, initiallyOpen: part.open || opened == section) {
            VStack(alignment: .leading, spacing: 0) { content() }
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.bottom, 8)
        }
        .id(section)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(part.title)
        .task { @MainActor in
            guard opened == section else { return }
            await Task.yield()
            scroll?.scrollTo(section, anchor: .top)
        }
    }
}

struct LinkedNoticeView: View {
    let notice: LinkedNotice
    let onOpen: (String) -> Void

    var body: some View {
        if !notice.text.isEmpty {
            DeviceNotice(notice: DeviceBadge(level: notice.level, text: notice.text))
            if let link = notice.link {
                Button("\(link.label) →") { onOpen(link.href) }
                    .buttonStyle(.text)
                    .padding(.horizontal, 8)
            }
        }
    }
}

struct NoteLink: View {
    let text: String
    let label: String
    var scale: TypeScale = .bodySmall
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        PlanFlowRow(spacing: 8, lineSpacing: 0) {
            Text(text).font(scale).foregroundStyle(theme.colors.onSurfaceVariant)
            Button(label, action: onClick).buttonStyle(.text)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, DEVICE_LIST_PADDING)
        .padding(.vertical, 8)
    }
}
