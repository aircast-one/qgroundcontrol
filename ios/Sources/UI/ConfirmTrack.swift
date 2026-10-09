import SwiftUI

func sentIsStillShowing(_ name: String?, _ snapshotAtSend: String?, _ live: String?) -> Bool {
    name != nil && snapshotAtSend != nil && snapshotAtSend == live
}

func sentText(_ name: String) -> String { "Sent \u{00B7} \(name)" }

let LAND_FROM = "view.instruments(altitudeRelative)"

func landFrom(_ view: JSON?) -> (String, String)? {
    guard let item = view?["items"][0], item.object != nil, !item["missing"].bool else { return nil }
    let value = item["value"].string
    guard !value.isBlank, value != "\u{2014}" else { return nil }
    return (value, item["units"].string)
}

let MISSION_ACTIONS: Set<String> = ["startMission", "continueMission"]

let MISSION_ON_DRONE = "The mission already on the drone"

struct MissionIdentity: Equatable {
    var title: String
    var facts: String
    var warning: String?
}

func missionIdentity(_ plan: JSON?, _ summary: JSON?) -> MissionIdentity {
    guard let plan, plan["hasMissionItems"].bool else {
        return MissionIdentity(title: MISSION_ON_DRONE, facts: "", warning: "It isn't open in Plan, so its route can't be shown here.")
    }
    if plan["dirty"].bool {
        return MissionIdentity(title: MISSION_ON_DRONE, facts: "", warning: "\(planTitle(plan["file"].string)) has changes that aren't on the drone. Upload it first to fly it.")
    }
    return MissionIdentity(
        title: planTitle(plan["file"].string),
        facts: [
            planStatusText(plan).isBlank ? nil : planStatusText(plan),
            summaryRow(summary, "Distance"),
            summaryRow(summary, "Time"),
        ].compactMap { $0 }.joined(separator: " \u{00B7} "),
        warning: nil
    )
}

private struct MissionIdentityCard: View {
    @QgcPath("view.plan") private var planJson
    @QgcPath("view.missionSummary") private var summaryJson
    @Environment(\.theme) private var theme

    var body: some View {
        let mission = missionIdentity(planJson, summaryJson)
        VStack(alignment: .leading, spacing: Space.s1) {
            Text(mission.title).font(.titleMedium)
            if !mission.facts.isBlank {
                Text(mission.facts).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if let warning = mission.warning {
                Text(warning).font(.bodyMedium).foregroundStyle(theme.aircast.warning)
            }
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s3)
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
    }
}

struct ConfirmTrack: View {
    let action: GuidedAction
    let onSent: () -> Void
    let onCancel: () -> Void
    @Binding private var optionChecked: Bool
    @QgcPath private var fromJson: JSON?
    @Environment(\.theme) private var theme

    init(action: GuidedAction, optionChecked: Binding<Bool>, onSent: @escaping () -> Void, onCancel: @escaping () -> Void) {
        self.action = action
        _optionChecked = optionChecked
        self.onSent = onSent
        self.onCancel = onCancel
        _fromJson = QgcPath(action.offerId == "land" ? LAND_FROM : nil)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text(sentenceCase(action.name)).font(.titleLarge)
            if !action.confirm.isBlank {
                Text(action.confirm).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if let offerId = action.offerId, MISSION_ACTIONS.contains(offerId) {
                MissionIdentityCard()
            }
            if action.offerId == "land", let from = landFrom(fromJson) {
                FactTile(label: "FROM", value: from.0, units: from.1)
            }
            if let option = action.option {
                Toggle(isOn: $optionChecked) {
                    Text(option.label).font(.labelLarge).frame(maxWidth: .infinity, alignment: .leading)
                }
                .toggleStyle(CheckboxToggle())
            }
            HoldToConfirm(label: holdLabel(action.name), destructive: action.destructive) {
                if let option = action.option { option.run(optionChecked) } else { action.run() }
                onSent()
            }
            .padding(.top, Space.s2)
            Button("Cancel", action: onCancel)
                .buttonStyle(.borderless)
                .frame(maxWidth: .infinity)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

let CHECKBOX_TOUCH_SIZE: CGFloat = 48

struct CheckboxToggle: ToggleStyle {
    var size: CGFloat = CHECKBOX_TOUCH_SIZE

    func makeBody(configuration: Configuration) -> some View {
        Button { configuration.isOn.toggle() } label: {
            HStack(spacing: Space.s3) {
                Image(configuration.isOn ? .checkBox : .checkBoxOutline)
                    .font(.title3)
                    .frame(width: size, height: size)
                configuration.label
            }
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
    }
}

struct HoldOrCancel: View {
    let label: String
    let onConfirm: () -> Void
    let onCancel: () -> Void

    var body: some View {
        VStack(spacing: Space.s1) {
            HoldToConfirm(label: holdLabel(label), onConfirm: onConfirm)
                .padding(.top, Space.s2)
            Button("Cancel", action: onCancel).buttonStyle(.borderless)
        }
        .frame(maxWidth: .infinity)
    }
}

struct SentNotice: View {
    let name: String
    let onDismiss: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Text(sentText(name))
            .font(.bodyMedium)
            .fontWeight(.bold)
            .foregroundStyle(theme.colors.primary)
            .frame(maxWidth: .infinity, alignment: .leading)
            .contentShape(Rectangle())
            .onTapGesture(perform: onDismiss)
    }
}

private struct FactTile: View {
    let label: String
    let value: String
    let units: String
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(label).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            HStack(alignment: .lastTextBaseline, spacing: Space.s1) {
                Text(value).font(.titleLarge)
                if !units.isBlank {
                    Text(units).font(.labelMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
        }
        .padding(.horizontal, Space.s3)
        .padding(.vertical, 6)
        .background(theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
    }
}
