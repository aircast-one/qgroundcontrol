import SwiftUI

private let OPAQUE_HEX_DIGITS = 6
private let ALPHA_HEX_DIGITS = 8
private let RGB_MASK: UInt32 = 0xFFFFFF
private let ALPHA_SHIFT: UInt32 = 24
private let CHANNEL_MAX = 255.0

func hexColour(_ hex: String) -> Color {
    let digits = hex.removingPrefix("#")
    guard [OPAQUE_HEX_DIGITS, ALPHA_HEX_DIGITS].contains(digits.count), let value = UInt32(digits, radix: 16) else { return .gray }
    return digits.count == ALPHA_HEX_DIGITS ? Color(hex: value & RGB_MASK).opacity(Double(value >> ALPHA_SHIFT) / CHANNEL_MAX) : Color(hex: value)
}

struct PlanDialog<Content: View, Buttons: View>: View {
    let title: String
    let onDismiss: () -> Void
    @ViewBuilder let content: () -> Content
    @ViewBuilder let buttons: () -> Buttons
    @State private var titleHeight: CGFloat?
    @State private var contentHeight: CGFloat?
    @State private var buttonsHeight: CGFloat?

    private var fittedHeight: CGFloat? {
        guard let titleHeight, let contentHeight, let buttonsHeight else { return nil }
        return Space.s6 + titleHeight + Space.s4 + contentHeight + Space.s4 + buttonsHeight + Space.s6
    }

    var body: some View {
        AircastSheet(onDismissRequest: onDismiss, fittedHeight: fittedHeight) {
            VStack(alignment: .leading, spacing: Space.s4) {
                Text(title).font(.headlineSmall).padding(.horizontal, Space.s6)
                    .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { titleHeight = $0 }
                ScrollView {
                    content()
                        .frame(maxWidth: .infinity, alignment: .leading)
                        .padding(.horizontal, Space.s6)
                        .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { contentHeight = $0 }
                }
                HStack(spacing: Space.s2) {
                    Spacer()
                    buttons()
                }
                .buttonStyle(.text)
                .padding(.horizontal, Space.s6)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { buttonsHeight = $0 }
            }
            .padding(.bottom, Space.s6)
        }
    }
}

struct PlanTextField: View {
    let label: String
    @Binding var text: String
    var suffix: String = ""
    var placeholder: String = ""
    var width: CGFloat? = nil
    var enabled: Bool = true
    var isError: Bool = false
    var onDone: () -> Void = {}
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label)
                .font(.labelSmall)
                .foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            HStack(spacing: 4) {
                TextField(placeholder, text: $text)
                    .font(.bodySmall)
                    .keyboardType(.numbersAndPunctuation)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.done)
                    .onSubmit(onDone)
                if !suffix.isBlank {
                    Text(suffix).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(isError ? theme.colors.error : theme.colors.outline, lineWidth: 1))
        }
        .frame(width: width)
        .disabled(!enabled)
        .opacity(enabled ? 1 : DISABLED_ALPHA)
    }
}

struct PlanMenuField: View {
    let label: String
    let value: String
    var width: CGFloat? = nil
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.labelSmall).foregroundStyle(theme.colors.onSurfaceVariant)
            HStack {
                Text(value).font(.bodySmall).lineLimit(1).frame(maxWidth: .infinity, alignment: .leading)
                Image(.arrowDropDown).font(.labelSmall)
            }
            .foregroundStyle(theme.colors.onSurface)
            .padding(.horizontal, 10)
            .padding(.vertical, 8)
            .overlay(RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(theme.colors.outline, lineWidth: 1))
        }
        .frame(width: width)
        .frame(maxWidth: width == nil ? .infinity : nil)
    }
}

private func flowLineWidth(_ line: [Int], _ sizes: [CGSize], _ spacing: CGFloat) -> CGFloat {
    line.map { sizes[$0].width }.reduce(0, +) + CGFloat(max(line.count - 1, 0)) * spacing
}

private func flowLineHeight(_ line: [Int], _ sizes: [CGSize]) -> CGFloat {
    line.map { sizes[$0].height }.max() ?? 0
}

func flowLines(_ sizes: [CGSize], _ maxWidth: CGFloat, spacing: CGFloat) -> [[Int]] {
    sizes.indices.reduce([[Int]]()) { lines, index in
        guard let last = lines.last, flowLineWidth(last + [index], sizes, spacing) <= maxWidth else { return lines + [[index]] }
        return lines.dropLast() + [last + [index]]
    }
}

func flowSize(_ sizes: [CGSize], _ width: CGFloat?, spacing: CGFloat, lineSpacing: CGFloat) -> CGSize {
    let lines = flowLines(sizes, width ?? .infinity, spacing: spacing)
    let gaps = CGFloat(max(lines.count - 1, 0)) * lineSpacing
    return CGSize(
        width: width ?? lines.map { flowLineWidth($0, sizes, spacing) }.max() ?? 0,
        height: lines.map { flowLineHeight($0, sizes) }.reduce(0, +) + gaps
    )
}

func flowPositions(_ sizes: [CGSize], _ width: CGFloat, spacing: CGFloat, lineSpacing: CGFloat, centred: Bool) -> [CGPoint] {
    let lines = flowLines(sizes, width, spacing: spacing)
    let heights = lines.map { flowLineHeight($0, sizes) }
    let tops = heights.indices.map { at in heights[..<at].reduce(0, +) + CGFloat(at) * lineSpacing }
    return lines.indices.flatMap { row in
        let line = lines[row]
        let start = centred ? (width - flowLineWidth(line, sizes, spacing)) / 2 : 0
        return line.indices.map { at in
            let index = line[at]
            let left = start + line[..<at].map { sizes[$0].width + spacing }.reduce(0, +)
            return CGPoint(x: left, y: tops[row] + (heights[row] - sizes[index].height) / 2)
        }
    }
}

struct PlanFlowRow: Layout {
    var spacing: CGFloat = 8
    var lineSpacing: CGFloat = 8
    var alignment: HorizontalAlignment = .leading

    private func sizes(_ proposal: ProposedViewSize, _ subviews: Subviews) -> [CGSize] {
        subviews.map { $0.sizeThatFits(ProposedViewSize(width: proposal.width, height: nil)) }
    }

    func sizeThatFits(proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) -> CGSize {
        flowSize(sizes(proposal, subviews), proposal.width, spacing: spacing, lineSpacing: lineSpacing)
    }

    func placeSubviews(in bounds: CGRect, proposal: ProposedViewSize, subviews: Subviews, cache: inout ()) {
        let measured = sizes(proposal, subviews)
        let positions = flowPositions(measured, bounds.width, spacing: spacing, lineSpacing: lineSpacing, centred: alignment == .center)
        zip(subviews, zip(positions, measured)).forEach { subview, placement in
            subview.place(at: CGPoint(x: bounds.minX + placement.0.x, y: bounds.minY + placement.0.y), proposal: ProposedViewSize(placement.1))
        }
    }
}
