import SwiftUI

struct PreflightScreen: View {
    var ticked: Set<String> = []
    var onTicked: (Set<String>) -> Void = { _ in }
    @QgcPath(PREFLIGHT) private var json
    @State private var collapsed: Set<String> = []
    @State private var passedBefore: Set<String> = []
    @Environment(\.theme) private var theme

    var body: some View {
        if let checks = preflight(json), !checks.groups.isEmpty {
            list(checks)
        } else {
            Text("Connect a vehicle to run its preflight checks.")
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(Space.s4)
                .onAppear {
                    collapsed = []
                    passedBefore = []
                }
        }
    }

    private func list(_ checks: Preflight) -> some View {
        let passedGroups = Set(checks.groups.filter { groupPassed($0, ticked) }.map(\.name))
        return VStack(alignment: .leading, spacing: 0) {
            HStack {
                Text(checklistHeading(checklistIsComplete(checks, ticked)))
                    .font(.titleMedium)
                    .frame(maxWidth: .infinity, alignment: .leading)
                Button("Reset") {
                    onTicked([])
                    collapsed = []
                }
                .buttonStyle(.borderless)
                .disabled(ticked.isEmpty)
            }
            .padding(.leading, Space.s4)
            .padding(.trailing, Space.s2)
            Text(preflightSummary(checks, ticked))
                .font(.titleSmall)
                .foregroundStyle(checks.blocked.isEmpty ? theme.colors.onSurface : theme.colors.error)
                .padding(.horizontal, Space.s4)
                .padding(.vertical, Space.s2)
            if !checks.airframe.isBlank {
                Text(checks.airframe)
                    .font(.bodySmall)
                    .foregroundStyle(theme.colors.onSurfaceVariant)
                    .padding(.horizontal, Space.s4)
            }
            Divider().padding(.top, Space.s2)
            ScrollView {
                LazyVStack(alignment: .leading, spacing: 0) {
                    ForEach(Array(checks.groups.enumerated()), id: \.element.name) { groupIndex, group in
                        let open = groupEnabled(checks.groups, groupIndex, ticked)
                        let folded = collapsed.contains(group.name)
                        Text(groupHeading(group, ticked))
                            .font(.labelLarge)
                            .foregroundStyle(open ? theme.colors.primary : theme.colors.onSurfaceVariant)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .padding(.leading, Space.s4)
                            .padding(.top, Space.s4)
                            .padding(.bottom, Space.s1)
                            .contentShape(Rectangle())
                            .onTapGesture {
                                collapsed = folded ? collapsed.subtracting([group.name]) : collapsed.union([group.name])
                            }
                        if !folded {
                            ForEach(group.checks, id: \.name) { check in
                                row(check, open)
                            }
                        }
                    }
                }
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .task(id: passedGroups) {
            guard (try? await Task.sleep(for: .milliseconds(GROUP_COLLAPSE_DELAY_MS))) != nil else { return }
            collapsed = collapsedAfterPass(collapsed, passedBefore, passedGroups)
            passedBefore = passedGroups
        }
    }

    private func row(_ check: PreflightCheck, _ open: Bool) -> some View {
        let isTicked = ticked.contains(check.name)
        return HStack(spacing: Space.s1) {
            switch checkMark(check) {
            case .TICKABLE:
                Button {
                    onTicked(isTicked ? ticked.subtracting([check.name]) : ticked.union([check.name]))
                } label: {
                    Image(systemName: isTicked ? "checkmark.square.fill" : "square")
                        .font(.title3)
                        .foregroundStyle(theme.colors.primary)
                        .frame(width: 48, height: 48)
                }
                .buttonStyle(.plain)
                .disabled(!open)
                .opacity(open ? 1 : 0.38)
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
        .padding(.horizontal, Space.s3)
        .padding(.vertical, 6)
    }
}
