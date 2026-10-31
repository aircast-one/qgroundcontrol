import AppKit
import SwiftUI

final class AppUpdateStore: ObservableObject, Probeable {
    static let probeID = "appUpdate"
    static let shared = AppUpdateStore()

    @Published private(set) var update = AppUpdate.none
    @Published private(set) var refusal: String?

    private let client = "appUpdate"
    private var watching = false
    private var openWhenReady = false
    private var opened = ""

    func startWatching() {
        guard !watching else { return }
        watching = true
        BridgeWatch.watch(client, ["view.appUpdate"]) { [weak self] view in
            self?.apply(AppUpdate(view))
        }
        apply(AppUpdate(Bridge.group("view.appUpdate")))
    }

    func perform(_ action: AppUpdate.Action) {
        openWhenReady = action == .update || (openWhenReady && action != .cancel)
        refusal = action.path.map { AppUpdate.refusal(Bridge.invoke($0)) } ?? openInstaller()
        openWhenReady = openWhenReady && refusal == nil
    }

    func setAutoCheck(_ on: Bool) {
        refusal = AppUpdate.refusal(Bridge.invoke("appUpdate.setAutoCheck", [on]))
    }

    func openReleaseNotes() {
        guard let url = URL(string: update.releaseUrl) else { return }
        NSWorkspace.shared.open(url)
    }

    private func apply(_ read: AppUpdate) {
        if read != update { update = read }
        switch read.phase {
        case .ready where openWhenReady && read.canInstall:
            openWhenReady = false
            refusal = openInstaller()
        case .failed, .idle, .upToDate:
            openWhenReady = false
        default:
            break
        }
    }

    private func openInstaller() -> String? {
        guard update.enabled(.openInstaller) else { return update.installRefusal ?? AppUpdate.notOpened }
        guard NSWorkspace.shared.open(URL(fileURLWithPath: update.file)) else { return AppUpdate.notOpened }
        opened = update.file
        return nil
    }

    func probeState() -> [String: Any] {
        ["phase": update.phase.rawValue, "current": update.current, "latest": update.latest,
         "title": update.title, "bannerShown": update.bannerShown, "canInstall": update.canInstall,
         "progress": update.progress, "file": update.file, "error": update.error,
         "snoozed": update.snoozed, "autoCheck": update.autoCheck, "refusal": refusal ?? "",
         "openWhenReady": openWhenReady, "opened": opened, "watching": watching]
    }
}

struct AppUpdateSection: View {
    @ObservedObject var store: AppUpdateStore

    var body: some View {
        let update = store.update
        VStack(alignment: .leading, spacing: 0) {
            SectionLabel(text: "Updates")
            GroupCard {
                GroupRow(title: "\(AppUpdate.application) \(update.current)",
                         description: update.caption, showSeparator: false,
                         trailing: { button(.check, update) })
                GroupRow(title: AppUpdate.autoCheckTitle, description: AppUpdate.autoCheckNote,
                         trailing: {
                             Toggle(AppUpdate.autoCheckTitle,
                                    isOn: Binding(get: { update.autoCheck }, set: store.setAutoCheck))
                                 .labelsHidden()
                                 .toggleStyle(.switch)
                         })
                if update.phase == .failed {
                    GroupRow(title: update.error, titleLines: 4,
                             leading: {
                                 Image(systemName: "exclamationmark.triangle.fill").foregroundColor(.orange)
                             },
                             trailing: {
                                 Button(AppUpdate.tryAgain) { store.perform(update.retry) }
                             })
                }
                if let action = update.offerAction {
                    GroupRow(title: update.title, description: update.offerDetail,
                             trailing: {
                                 HStack(spacing: Overlay.step) {
                                     if update.phase == .downloading {
                                         ProgressView(value: update.progress).frame(width: 120)
                                     }
                                     button(action, update)
                                 }
                             })
                }
                if update.offered && !update.plainNotes.isEmpty {
                    GroupRow(title: "", description: update.plainNotes, descriptionLines: 12,
                             trailing: {
                                 Button("Release notes", action: store.openReleaseNotes)
                                     .buttonStyle(.link)
                             })
                }
            }
            if let refusal = store.refusal {
                Text(refusal)
                    .font(.caption)
                    .foregroundColor(.red)
                    .padding(.horizontal, Overlay.horizontalPadding)
                    .padding(.top, Overlay.step)
            }
        }
        .onAppear(perform: store.startWatching)
    }

    private func button(_ action: AppUpdate.Action, _ update: AppUpdate) -> some View {
        Button(update.label(action)) { store.perform(action) }
            .disabled(!update.enabled(action))
            .help(action == .openInstaller ? update.installRefusal ?? "" : "")
    }
}

struct AppUpdateBanner: View {
    @ObservedObject var store: AppUpdateStore

    var body: some View {
        let update = store.update
        GlassPanel {
            HStack(spacing: Overlay.step) {
                Image(systemName: "arrow.down.circle.fill")
                    .foregroundColor(.accentColor)
                VStack(alignment: .leading, spacing: 3) {
                    Text(update.title)
                        .font(.callout.weight(.medium))
                    if update.phase == .downloading {
                        ProgressView(value: update.progress)
                            .frame(width: 200)
                    }
                    if let refusal = store.refusal {
                        Text(refusal)
                            .font(.caption)
                            .foregroundColor(.red)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                }
                ForEach(update.bannerActions, id: \.self) { action in
                    Button(update.label(action)) { store.perform(action) }
                        .disabled(!update.enabled(action))
                        .controlSize(.small)
                }
            }
            .padding(.horizontal, Overlay.unit * 0.9)
            .padding(.vertical, Overlay.unit * 0.5)
        }
        .frame(maxWidth: 460)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Update available")
    }
}
