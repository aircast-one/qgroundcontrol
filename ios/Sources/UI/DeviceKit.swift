import SwiftUI
#if os(iOS)
import UIKit
#endif
import UniformTypeIdentifiers

struct DeviceBadge: Equatable, Hashable {
    let level: Int
    let text: String
}

struct DeviceOption: Equatable, Hashable {
    let value: String
    let label: String
}

struct DeviceChartView: Equatable {
    let label: String
    let values: [Double]
    let max: Int
    let value: String
    let level: Int
}

struct DeviceRequest: Equatable {
    let method: DeviceMethod
    let path: String
    var body: JSON? = nil
}

func parseBadge(_ json: JSON?) -> DeviceBadge { DeviceBadge(level: json?["level"].int(0) ?? 0, text: json?["text"].string ?? "") }

func parseBadges(_ json: JSON?, _ key: String) -> [DeviceBadge] { (json?[key].objects ?? []).map { parseBadge($0) } }

func optDeviceBadge(_ json: JSON?, _ key: String) -> DeviceBadge? { json?[key].objectOrNil.map { parseBadge($0) } }

func parseOptions(_ json: JSON?, _ key: String) -> [DeviceOption] {
    (json?[key].objects ?? []).map { DeviceOption(value: $0["value"].string, label: $0["label"].string) }
}

func parseChart(_ json: JSON?) -> DeviceChartView? {
    guard let json else { return nil }
    return DeviceChartView(
        label: json["label"].string,
        values: json["values"].array.compactMap(\.numberOrNil),
        max: json["max"].int(100),
        value: json["value"].string,
        level: json["level"].int(DeviceLevel.Neutral)
    )
}

func deviceLevelColor(_ level: Int, _ theme: Theme) -> Color {
    switch level {
    case DeviceLevel.Ready: theme.aircast.success
    case DeviceLevel.Warn: theme.aircast.warning
    case DeviceLevel.Blocked: theme.aircast.alert
    default: theme.colors.onSurfaceVariant
    }
}

func deviceLevelTextColor(_ level: Int, _ theme: Theme) -> Color { level == DeviceLevel.Neutral ? theme.colors.onSurface : deviceLevelColor(level, theme) }

func deviceLevelIcon(_ level: Int) -> Icon {
    switch level {
    case DeviceLevel.Ready: .checkCircle
    case DeviceLevel.Neutral: .info
    default: .warning
    }
}

func deviceMono(_ scale: TypeScale) -> Font { .system(size: scale.size, weight: scale.weight, design: .monospaced) }

let DEVICE_LIST_PADDING: CGFloat = 16
let DEVICE_WAITING = "Waiting for the device to answer\u{2026}"

struct DeviceSection<Content: View>: View {
    let title: String
    var badge: DeviceBadge? = nil
    @ViewBuilder let content: () -> Content
    @Environment(\.LocalSettingsList) private var band
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            Section {
                content()
            } header: {
                HStack(spacing: Space.s2) {
                    Text(title)
                    Spacer(minLength: 0)
                    if let badge { DeviceBadgeText(badge: badge).textCase(nil) }
                }
            }
        } else {
            handDrawn
        }
    }

    private var handDrawn: some View {
        VStack(alignment: .leading, spacing: 0) {
            SectionHeader(text: title)
                .overlay(alignment: .trailing) {
                    if let badge {
                        DeviceBadgeText(badge: badge)
                            .padding(.trailing, DEVICE_LIST_PADDING)
                            .padding(.top, band ? 12 : 0)
                    }
                }
            content()
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .accessibilityElement(children: .contain)
        .accessibilityLabel(title)
    }
}

struct DeviceBadgeText: View {
    let badge: DeviceBadge
    @Environment(\.theme) private var theme

    var body: some View {
        if !badge.text.isEmpty {
            HStack(spacing: 6) {
                Circle().fill(deviceLevelColor(badge.level, theme)).frame(width: 8, height: 8)
                Text(badge.text).font(.labelMedium).foregroundStyle(deviceLevelColor(badge.level, theme))
            }
        }
    }
}

struct DeviceNotice: View {
    let notice: DeviceBadge
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        HStack(alignment: .top, spacing: 12) {
            Image(deviceLevelIcon(notice.level))
                .font(.system(size: 18))
                .foregroundStyle(deviceLevelColor(notice.level, theme))
                .frame(width: 20, height: 20)
            Text(notice.text).font(.bodyMedium).frame(maxWidth: .infinity, alignment: .leading)
        }
        .rowPadding(horizontal: 16, vertical: 12)
        .background(nativeList ? Color.clear : theme.colors.surfaceContainerHigh, in: RoundedRectangle(cornerRadius: Corner.medium))
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
    }
}

struct DeviceHint: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        Text(text).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct DeviceText: View {
    let text: String
    @Environment(\.theme) private var theme

    var body: some View {
        if !text.isEmpty {
            Text(text)
                .font(.bodyMedium)
                .foregroundStyle(theme.colors.onSurfaceVariant)
                .frame(maxWidth: .infinity, alignment: .leading)
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        }
    }
}

struct DeviceFact: View {
    let label: String
    let value: String
    var level: Int = DeviceLevel.Neutral
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            LabeledContent(label) {
                Text(value)
                    .foregroundStyle(level == DeviceLevel.Neutral ? Color.secondary : deviceLevelColor(level, theme))
                    .multilineTextAlignment(.trailing)
            }
        } else {
            handDrawn
        }
    }

    private var handDrawn: some View {
        HStack(spacing: 16) {
            Text(label).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            Text(value)
                .font(.bodyLarge)
                .foregroundStyle(deviceLevelTextColor(level, theme))
                .multilineTextAlignment(.trailing)
                .frame(maxWidth: .infinity, alignment: .trailing)
        }
        .padding(.horizontal, DEVICE_LIST_PADDING)
        .padding(.vertical, 8)
    }
}

func copyToClipboard(_ text: String, _ notice: ChangeNotice, _ message: String = "Copied") {
    PlatformPasteboard.copy(text)
    notice.show(message, nil)
}

struct CopyRow: View {
    let label: String
    let text: String
    var hint: String = ""
    var qr: Bool = false
    var level: Int = DeviceLevel.Neutral
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var showingQr = false

    var body: some View {
        HStack(spacing: 8) {
            VStack(alignment: .leading, spacing: 2) {
                Text(label).font(.bodyMedium).foregroundStyle(level == DeviceLevel.Neutral ? theme.colors.onSurfaceVariant : deviceLevelColor(level, theme))
                Text(text).font(deviceMono(.bodyLarge)).textSelection(.enabled)
                if !hint.isEmpty {
                    Text(hint).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                }
            }
            .frame(maxWidth: .infinity, alignment: .leading)
            if qr {
                Button { showingQr = true } label: { Image(.qrCode2).frame(width: Metrics.iconButton, height: Metrics.iconButton).contentShape(Rectangle()).macHover(Circle()) }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Show \(label) as a QR code")
                    .macTooltip("Show \(label) as a QR code")
            }
            Button { copyToClipboard(text, notice) } label: { Image(.contentCopy).frame(width: Metrics.iconButton, height: Metrics.iconButton).contentShape(Rectangle()).macHover(Circle()) }
                .buttonStyle(.plain)
                .accessibilityLabel("Copy \(label)")
                .macTooltip("Copy \(label)")
        }
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        .sheet(isPresented: $showingQr) { QrDialog(title: label, text: text) { showingQr = false } }
    }
}

struct QrDialog: View {
    let title: String
    let text: String
    let onDismiss: () -> Void
    @State private var image: PlatformImage?

    var body: some View {
        VStack(spacing: 12) {
            Text(title).font(.titleLarge).frame(maxWidth: .infinity, alignment: .leading)
            ZStack {
                RoundedRectangle(cornerRadius: Corner.small).fill(Color.white)
                if let image {
                    Image(platformImage: image)
                        .interpolation(.none)
                        .resizable()
                        .scaledToFit()
                        .frame(width: 240, height: 240)
                        .accessibilityLabel("QR code for \(text)")
                }
            }
            .frame(width: 256, height: 256)
            Text(text).font(deviceMono(.bodySmall)).multilineTextAlignment(.center).frame(maxWidth: 256).textSelection(.enabled)
            Button("Done", action: onDismiss).buttonStyle(.text).frame(maxWidth: .infinity, alignment: .trailing)
        }
        .padding(24)
        .presentationDetents([.medium, .large])
        .task(id: text) { image = await deviceQr(text).flatMap { PlatformImage(data: $0) } }
    }
}

@Observable final class DeviceActionState {
    private(set) var busy = false
    var refusal: DeviceRefusal?

    init() {}

    @MainActor
    func run(_ request: DeviceRequest, success: String = "", notice: ChangeNotice) async -> Bool {
        busy = true
        refusal = nil
        let refused = await deviceCall(request.method, request.path, request.body)
        busy = false
        refusal = refused
        if refused == nil && !success.isEmpty { notice.show(success, nil) }
        return refused == nil
    }
}

struct DeviceRefusalText: View {
    let refusal: DeviceRefusal?
    var field: String = ""
    @Environment(\.theme) private var theme

    var body: some View {
        if let refusal, refusal.field == field {
            Text(refusal.message)
                .font(.bodySmall)
                .foregroundStyle(theme.colors.error)
                .frame(maxWidth: .infinity, alignment: .leading)
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 4)
        }
    }
}

struct ActionConfirm: Equatable {
    let title: String
    let text: String
    let confirm: String
}

extension View {
    func deviceConfirm(_ confirm: ActionConfirm?, isPresented: Binding<Bool>, onConfirm: @escaping () -> Void) -> some View {
        alert(confirm?.title ?? "", isPresented: isPresented, presenting: confirm) { shown in
            Button(shown.confirm, action: onConfirm)
            Button("Cancel", role: .cancel) {}
        } message: { shown in
            Text(shown.text)
        }
    }

    func deviceTypedConfirm(
        _ isPresented: Binding<Bool>,
        title: String,
        text: String,
        label: String,
        expected: String,
        confirm: String,
        typed: Binding<String>,
        onConfirm: @escaping (String) -> Void
    ) -> some View {
        alert(title, isPresented: isPresented) {
            TextField(label, text: typed)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
            Button(confirm) { onConfirm(typed.wrappedValue.trimmed) }
                .disabled(expected.isEmpty || typed.wrappedValue.trimmed != expected)
            Button("Cancel", role: .cancel) {}
        } message: {
            Text("\(text)\n\n\(label) (\(expected))")
        }
    }
}

enum LinkLook { case Filled, Tonal, Outlined, Text }

struct DeviceButtonLook: ViewModifier {
    let look: LinkLook

    func body(content: Content) -> some View {
        switch look {
        case .Filled: content.buttonStyle(.filled)
        case .Tonal, .Outlined: content.buttonStyle(.bordered)
        case .Text: content.buttonStyle(.text)
        }
    }
}

extension View {
    func deviceButton(_ look: LinkLook) -> some View { modifier(DeviceButtonLook(look: look)) }
}

struct ActionButton: View {
    let label: String
    let request: DeviceRequest
    var success: String = ""
    var confirm: ActionConfirm? = nil
    var tonal: Bool = true
    var onDone: () -> Void = {}
    @Environment(\.LocalChangeNotice) private var notice
    @Environment(\.LocalNativeList) private var nativeList
    @State private var action = DeviceActionState()
    @State private var asking = false

    private func go() {
        let request = request, success = success, notice = notice, onDone = onDone, action = action
        Task { @MainActor in if await action.run(request, success: success, notice: notice) { onDone() } }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button(label) { if confirm == nil { go() } else { asking = true } }
                .deviceButton(tonal ? .Tonal : .Text)
                .disabled(action.busy)
            if let refusal = action.refusal, refusal.field.isEmpty {
                DeviceRefusalText(refusal: refusal).padding(.horizontal, nativeList ? 0 : -DEVICE_LIST_PADDING)
            }
        }
        .deviceConfirm(confirm, isPresented: $asking) { go() }
    }
}

struct DeviceSwitchRow: View {
    let title: String
    let checked: Bool
    let on: DeviceRequest
    let off: DeviceRequest
    var detail: String = ""
    var success: String = ""
    var offSuccess: String? = nil
    var enabled: Bool = true
    var confirmOn: ActionConfirm? = nil
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var pending: Bool?
    @State private var asking = false

    private func flip(_ next: Bool) {
        pending = next
        let request = next ? on : off
        let message = next ? success : (offSuccess ?? success)
        let notice = notice, action = action
        Task { @MainActor in
            _ = await action.run(request, success: message, notice: notice)
            pending = nil
        }
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            HStack(spacing: 16) {
                VStack(alignment: .leading, spacing: 2) {
                    Text(title).font(.bodyLarge)
                    if !detail.isEmpty {
                        Text(detail).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                }
                .frame(maxWidth: .infinity, alignment: .leading)
                Toggle(title, isOn: Binding(
                    get: { pending ?? checked },
                    set: { next in if next && confirmOn != nil { asking = true } else { flip(next) } }
                ))
                .labelsHidden()
                .disabled(!enabled || action.busy)
            }
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 12)
            DeviceRefusalText(refusal: action.refusal, field: action.refusal?.field ?? "")
        }
        .deviceConfirm(confirmOn, isPresented: $asking) { flip(true) }
    }
}

enum DeviceKeyboard: Equatable { case text, uri, number, password }

private func uiKeyboard(_ keyboard: DeviceKeyboard) -> UIKeyboardType {
    switch keyboard {
    case .text, .password: .default
    case .uri: .URL
    case .number: .numberPad
    }
}

struct DeviceTextField: View {
    let label: String
    let value: String
    let onValueChange: (String) -> Void
    var placeholder: String = ""
    var isError: Bool = false
    var supporting: String = ""
    var keyboard: DeviceKeyboard = .text
    var lines: Int = 1
    var onSubmit: () -> Void = {}
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList
    @FocusState private var focused: Bool

    var body: some View {
        if nativeList && lines <= 1 && !label.isEmpty {
            // A native form row: the label on the left, the value typed on the right.
            FieldRow(
                label: label,
                value: value,
                prompt: placeholder,
                error: isError ? supporting : nil,
                hint: isError ? "" : supporting,
                keyboard: uiKeyboard(keyboard),
                autocapitalize: keyboard == .text,
                onSubmit: onSubmit,
                onChange: onValueChange
            )
        } else {
            stacked
        }
    }

    private var stacked: some View {
        VStack(alignment: .leading, spacing: 4) {
            if !label.isEmpty {
                Text(label).font(.bodySmall).foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            }
            TextField(placeholder, text: Binding(get: { value }, set: onValueChange), axis: lines > 1 ? .vertical : .horizontal)
                .lineLimit(lines > 1 ? lines...max(lines, 8) : 1...1)
                .font(.bodyLarge)
                .keyboardType(uiKeyboard(keyboard))
                .textInputAutocapitalization(keyboard == .text ? TextInputAutocapitalization.sentences : TextInputAutocapitalization.never)
                .autocorrectionDisabled(keyboard != .text)
                .submitLabel(.done)
                .onSubmit(onSubmit)
                .focused($focused)
                .rowPadding(horizontal: 12, vertical: 12)
                .overlay {
                    if !nativeList {
                        RoundedRectangle(cornerRadius: Corner.extraSmall)
                            .stroke(isError ? theme.colors.error : focused ? theme.colors.primary : theme.colors.outline, lineWidth: focused ? 2 : 1)
                    }
                }
            if !supporting.isEmpty {
                Text(supporting).font(.bodySmall).foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct SecretField: View {
    let label: String
    let value: String
    let onValueChange: (String) -> Void
    var placeholder: String = ""
    var isError: Bool = false
    var supportingText: String = ""
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList
    @State private var revealed = false

    var body: some View {
        VStack(alignment: .leading, spacing: 4) {
            Text(label).font(.bodySmall).foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            HStack(spacing: 8) {
                Button(revealed ? "Hide" : "Show") { revealed.toggle() }.buttonStyle(.text)
                Group {
                    if revealed {
                        TextField(placeholder, text: Binding(get: { value }, set: onValueChange))
                    } else {
                        SecureField(placeholder, text: Binding(get: { value }, set: onValueChange))
                    }
                }
                .font(.bodyLarge)
                .textInputAutocapitalization(.never)
                .autocorrectionDisabled()
            }
            .padding(.trailing, nativeList ? 0 : 12)
            .overlay {
                if !nativeList {
                    RoundedRectangle(cornerRadius: Corner.extraSmall).stroke(isError ? theme.colors.error : theme.colors.outline)
                }
            }
            if !supportingText.isEmpty {
                Text(supportingText).font(.bodySmall).foregroundStyle(isError ? theme.colors.error : theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct LinkButton: View {
    let href: String
    let text: String
    var look: LinkLook = .Tonal
    var external: Bool = false
    var icon: Icon? = nil
    @Environment(\.openURL) private var openURL

    var body: some View {
        Button {
            if let target = URL(string: href) { openURL(target) }
        } label: {
            HStack(spacing: 8) {
                if let icon { Image(icon) }
                Text(text)
                if external { Image(.openInNew) }
            }
        }
        .deviceButton(look)
    }
}

struct DeviceSelect: View {
    let label: String
    let value: String
    let options: [DeviceOption]
    let onPick: (String) -> Void
    var enabled: Bool = true

    var body: some View {
        ChoiceField(
            label: label,
            value: options.first { $0.value == value }?.label ?? value,
            options: options.map(\.label),
            enabled: enabled,
            onPick: { index in if options.indices.contains(index) { onPick(options[index].value) } }
        )
        .frame(maxWidth: .infinity)
    }
}

struct DeviceField {
    let name: String
    let label: String
    let value: String
    var placeholder: String = ""
    var hint: String = ""
    var keyboard: DeviceKeyboard = .text
    var options: [DeviceOption]? = nil
    var lines: Int = 1
    var shownWhen: (([String: String]) -> Bool)? = nil
}

func formBody(_ fields: [DeviceField], _ values: [String: String]) -> [String: String] {
    Dictionary(fields.map { ($0.name, values[$0.name] ?? $0.value) }, uniquingKeysWith: { _, last in last })
}

func shownFields(_ fields: [DeviceField], _ values: [String: String]) -> [DeviceField] {
    fields.filter { $0.shownWhen?(values) ?? true }
}

private struct FieldInput: View {
    let field: DeviceField
    let value: String
    let refusal: DeviceRefusal?
    let onValueChange: (String) -> Void

    var body: some View {
        let refused = refusal?.field == field.name
        let supporting = refused ? refusal?.message ?? "" : field.hint
        if let options = field.options {
            DeviceSelect(label: field.label, value: value, options: options, onPick: onValueChange)
        } else if field.keyboard == .password {
            SecretField(label: field.label, value: value, onValueChange: onValueChange, placeholder: field.placeholder, isError: refused, supportingText: supporting)
        } else {
            DeviceTextField(
                label: field.label,
                value: value,
                onValueChange: onValueChange,
                placeholder: field.placeholder,
                isError: refused,
                supporting: supporting,
                keyboard: field.keyboard,
                lines: field.lines
            )
        }
    }
}

struct DeviceForm: View {
    let fields: [DeviceField]
    let method: DeviceMethod
    let path: String
    let submit: String
    var success: String = ""
    var confirm: ActionConfirm? = nil
    var note: String = ""
    var extra: [String: String] = [:]
    var onDone: () -> Void = {}
    @Environment(\.theme) private var theme
    @Environment(\.LocalChangeNotice) private var notice
    @State private var action = DeviceActionState()
    @State private var values: [String: String] = [:]
    @State private var asking = false

    private func send() {
        let body = deviceBody(formBody(fields, values).merging(extra) { _, added in added })
        let request = DeviceRequest(method: method, path: path, body: body)
        let success = success, notice = notice, onDone = onDone, action = action
        Task { @MainActor in
            if await action.run(request, success: success, notice: notice) {
                values = [:]
                onDone()
            }
        }
    }

    var body: some View {
        let current = formBody(fields, values)
        let shown = shownFields(fields, current)
        VStack(alignment: .leading, spacing: 12) {
            ForEach(shown, id: \.name) { field in
                FieldInput(field: field, value: current[field.name] ?? "", refusal: action.refusal) { values[field.name] = $0 }
            }
            if !note.isEmpty {
                Text(note).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            if let refusal = action.refusal, !shown.contains(where: { $0.name == refusal.field }) {
                Text(refusal.message).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
            Button(submit) { if confirm == nil { send() } else { asking = true } }
                .buttonStyle(.bordered)
                .disabled(action.busy)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        .deviceConfirm(confirm, isPresented: $asking) { send() }
    }
}

func chartPoints(_ values: [Double], _ max: Int, _ width: CGFloat, _ height: CGFloat) -> [CGPoint] {
    guard values.count >= 2 else { return [] }
    let scale = Double(max > 0 ? max : 1)
    return values.enumerated().map { index, value in
        let clamped = Swift.min(Swift.max(value, 0), Double(max))
        return CGPoint(x: CGFloat(index) / CGFloat(values.count - 1) * width, y: height - CGFloat(clamped / scale) * height)
    }
}

struct DeviceChart: View {
    let chart: DeviceChartView
    @Environment(\.theme) private var theme

    var body: some View {
        let stroke = chart.level == DeviceLevel.Neutral ? theme.colors.primary : deviceLevelColor(chart.level, theme)
        HStack(spacing: 16) {
            VStack(alignment: .leading, spacing: 2) {
                Text(chart.label).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                Text(chart.value).font(.titleMedium).foregroundStyle(deviceLevelTextColor(chart.level, theme))
            }
            .frame(width: 112, alignment: .leading)
            Canvas { context, size in
                let points = chartPoints(chart.values, chart.max, size.width, size.height)
                guard let first = points.first else { return }
                var line = Path()
                line.move(to: first)
                points.dropFirst().forEach { line.addLine(to: $0) }
                context.stroke(line, with: .color(stroke), lineWidth: 2)
            }
            .frame(maxWidth: .infinity)
            .frame(height: 40)
            .accessibilityLabel("\(chart.label) history")
        }
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
    }
}

struct DeviceDisclosure<Content: View>: View {
    let title: String
    var summary: String = ""
    var badge: DeviceBadge? = nil
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme
    @State private var open: Bool

    init(_ title: String, summary: String = "", badge: DeviceBadge? = nil, initiallyOpen: Bool = false, @ViewBuilder content: @escaping () -> Content) {
        self.title = title
        self.summary = summary
        self.badge = badge
        self.content = content
        _open = State(initialValue: initiallyOpen)
    }

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button { open.toggle() } label: {
                HStack(spacing: 16) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                        if !summary.isEmpty {
                            Text(summary).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    if let badge { DeviceBadgeText(badge: badge) }
                    Image(open ? .arrowUp : .arrowDropDown).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
                .frame(maxWidth: .infinity, minHeight: Metrics.row)
                .contentShape(Rectangle())
                .macHover()
            }
            .buttonStyle(.plain)
            .accessibilityLabel(open ? "Collapse \(title)" : "Expand \(title)")
            if open { content() }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

struct StepText: View {
    let step: Int
    let title: String
    var text: String = ""
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("\(step) · \(title)").font(.titleSmall)
            if !text.isEmpty {
                Text(text).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .rowPadding(EdgeInsets(top: 12, leading: DEVICE_LIST_PADDING, bottom: 4, trailing: DEVICE_LIST_PADDING))
    }
}

struct DeviceStatusLine: View {
    let text: String
    var level: Int = DeviceLevel.Neutral
    @Environment(\.theme) private var theme

    var body: some View {
        if !text.isEmpty {
            Text(text)
                .font(.bodyMedium)
                .foregroundStyle(deviceLevelColor(level, theme))
                .frame(maxWidth: .infinity, alignment: .leading)
                .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
        }
    }
}

struct DeviceButtonRow<Content: View>: View {
    @ViewBuilder let content: () -> Content

    var body: some View {
        PlanFlowRow(spacing: 8, lineSpacing: 8) { content() }
            .frame(maxWidth: .infinity, alignment: .leading)
            .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
    }
}

struct DeviceChip: View {
    let label: String
    let selected: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            HStack(spacing: 4) {
                if selected { Image(.check) }
                Text(label).font(.labelLarge)
            }
            .padding(.horizontal, 12)
            .frame(height: 32)
            .foregroundStyle(selected ? theme.colors.onSecondaryContainer : theme.colors.onSurfaceVariant)
            .background(selected ? theme.colors.secondaryContainer : .clear, in: RoundedRectangle(cornerRadius: Corner.small))
            .overlay(RoundedRectangle(cornerRadius: Corner.small).stroke(selected ? .clear : theme.colors.outline))
            .contentShape(Rectangle())
        }
        .buttonStyle(.plain)
        .accessibilityAddTraits(selected ? .isSelected : [])
    }
}

struct StackedFact: View {
    let label: String
    let value: String
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text(label).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            Text(value).font(.bodyLarge)
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .rowPadding(horizontal: DEVICE_LIST_PADDING, vertical: 8)
    }
}

struct Fold<Content: View>: View {
    let title: String
    let open: Bool
    let onToggle: (Bool) -> Void
    var summary: String = ""
    @ViewBuilder let content: () -> Content
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Button { onToggle(!open) } label: {
                HStack(spacing: 12) {
                    VStack(alignment: .leading, spacing: 2) {
                        Text(title).font(.labelLarge).foregroundStyle(theme.colors.onSurfaceVariant)
                        if !summary.isEmpty {
                            Text(summary).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
                        }
                    }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    Image(open ? .arrowUp : .arrowDropDown).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                .padding(.vertical, 8)
                .frame(maxWidth: .infinity, minHeight: Metrics.rowCompact)
                .contentShape(Rectangle())
                .macHover()
            }
            .buttonStyle(.plain)
            .accessibilityLabel(open ? "Collapse \(title)" : "Expand \(title)")
            if open {
                VStack(alignment: .leading, spacing: 12) { content() }
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .padding(.bottom, 8)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
    }
}

func stagedPick(_ chosen: URL) -> URL? {
    let scoped = chosen.startAccessingSecurityScopedResource()
    defer { if scoped { chosen.stopAccessingSecurityScopedResource() } }
    let files = FileManager.default
    let folder = files.temporaryDirectory.appendingPathComponent("device-uploads", isDirectory: true)
    guard (try? files.createDirectory(at: folder, withIntermediateDirectories: true)) != nil else { return nil }
    let staged = folder.appendingPathComponent(chosen.lastPathComponent.replacingOccurrences(of: "/", with: "_"))
    try? files.removeItem(at: staged)
    return (try? files.copyItem(at: chosen, to: staged)).map { staged }
}

extension View {
    func deviceFilePicker(_ isPresented: Binding<Bool>, onPick: @escaping (URL?) -> Void) -> some View {
        fileImporter(isPresented: isPresented, allowedContentTypes: [UTType.item]) { result in
            guard case .success(let chosen) = result else { return }
            Task { @MainActor in onPick(await offMain { stagedPick(chosen) }) }
        }
    }
}
