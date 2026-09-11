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
    static let blocked = Color.orange

    // A single-sample collision still has to be visible, so a run narrower than this is drawn
    // at this width rather than as a hairline.
    static let collisionMark: CGFloat = 5
    static let markerType: Double = 8
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
    var titleLines = 1
    var descriptionLines = 1
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
                        Text(title)
                            .lineLimit(titleLines)
                            .fixedSize(horizontal: false, vertical: titleLines > 1)
                    }
                    if !description.isEmpty {
                        Text(description)
                            .font(.caption)
                            .foregroundColor(.secondary)
                            .lineLimit(descriptionLines)
                            .fixedSize(horizontal: false, vertical: descriptionLines > 1)
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
         showSeparator: Bool = true, current: Bool = false, titleLines: Int = 1,
         @ViewBuilder trailing: () -> Trailing) {
        self.init(title: title, description: description, value: value,
                  showSeparator: showSeparator, current: current, titleLines: titleLines,
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
         showSeparator: Bool = true, current: Bool = false, titleLines: Int = 1,
         descriptionLines: Int = 1) {
        self.init(title: title, description: description, value: value,
                  showSeparator: showSeparator, current: current, titleLines: titleLines,
                  descriptionLines: descriptionLines,
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

struct Tile: View {
    let symbol: String
    let colour: Color

    var body: some View {
        RoundedRectangle(cornerRadius: 6, style: .continuous)
            .fill(colour)
            .frame(width: 22, height: 22)
            .overlay(
                Image(systemName: symbol)
                    .font(.system(size: 12, weight: .semibold))
                    .foregroundColor(.white))
    }
}

struct StatusPill: View {
    let text: String
    let good: Bool

    var body: some View {
        HStack(spacing: 5) {
            Image(systemName: good ? "checkmark.circle.fill" : "exclamationmark.triangle.fill")
            Text(text)
        }
        .font(.callout.weight(.medium))
        .foregroundColor(good ? .green : .orange)
        .padding(.horizontal, Overlay.step)
        .padding(.vertical, 4)
        .background((good ? Color.green : Color.orange).opacity(0.15))
        .clipShape(Capsule())
    }
}

struct SidebarRow: View {
    let title: String
    let symbol: String
    let colour: Color
    var badge = false

    var body: some View {
        HStack(spacing: Overlay.step) {
            Tile(symbol: symbol, colour: colour)
            Text(title)
            Spacer(minLength: 0)
            if badge {
                Circle().fill(Color.red).frame(width: 8, height: 8)
            }
        }
        .padding(.vertical, 1)
    }
}

struct ContentHeightKey: PreferenceKey {
    static var defaultValue: CGFloat = 0

    static func reduce(value: inout CGFloat, nextValue: () -> CGFloat) {
        value = max(value, nextValue())
    }
}

struct ContentSizeKey: PreferenceKey {
    static var defaultValue = CGSize.zero

    static func reduce(value: inout CGSize, nextValue: () -> CGSize) {
        let next = nextValue()
        if next != .zero { value = next }
    }
}

extension View {
    // Row heights vary once a label wraps, so an estimate of rows times row height
    // clipped the last one. Measure instead.
    func measuringHeight(into height: Binding<CGFloat>) -> some View {
        background(GeometryReader { proxy in
            Color.clear.preference(key: ContentHeightKey.self, value: proxy.size.height)
        })
        .onPreferenceChange(ContentHeightKey.self) { height.wrappedValue = $0 }
    }
    func measuringSize(into size: Binding<CGSize>) -> some View {
        background(GeometryReader { proxy in
            Color.clear.preference(key: ContentSizeKey.self, value: proxy.size)
        })
        .onPreferenceChange(ContentSizeKey.self) { size.wrappedValue = $0 }
    }
    func writeFailureAlert(title: String = "That change was not accepted",
                           _ reports: Binding<String?>...) -> some View {
        let shown = reports.compactMap(\.wrappedValue).first
        let clear = { reports.forEach { $0.wrappedValue = nil } }
        return alert(title,
                     isPresented: Binding(get: { shown != nil }, set: { if !$0 { clear() } })) {
            Button("OK", action: clear)
        } message: {
            Text(shown ?? "")
        }
    }
}

struct MapScaleView: View {
    let bar: MapScaleBar

    var body: some View {
        if !bar.text.isEmpty {
            VStack(alignment: .leading, spacing: 2) {
                Text(bar.text)
                    .font(.caption2.monospacedDigit())
                    .foregroundColor(.primary)
                ZStack(alignment: .leading) {
                    Rectangle()
                        .fill(Color.primary.opacity(0.35))
                        .frame(width: MissionMap.scalePixels * bar.fraction, height: 3)
                    HStack {
                        Rectangle().fill(Color.primary).frame(width: 2, height: 9)
                        Spacer(minLength: 0)
                        Rectangle().fill(Color.primary).frame(width: 2, height: 9)
                    }
                    .frame(width: MissionMap.scalePixels * bar.fraction)
                }
            }
            .padding(.horizontal, 6)
            .padding(.vertical, 4)
        }
    }
}
