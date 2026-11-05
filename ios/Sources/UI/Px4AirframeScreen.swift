import SwiftUI

let PX4_AIRFRAME_VIEW = "view.px4Airframe"
let PX4_AIRFRAME_SCREEN = "px4Airframe"
let PX4_AIRFRAME_APPLY = "px4Airframe.apply"
let PX4_AIRFRAME_RESET = "px4Airframe.reset"

struct AirframeChoice: Equatable {
    let name: String
    let autostartId: Int64
}

struct AirframeGroup: Equatable {
    let name: String
    let airframes: [AirframeChoice]
    var image: String = ""
}

func airframeImageAsset(_ image: String) -> String? {
    image.isBlank ? nil : "Airframe/" + image.removingSuffix(".svg")
}

struct Px4Airframes: Equatable {
    let autostartId: Int64
    let custom: Bool
    let customText: String
    let heading: String
    let currentType: String?
    let currentIndex: Int
    let applyTitle: String
    let applyText: String
    let groups: [AirframeGroup]
}

func px4Airframes(_ view: JSON?) -> Px4Airframes? {
    guard let read = view, read["available"].bool else { return nil }
    return Px4Airframes(
        autostartId: read["autostartId"].int64 ?? 0,
        custom: read["custom"].bool,
        customText: read["customText"].string,
        heading: read["heading"].string,
        currentType: read["currentType"].string.isEmpty ? nil : read["currentType"].string,
        currentIndex: read["currentIndex"].int(0),
        applyTitle: read["applyTitle"].string,
        applyText: read["applyText"].string.replacingOccurrences(of: "<br>", with: "\n"),
        groups: read["types"].objects.map { type in
            AirframeGroup(
                name: type["name"].string,
                airframes: type["airframes"].objects.map { AirframeChoice(name: $0["name"].string, autostartId: $0["autostartId"].int64 ?? 0) },
                image: type["image"].string
            )
        }
    )
}

struct AirframeSelection: Equatable {
    let group: String
    let index: Int
}

func initialSelection(_ read: Px4Airframes) -> AirframeSelection? {
    read.currentType.map { AirframeSelection(group: $0, index: read.currentIndex) }
}

struct Px4AirframeScreen: View {
    @Environment(\.theme) private var theme
    @QgcPath(PX4_AIRFRAME_VIEW) private var view
    @State private var refusal: String?
    @State private var confirming = false
    @State private var selection: AirframeSelection?

    var body: some View {
        let read = px4Airframes(view)
        ZStack(alignment: .topLeading) {
            Color.clear
            if let read {
                if read.custom {
                    custom(read)
                } else {
                    screen(read)
                }
            } else {
                NativeForm {
                    Section { Text("This vehicle has no SYS_AUTOSTART and SYS_AUTOCONFIG.").foregroundStyle(Color.secondary) }
                }
            }
        }
        .onChange(of: read?.autostartId, initial: true) { selection = read.flatMap(initialSelection) }
    }

    private func custom(_ read: Px4Airframes) -> some View {
        NativeForm {
            Section {
                Text(read.customText)
            }
            Section {
                Button("Reset", role: .destructive) { act(PX4_AIRFRAME_RESET) }
            }
            if let refusal {
                Section {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
            }
        }
    }

    private func screen(_ read: Px4Airframes) -> some View {
        let chosen = selection.flatMap { picked in
            read.groups.first { $0.name == picked.group }.flatMap { $0.airframes.indices.contains(picked.index) ? $0.airframes[picked.index] : nil }
        }
        return NativeForm {
            if let refusal {
                Section {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
            }
            Section {
                ForEach(read.groups, id: \.name) { group in
                    let picked = selection?.group == group.name
                    AirframeGroupRow(group: group, picked: picked, index: picked ? selection?.index ?? 0 : 0) { index in
                        selection = AirframeSelection(group: group.name, index: index)
                    }
                }
            } header: {
                if !read.heading.isBlank { Text(read.heading).textCase(nil) }
            }
        }
        .pagePrimaryAction(read.applyTitle, enabled: chosen != nil) { confirming = true }
        .alert(read.applyTitle, isPresented: Binding(get: { confirming && chosen != nil }, set: { if !$0 { confirming = false } })) {
            Button("Apply") {
                confirming = false
                if let chosen { act(PX4_AIRFRAME_APPLY, chosen.autostartId) }
            }
            Button("Cancel", role: .cancel) { confirming = false }
        } message: {
            Text(read.applyText)
        }
    }

    private func act(_ path: String, _ args: Any...) {
        Task { refusal = await offMain { Qgc.refusalOf(path, arguments: args) } }
    }
}

private let AIRFRAME_IMAGE_WIDTH: CGFloat = 88
private let AIRFRAME_IMAGE_HEIGHT: CGFloat = 56

// One airframe type as a form row: its picture, its name, the airframe of that type to use, and a checkmark on the
// chosen type. Tapping the row picks the type with the airframe shown.
private struct AirframeGroupRow: View {
    let group: AirframeGroup
    let picked: Bool
    let index: Int
    let onPick: (Int) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: Space.s3) {
            if let asset = airframeImageAsset(group.image) {
                Image(asset)
                    .resizable()
                    .scaledToFit()
                    .frame(width: AIRFRAME_IMAGE_WIDTH, height: AIRFRAME_IMAGE_HEIGHT)
                    .accessibilityLabel(group.name)
            }
            VStack(alignment: .leading, spacing: Space.s1) {
                Text(group.name)
                Menu {
                    ForEach(Array(group.airframes.enumerated()), id: \.offset) { at, frame in
                        Button(frame.name) { onPick(at) }
                    }
                } label: {
                    Text(group.airframes.indices.contains(index) ? group.airframes[index].name : "")
                        .font(.subheadline)
                        .multilineTextAlignment(.leading)
                }
                .fixedSize()
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if picked {
                Image(systemName: "checkmark")
                    .fontWeight(.semibold)
                    .foregroundStyle(theme.colors.primary)
                    .accessibilityHidden(true)
            }
        }
        .contentShape(Rectangle())
        .onTapGesture { onPick(index) }
        .accessibilityAddTraits(picked ? .isSelected : [])
    }
}
