import SwiftUI

struct PreflightScreen: View {
    var ticked: Set<String> = []
    var onTicked: (Set<String>) -> Void = { _ in }
    @QgcPath(PREFLIGHT) private var json
    @State private var collapsed: Set<String> = []
    @State private var passedBefore: Set<String> = []
    @Environment(\.theme) private var theme

    var body: some View {
        let shown = preflight(json).flatMap { $0.groups.isEmpty ? nil : $0 }
        let passedGroups = shown.map { checks in Set(checks.groups.filter { groupPassed($0, ticked) }.map(\.name)) }
        NativeForm {
            if let checks = shown {
                list(checks)
            } else {
                Section {
                    Text("Connect a vehicle to run its preflight checks.")
                        .foregroundStyle(Color.secondary)
                        .onAppear {
                            collapsed = []
                            passedBefore = []
                        }
                }
            }
        }
        .task(id: passedGroups) {
            guard let passedGroups, (try? await Task.sleep(for: .milliseconds(GROUP_COLLAPSE_DELAY_MS))) != nil else { return }
            collapsed = collapsedAfterPass(collapsed, passedBefore, passedGroups)
            passedBefore = passedGroups
        }
    }

    @ViewBuilder
    private func list(_ checks: Preflight) -> some View {
        Section {
            Text(preflightSummary(checks, ticked))
                .foregroundStyle(checks.blocked.isEmpty ? Color.primary : theme.colors.error)
        } header: {
            Text(checklistHeading(checklistIsComplete(checks, ticked)))
        } footer: {
            if !checks.airframe.isBlank { Text(checks.airframe) }
        }
        ForEach(Array(checks.groups.enumerated()), id: \.element.name) { groupIndex, group in
            let open = groupEnabled(checks.groups, groupIndex, ticked)
            let folded = collapsed.contains(group.name)
            Section {
                if !folded {
                    ForEach(group.checks, id: \.name) { check in
                        row(check, open)
                    }
                }
            } header: {
                Button {
                    collapsed = folded ? collapsed.subtracting([group.name]) : collapsed.union([group.name])
                } label: {
                    HStack {
                        Text(groupHeading(group, ticked))
                            .foregroundStyle(open ? theme.colors.primary : Color.secondary)
                            .frame(maxWidth: .infinity, alignment: .leading)
                        Image(systemName: folded ? "chevron.right" : "chevron.down")
                            .font(.footnote.weight(.semibold))
                            .foregroundStyle(Color.secondary)
                    }
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityHint(folded ? "Show checks" : "Hide checks")
            }
        }
        Section {
            Button("Reset checklist") {
                onTicked([])
                collapsed = []
            }
            .disabled(ticked.isEmpty)
        }
    }

    private func row(_ check: PreflightCheck, _ open: Bool) -> some View {
        let isTicked = ticked.contains(check.name)
        return HStack(spacing: Space.s1) {
            switch checkMark(check) {
            case .TICKABLE:
                Toggle(isOn: Binding(get: { isTicked }, set: { on in
                    onTicked(on ? ticked.union([check.name]) : ticked.subtracting([check.name]))
                })) { EmptyView() }
                .toggleStyle(CheckboxToggle())
                .foregroundStyle(theme.colors.primary)
                .disabled(!open)
                .opacity(open ? 1 : DISABLED_ALPHA)
                .accessibilityLabel(check.prompt.ifBlank(check.name))
            case .PASSED:
                Image(systemName: "checkmark")
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .frame(width: 48, height: 48)
                    .accessibilityLabel("Passing")
            case .ATTENTION:
                Image(systemName: "exclamationmark.triangle.fill")
                    .foregroundStyle(check.blocked ? theme.colors.error : theme.colors.onSurfaceVariant)
                    .frame(width: 48, height: 48)
                    .accessibilityLabel(check.blocked ? "Will stop the flight" : "Needs attention")
            }
            VStack(alignment: .leading, spacing: 0) {
                Text(check.prompt.ifBlank(check.name)).font(.bodyMedium)
                Text(checkStatusText(check, isTicked))
                    .font(.labelSmall)
                    .foregroundStyle(check.blocked ? theme.colors.error : theme.colors.onSurfaceVariant)
            }
            .frame(maxWidth: .infinity, alignment: .leading)
        }
    }
}
