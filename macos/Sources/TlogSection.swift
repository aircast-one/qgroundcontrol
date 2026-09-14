import SwiftUI
import UniformTypeIdentifiers

final class TlogStore: ObservableObject, Probeable {
    static let probeID = "tlog"

    @Published private(set) var summary: TlogSummary?
    @Published private(set) var problem = ""
    @Published private(set) var chosen = ""

    func choose() {
        let panel = NSOpenPanel()
        panel.canChooseDirectories = false
        panel.allowsMultipleSelection = false
        panel.message = "Choose a telemetry log to inspect"
        if let tlog = UTType(filenameExtension: "tlog") {
            panel.allowedContentTypes = [tlog]
        }
        guard panel.runModal() == .OK, let url = panel.url else { return }
        load(url.path)
    }

    func load(_ path: String) {
        chosen = path
        problem = ""
        summary = TlogSummary(Bridge.group("view.tlog(\(path))"))
        if summary == nil {
            problem = "The core could not read that file."
        }
    }

    func probeState() -> [String: Any] {
        ["chosen": chosen, "problem": problem,
         "readable": summary?.readable ?? false,
         "frames": summary?.frames ?? -1,
         "empty": summary?.empty ?? false,
         "whollyUndecodable": summary?.whollyUndecodable ?? false,
         "span": summary?.spanText ?? "", "size": summary?.sizeText ?? "",
         "kinds": summary?.messageKinds ?? 0,
         "busiest": summary?.busiest?.name ?? ""]
    }

    func probeInvoke(action: String, args: [String: String]) -> [String: Any] {
        switch action {
        case "load":
            guard let path = args["path"], !path.isEmpty else {
                return ["ok": false, "error": "load needs a path"]
            }
            load(path)
        default:
            return ["ok": false, "error": "unknown action \(action)"]
        }
        return ["ok": true, "state": probeState()]
    }
}

struct TlogView: View {
    @ObservedObject var store: TlogStore

    var body: some View {
        SetupPageBody(title: "Telemetry Log") {
            VStack(alignment: .leading, spacing: 0) {
                HStack {
                    Button("Choose a Log…", action: store.choose)
                    if !store.chosen.isEmpty {
                        Text((store.chosen as NSString).lastPathComponent)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                            .truncationMode(.middle)
                    }
                    Spacer()
                }
                .padding(.bottom, 10)

                if !store.problem.isEmpty {
                    Text(store.problem).foregroundColor(.red)
                } else if let summary = store.summary {
                    read(summary)
                } else {
                    Text("No log chosen yet.").foregroundColor(.secondary)
                }
            }
        }
    }

    @ViewBuilder
    private func read(_ summary: TlogSummary) -> some View {
        if !summary.readable {
            Text("That file could not be opened.").foregroundColor(.secondary)
        } else if summary.whollyUndecodable {
            Text("The file opened and none of it decoded as MAVLink.")
                .foregroundColor(.secondary)
        } else if summary.empty {
            Text("The file opened and holds no frames.").foregroundColor(.secondary)
        } else {
            GroupCard {
                row("Size", summary.sizeText)
                Divider()
                row("Duration", summary.spanText)
                Divider()
                row("Frames", String(summary.frames))
                Divider()
                row("Message kinds", String(summary.messageKinds))
                if let busiest = summary.busiest {
                    Divider()
                    row("Busiest", "\(busiest.name)  \(busiest.count)")
                }
                if summary.undecodable > 0 {
                    Divider()
                    row("Undecodable", String(summary.undecodable))
                }
            }
        }
    }

    private func row(_ label: String, _ value: String) -> some View {
        HStack {
            Text(label).foregroundColor(.secondary)
            Spacer()
            Text(value.isEmpty ? "Not reported" : value)
                .foregroundColor(value.isEmpty ? .secondary : .primary)
        }
        .padding(.vertical, 5)
        .padding(.horizontal, 4)
    }
}
