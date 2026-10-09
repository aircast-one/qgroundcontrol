import SwiftUI

enum SetupState { case NeedsAttention, Done, Neutral, Unavailable }

func setupStateColor(_ state: SetupState, _ theme: Theme) -> Color {
    switch state {
    case .NeedsAttention: theme.aircast.warning
    case .Done: theme.aircast.success
    case .Neutral, .Unavailable: theme.colors.onSurfaceVariant
    }
}

struct SectionHeader: View {
    let text: String
    var image: String = ""
    @Environment(\.theme) private var theme
    @Environment(\.LocalSettingsList) private var band

    var body: some View {
        HStack(spacing: 12) {
            if !image.isBlank {
                Image(sectionImageAsset(image))
                    .resizable()
                    .renderingMode(.template)
                    .scaledToFit()
                    .foregroundStyle(theme.colors.onSurface)
                    .frame(width: 48, height: 24)
            }
            Text(text).font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        .padding(band ? EdgeInsets(top: 10, leading: 16, bottom: 10, trailing: 16) : EdgeInsets(top: 20, leading: 16, bottom: 8, trailing: 16))
        .frame(maxWidth: .infinity, alignment: .leading)
        .background(band ? theme.colors.surfaceContainerHigh : .clear)
        .padding(.top, band ? 12 : 0)
    }
}

func sectionImageAsset(_ image: String) -> String {
    "SetupSections/" + image.removingSuffix(".svg")
}

struct SetupRow: View {
    let title: String
    var status: String = ""
    var state: SetupState = .Neutral
    var onClick: (() -> Void)? = nil
    var summary: [SummaryLine] = []
    var icon: Icon? = nil
    var subtitle: String = ""
    var selected: Bool = false
    @Environment(\.theme) private var theme

    var body: some View {
        if let onClick {
            Button(action: onClick) { row }.buttonStyle(.plain)
        } else {
            row
        }
    }

    private var row: some View {
        HStack(spacing: 16) {
            if let icon {
                Image(icon)
                    .font(.system(size: 18))
                    .foregroundStyle(state == .NeedsAttention ? theme.aircast.warning : theme.colors.onSecondaryContainer)
                    .frame(width: 40, height: 40)
                    .background(state == .NeedsAttention ? theme.aircast.warningContainer : theme.colors.secondaryContainer, in: Circle())
            }
            VStack(alignment: .leading, spacing: 2) {
                Text(title)
                    .font(.bodyLarge)
                    .foregroundStyle(state == .Unavailable ? theme.colors.onSurfaceVariant : theme.colors.onSurface)
                if !subtitle.isBlank {
                    Text(subtitle).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
                }
                let parts = summaryGlanceParts(summary)
                if !parts.isEmpty {
                    glanceText(parts, theme.colors.error).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).lineLimit(1)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if !status.isBlank {
                Text(status).font(.labelMedium).foregroundStyle(setupStateColor(state, theme))
            }
            if onClick != nil {
                Image(.chevronRight).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .padding(.horizontal, 16)
        .padding(.vertical, 12)
        .frame(maxWidth: .infinity, minHeight: 72)
        .background(selected ? theme.colors.surfaceContainerHighest : .clear)
        .contentShape(Rectangle())
    }
}

struct PageHeading {
    let title: String
    let back: () -> Void
    var owner: UUID? = nil
}

@Observable final class PageHeadingSlot {
    var value: PageHeading? = nil
}

extension EnvironmentValues {
    @Entry var LocalTwoPane = false
    @Entry var LocalPageHeading: PageHeadingSlot? = nil
}

struct OverridePageHeading: View {
    let title: String
    let onBack: () -> Void
    @Environment(\.LocalPageHeading) private var host
    @State private var owner = UUID()

    var body: some View {
        Color.clear
            .frame(width: 0, height: 0)
            .onChange(of: title, initial: true) { host?.value = PageHeading(title: title, back: onBack, owner: owner) }
            .onDisappear { if host?.value?.owner == owner { host?.value = nil } }
    }
}

struct AdvancedToggle: View {
    let open: Bool
    var label: String = "Advanced"
    let onToggle: () -> Void

    var body: some View {
        Button(action: onToggle) {
            HStack(spacing: 4) {
                Text(open ? "Hide \(label.lowercasedFirst)" : label)
                Image(open ? .arrowUp : .arrowDropDown)
            }
        }
        .buttonStyle(.borderless)
        .padding(8)
    }
}

struct PageTopBar: View {
    let title: String
    let backLabel: String
    let onBack: () -> Void
    @Environment(\.LocalTwoPane) private var twoPane

    var body: some View {
        HStack(spacing: 4) {
            if twoPane {
                Color.clear.frame(width: 12, height: 1)
            } else {
                Button(action: onBack) { Image(.arrowBack).frame(width: 48, height: 48) }
                    .accessibilityLabel(backLabel)
            }
            Text(title).font(.titleLarge).lineLimit(1)
            Spacer(minLength: 0)
        }
        .padding(.horizontal, 4)
        .frame(maxWidth: .infinity, minHeight: 64)
    }
}

struct EmptyState: View {
    let icon: Icon
    let title: String
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(spacing: 16) {
            Image(icon)
                .font(.system(size: 40))
                .foregroundStyle(theme.colors.onSecondaryContainer)
                .frame(width: 96, height: 96)
                .background(theme.colors.secondaryContainer, in: Circle())
            Text(sentenceCase(title)).font(.titleLarge).multilineTextAlignment(.center)
            if !text.isBlank {
                Text(text).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant).multilineTextAlignment(.center)
            }
        }
        .padding(.horizontal, 32)
        .padding(.vertical, 48)
        .frame(maxWidth: .infinity)
    }
}

struct SummaryLine: Equatable {
    var label: String
    var value: String
    var warn: Bool = false
}

private let UNREAD_VALUES: Set<String> = ["Unknown", "--", ""]
private let GLANCE_SEPARATOR = " \u{00b7} "

private func leadingWord(_ text: String) -> String {
    text.firstIndex(of: " ").map { String(text[..<$0]) } ?? ""
}

func summaryGlanceParts(_ summary: [SummaryLine]) -> [SummaryLine] {
    let read = summary.map {
        SummaryLine(
            label: $0.label.trimmingTrailing(":", " "),
            value: $0.value.trimmed.replacingOccurrences(of: "(\\S)\\(", with: "$1 (", options: .regularExpression),
            warn: $0.warn
        )
    }.filter { !UNREAD_VALUES.contains($0.value) }
    let leads = Set(read.map { leadingWord($0.value) })
    let shared = leads.count == 1 && read.count > 1 ? leads.first.flatMap { $0.isEmpty ? nil : $0 } : nil
    let parts = shared.map { prefix in
        read.map { SummaryLine(label: $0.label, value: "\($0.label) \($0.value.removingPrefix(prefix).trimmed)", warn: $0.warn) }
    } ?? read.enumerated().filter { at, line in !read[..<at].contains { $0.value == line.value } }.map(\.element)
    return parts.map { SummaryLine(label: $0.label, value: sentenceCase($0.value), warn: $0.warn) }
}

func summaryGlance(_ summary: [SummaryLine]) -> String? {
    let parts = summaryGlanceParts(summary)
    return parts.isEmpty ? nil : parts.map(\.value).joined(separator: GLANCE_SEPARATOR)
}

private func glanceText(_ parts: [SummaryLine], _ warn: Color) -> Text {
    let pieces = parts.map { $0.warn ? Text($0.value).foregroundStyle(warn) : Text($0.value) }
    return pieces.dropFirst().reduce(pieces.first ?? Text("")) { line, part in line + Text(GLANCE_SEPARATOR) + part }
}

struct FootNote: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Text(text)
            .font(.bodySmall)
            .foregroundStyle(theme.colors.onSurfaceVariant)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(EdgeInsets(top: 24, leading: 20, bottom: 24, trailing: 20))
    }
}

struct SearchPill: View {
    let value: String
    let onValueChange: (String) -> Void
    let placeholder: String
    var keyboardOptions: SubmitLabel = .done
    var keyboardActions: () -> Void = {}
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 12) {
            Image(.search).foregroundStyle(theme.colors.onSurfaceVariant)
            TextField(placeholder, text: Binding(get: { value }, set: onValueChange))
                .textFieldStyle(.plain)
                .submitLabel(keyboardOptions)
                .onSubmit(keyboardActions)
        }
        .padding(.horizontal, 16)
        .frame(minHeight: 56)
        .background(theme.colors.surfaceContainerHigh, in: Capsule())
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}
