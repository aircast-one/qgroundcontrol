import SwiftUI

enum Overlay {
    static let unit: CGFloat = 16
    static let step: CGFloat = 7

    static let cardRadius = unit * 0.9
    static let panelRadius = unit * 1.1
    static let rowMinHeight = unit * 2.6
    static let horizontalPadding = step * 1.5
    static let verticalPadding = unit * 0.45
    static let sealSize = unit * 1.5
    static let gutter = unit * 0.75

    static let card = Color.primary.opacity(0.11)
    static let separator = Color.primary.opacity(0.09)
    static let value = Color.primary.opacity(0.6)
    static let chevron = Color.primary.opacity(0.4)
    static let border = Color.primary.opacity(0.15)
    static let currentRow = Color.accentColor.opacity(0.22)

    static let mission = Color.accentColor
    static let launch = Color.green
    static let rally = Color.green
    static let fence = Color.orange
    static let vehicle = Color.red
}

struct GlassPanel<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        content
            .background(.regularMaterial, in: RoundedRectangle(cornerRadius: Overlay.panelRadius))
            .overlay(
                RoundedRectangle(cornerRadius: Overlay.panelRadius)
                    .strokeBorder(Overlay.border, lineWidth: 1))
            .shadow(color: .black.opacity(0.22), radius: 12, y: 4)
    }
}

struct GroupCard<Content: View>: View {
    @ViewBuilder var content: Content

    var body: some View {
        VStack(spacing: 0) { content }
            .background(Overlay.card)
            .clipShape(RoundedRectangle(cornerRadius: Overlay.cardRadius))
    }
}

struct SectionLabel: View {
    let text: String

    var body: some View {
        Text(text.uppercased())
            .font(.caption2.weight(.semibold))
            .kerning(0.6)
            .foregroundColor(.secondary)
            .padding(.horizontal, Overlay.horizontalPadding)
            .padding(.bottom, Overlay.unit * 0.25)
    }
}

struct Seal: View {
    let label: String
    let colour: Color
    var rounded = false

    var body: some View {
        RoundedRectangle(cornerRadius: rounded ? Overlay.sealSize * 0.3 : Overlay.sealSize / 2)
            .fill(colour)
            .frame(width: Overlay.sealSize, height: Overlay.sealSize)
            .overlay(
                Text(label)
                    .font(.caption2.weight(.bold))
                    .foregroundColor(.white))
    }
}

struct GroupRow<Leading: View, Trailing: View>: View {
    let title: String
    var description = ""
    var value = ""
    var showSeparator = true
    var current = false
    @ViewBuilder var leading: Leading
    @ViewBuilder var trailing: Trailing

    var body: some View {
        VStack(spacing: 0) {
            if showSeparator {
                Rectangle()
                    .fill(Overlay.separator)
                    .frame(height: 1)
                    .padding(.leading, Overlay.horizontalPadding)
            }

            HStack(spacing: Overlay.step) {
                leading

                VStack(alignment: .leading, spacing: Overlay.unit * 0.1) {
                    if !title.isEmpty {
                        Text(title).lineLimit(1)
                    }
                    if !description.isEmpty {
                        Text(description)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(1)
                    }
                }

                Spacer(minLength: Overlay.step)

                if !value.isEmpty {
                    Text(value)
                        .font(.caption.monospacedDigit())
                        .foregroundColor(Overlay.value)
                }
                trailing
            }
            .padding(.horizontal, Overlay.horizontalPadding)
            .padding(.vertical, Overlay.verticalPadding)
            .frame(minHeight: Overlay.rowMinHeight)
        }
        .background(current ? Overlay.currentRow : .clear)
    }
}

extension GroupRow where Leading == EmptyView {
    init(title: String, description: String = "", value: String = "",
         showSeparator: Bool = true, current: Bool = false,
         @ViewBuilder trailing: () -> Trailing) {
        self.init(title: title, description: description, value: value,
                  showSeparator: showSeparator, current: current,
                  leading: { EmptyView() }, trailing: trailing)
    }
}

extension GroupRow where Trailing == EmptyView {
    init(title: String, description: String = "", value: String = "",
         showSeparator: Bool = true, current: Bool = false,
         @ViewBuilder leading: () -> Leading) {
        self.init(title: title, description: description, value: value,
                  showSeparator: showSeparator, current: current,
                  leading: leading, trailing: { EmptyView() })
    }
}

extension GroupRow where Leading == EmptyView, Trailing == EmptyView {
    init(title: String, description: String = "", value: String = "",
         showSeparator: Bool = true, current: Bool = false) {
        self.init(title: title, description: description, value: value,
                  showSeparator: showSeparator, current: current,
                  leading: { EmptyView() }, trailing: { EmptyView() })
    }
}

struct EmptyStateRow: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.callout)
            .foregroundColor(.secondary)
            .fixedSize(horizontal: false, vertical: true)
            .frame(maxWidth: .infinity, alignment: .leading)
            .padding(.horizontal, Overlay.horizontalPadding)
            .padding(.vertical, Overlay.unit * 0.7)
    }
}
