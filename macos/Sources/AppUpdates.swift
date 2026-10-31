import AppKit
import Network
import SwiftUI

final class AppUpdateStore: ObservableObject, Probeable {
    static let probeID = "appUpdate"
    static let shared = AppUpdateStore()

    @Published private(set) var update = AppUpdate.none
    @Published private(set) var refusal: String?
    @Published private(set) var prompt: AppUpdate.Prompt?

    private let client = "appUpdate"
    private let paths = NWPathMonitor()
    private var watching = false
    private var openWhenReady = false
    private var expensive = false
    private var constrained = false
    private var opened = ""

    func startWatching() {
        guard !watching else { return }
        watching = true
        paths.pathUpdateHandler = { [weak self] path in
            DispatchQueue.main.async {
                self?.expensive = path.isExpensive
                self?.constrained = path.isConstrained
            }
        }
        paths.start(queue: DispatchQueue(label: "appUpdate.paths"))
        BridgeWatch.watch(client, ["view.appUpdate"]) { [weak self] view in
            self?.apply(AppUpdate(view))
        }
        apply(AppUpdate(Bridge.group("view.appUpdate")))
    }

    func ask(_ action: AppUpdate.Action, from place: AppUpdate.Place) {
        guard let asked = update.prompt(for: action, from: place, expensive: expensive,
                                        constrained: constrained) else { return perform(action) }
        prompt = asked
    }

    func answer(_ asked: AppUpdate.Prompt, confirmed: Bool) {
        prompt = nil
        guard confirmed else { return }
        perform(asked.action)
    }

    func dismissPrompt() {
        prompt = nil
    }

    func setAutoCheck(_ on: Bool) {
        refusal = AppUpdate.refusal(Bridge.invoke("appUpdate.setAutoCheck", [on]))
    }

    func openReleaseNotes() {
        open(update.releaseUrl)
    }

    func openWhatsNew() {
        open(update.whatsNewUrl)
    }

    private func open(_ link: String) {
        guard let url = URL(string: link) else { return }
        NSWorkspace.shared.open(url)
    }

    private func perform(_ action: AppUpdate.Action) {
        openWhenReady = action == .update || (openWhenReady && action != .cancel)
        refusal = action.path.map { AppUpdate.refusal(Bridge.invoke($0)) } ?? installAndQuit()
        openWhenReady = openWhenReady && refusal == nil
    }

    private func apply(_ read: AppUpdate) {
        if read != update { update = read }
        switch read.phase {
        case .ready where openWhenReady && read.bannerShown:
            openWhenReady = false
            ask(.openInstaller, from: .fly)
        case .failed, .idle, .upToDate:
            openWhenReady = false
        default:
            break
        }
    }

    private func installAndQuit() -> String? {
        guard update.enabled(.openInstaller) else { return update.installRefusal ?? AppUpdate.notOpened }
        guard NSWorkspace.shared.open(URL(fileURLWithPath: update.file)) else { return AppUpdate.notOpened }
        opened = update.file
        DispatchQueue.main.async { NSApp.terminate(nil) }
        return nil
    }

    func probeState() -> [String: Any] {
        ["phase": update.phase.rawValue, "current": update.current, "latest": update.latest,
         "title": update.title, "bannerShown": update.bannerShown, "canInstall": update.canInstall,
         "progress": update.progress, "file": update.file, "error": update.error,
         "skipped": update.skipped, "later": update.later, "updatedFrom": update.updatedFrom,
         "updatedShown": update.updatedShown, "autoCheck": update.autoCheck,
         "refusal": refusal ?? "", "prompt": prompt?.message ?? "",
         "expensive": expensive, "constrained": constrained,
         "openWhenReady": openWhenReady, "opened": opened, "watching": watching]
    }
}

extension View {
    func appUpdatePrompt(_ store: AppUpdateStore, at place: AppUpdate.Place) -> some View {
        alert(store.prompt?.title ?? "",
              isPresented: Binding(get: { store.prompt?.place == place },
                                   set: { if !$0 { store.dismissPrompt() } }),
              presenting: store.prompt) { asked in
            Button(asked.confirm) { store.answer(asked, confirmed: true) }
            Button(asked.decline, role: .cancel) { store.answer(asked, confirmed: false) }
        } message: { asked in
            Text(asked.message)
        }
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
                                 Button(AppUpdate.tryAgain) { store.ask(update.retry, from: .settings) }
                             })
                }
                if !update.offerActions.isEmpty {
                    GroupRow(title: update.title, description: update.offerDetail, titleLines: 2,
                             descriptionLines: 2,
                             trailing: {
                                 HStack(spacing: Overlay.step) {
                                     if update.phase == .downloading {
                                         ProgressView(value: update.progress).frame(width: 120)
                                     }
                                     ForEach(update.offerActions, id: \.self) { button($0, update) }
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
        .appUpdatePrompt(store, at: .settings)
    }

    private func button(_ action: AppUpdate.Action, _ update: AppUpdate) -> some View {
        Button(update.label(action)) { store.ask(action, from: .settings) }
            .fixedSize()
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
                    Button(update.label(action)) { store.ask(action, from: .fly) }
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

struct AppUpdatedBanner: View {
    @ObservedObject var store: AppUpdateStore

    var body: some View {
        GlassPanel {
            HStack(spacing: Overlay.step) {
                Image(systemName: "checkmark.circle.fill")
                    .foregroundColor(.green)
                Text(store.update.updatedTitle)
                    .font(.callout.weight(.medium))
                Button(AppUpdate.whatsNew, action: store.openWhatsNew)
                    .controlSize(.small)
                Button(AppUpdate.dismiss) { store.ask(.dismissUpdated, from: .fly) }
                    .controlSize(.small)
            }
            .padding(.horizontal, Overlay.unit * 0.9)
            .padding(.vertical, Overlay.unit * 0.5)
        }
        .frame(maxWidth: 460)
        .accessibilityElement(children: .contain)
        .accessibilityLabel("Update installed")
    }
}
