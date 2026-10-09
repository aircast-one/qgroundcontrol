import SwiftUI

private let DISPLAY_STORE = "fly-instrument-display"
private let DEFAULT_RANGE_LOW = 0.0
private let DEFAULT_RANGE_HIGH = 100.0
private let GREEN: Int64 = 0xFF008000
let NO_COLOUR: Int64 = 0
private let ICON_FOLDER = "InstrumentValueIcons"
let RANGE_COLOURS: [Int64] = [GREEN, 0xFFFFFF00, 0xFFFFA500, 0xFFFF0000, 0xFF0000FF, 0xFFFFFFFF, NO_COLOUR]

enum RangeType: String, CaseIterable, Hashable {
    case None, Color, Opacity, Icon

    var label: String { rawValue }
    var name: String { rawValue }
}

struct ValueDisplay: Equatable {
    var text: String = ""
    var showUnits: Bool = true
    var showIcon: Bool = false
    var icon: String = ""
    var rangeType: RangeType = .None
    var values: [Double] = []
    var colours: [Int64] = []
    var opacities: [Double] = []
    var icons: [String] = []

    func with(_ change: (inout ValueDisplay) -> Void) -> ValueDisplay {
        var next = self
        change(&next)
        return next
    }
}

private func replacing<T>(_ list: [T], _ at: Int, _ value: T) -> [T] {
    list.enumerated().map { $0.offset == at ? value : $0.element }
}

private func dropping<T>(_ list: [T], _ at: Int) -> [T] {
    list.enumerated().filter { $0.offset != at }.map(\.element)
}

func withRangeType(_ display: ValueDisplay, _ type: RangeType, _ firstIcon: String) -> ValueDisplay {
    let values = type == .None ? [] : [DEFAULT_RANGE_LOW, DEFAULT_RANGE_HIGH]
    let slots = type == .None ? 0 : values.count + 1
    return display.with {
        $0.rangeType = type
        $0.values = values
        $0.colours = type == .Color ? Array(repeating: GREEN, count: slots) : []
        $0.opacities = type == .Opacity ? Array(repeating: 1.0, count: slots) : []
        $0.icons = type == .Icon ? Array(repeating: firstIcon, count: slots) : []
    }
}

func withRow(_ display: ValueDisplay, _ firstIcon: String) -> ValueDisplay {
    display.with {
        $0.values = display.values + [(display.values.last ?? DEFAULT_RANGE_LOW) + 1]
        $0.colours = display.rangeType == .Color ? display.colours + [GREEN] : display.colours
        $0.opacities = display.rangeType == .Opacity ? display.opacities + [1.0] : display.opacities
        $0.icons = display.rangeType == .Icon ? display.icons + [firstIcon] : display.icons
    }
}

func withoutRow(_ display: ValueDisplay, _ index: Int) -> ValueDisplay {
    display.with {
        $0.values = dropping(display.values, index)
        $0.colours = dropping(display.colours, index + 1)
        $0.opacities = dropping(display.opacities, index + 1)
        $0.icons = dropping(display.icons, index + 1)
    }
}

func rangeIndex(_ raw: Double?, _ values: [Double]) -> Int {
    guard let value = raw, !value.isNaN else { return 0 }
    return values.firstIndex { value <= $0 } ?? values.count
}

func displayColour(_ display: ValueDisplay, _ raw: Double?) -> Int64? {
    guard display.rangeType == .Color else { return nil }
    let at = rangeIndex(raw, display.values)
    return display.colours.indices.contains(at) && display.colours[at] != NO_COLOUR ? display.colours[at] : nil
}

func displayOpacity(_ display: ValueDisplay, _ raw: Double?) -> Double {
    let at = rangeIndex(raw, display.values)
    guard display.rangeType == .Opacity, display.opacities.indices.contains(at) else { return 1 }
    return min(max(display.opacities[at], 0), 1)
}

func displayIcon(_ display: ValueDisplay, _ raw: Double?) -> String? {
    let at = rangeIndex(raw, display.values)
    let icon: String? = display.rangeType == .Icon
        ? (display.icons.indices.contains(at) ? display.icons[at] : nil)
        : display.showIcon ? display.icon : nil
    return icon.flatMap { $0.isBlank ? nil : $0 }
}

func iconShown(_ display: ValueDisplay) -> Bool { display.rangeType == .Icon || display.showIcon }

func displayReading(_ display: ValueDisplay, _ value: String, _ units: String) -> String {
    display.showUnits && !units.isBlank ? "\(value) \(units)" : value
}

func displayJson(_ display: ValueDisplay) -> String {
    JSON.encode([
        "text": display.text,
        "showUnits": display.showUnits,
        "showIcon": display.showIcon,
        "icon": display.icon,
        "rangeType": display.rangeType.name,
        "values": display.values,
        "colours": display.colours,
        "opacities": display.opacities,
        "icons": display.icons,
    ] as [String: Any])
}

func displayFrom(_ json: String?) -> ValueDisplay {
    guard let json else { return ValueDisplay() }
    let o = JSON.parse(json)
    guard o.object != nil else { return ValueDisplay() }
    return ValueDisplay(
        text: o["text"].string,
        showUnits: o["showUnits"].bool(true),
        showIcon: o["showIcon"].bool,
        icon: o["icon"].string,
        rangeType: RangeType(rawValue: o["rangeType"].string) ?? .None,
        values: o["values"].array.map { $0.double ?? .nan },
        colours: o["colours"].array.map { $0.int64 ?? 0 },
        opacities: o["opacities"].array.map { $0.double ?? .nan },
        icons: o["icons"].array.map(\.string)
    )
}

private let displayStore = UserDefaults(suiteName: DISPLAY_STORE) ?? .standard

func readDisplays(_ vehicleClass: String) -> [String: ValueDisplay] {
    let prefix = "\(vehicleClass)/"
    return Dictionary(
        displayStore.dictionaryRepresentation()
            .filter { $0.key.hasPrefix(prefix) }
            .map { (String($0.key.dropFirst(prefix.count)), displayFrom($0.value as? String)) },
        uniquingKeysWith: { $1 }
    )
}

func writeDisplay(_ vehicleClass: String, _ id: String, _ display: ValueDisplay) {
    displayStore.set(displayJson(display), forKey: "\(vehicleClass)/\(id)")
}

private let iconIndex: [String] = NSDataAsset(name: "\(ICON_FOLDER)/index")
    .map { JSON.parse(String(decoding: $0.data, as: UTF8.self)).strings.filter { $0.hasSuffix(".svg") }.sorted() } ?? []

func iconNames() -> [String] { iconIndex }

func argbColor(_ colour: Int64) -> Color {
    Color(
        .sRGB,
        red: Double((colour >> 16) & 0xFF) / 255,
        green: Double((colour >> 8) & 0xFF) / 255,
        blue: Double(colour & 0xFF) / 255,
        opacity: Double((colour >> 24) & 0xFF) / 255
    )
}

struct ValueIcon: View {
    let name: String
    let tint: Color
    let size: CGFloat

    var body: some View {
        Image("\(ICON_FOLDER)/\(name.removingSuffix(".svg"))")
            .renderingMode(.template)
            .resizable()
            .scaledToFit()
            .foregroundStyle(tint)
            .frame(width: size, height: size)
            .accessibilityLabel(name.removingSuffix(".svg"))
    }
}

private struct Swatch: View {
    let colour: Int64
    let chosen: Bool
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            Circle()
                .fill(colour == NO_COLOUR ? SwiftUI.Color.clear : argbColor(colour))
                .overlay(Circle().stroke(chosen ? theme.colors.primary : theme.colors.outline, lineWidth: 2))
                .frame(width: 24, height: 24)
                .contentShape(Circle())
        }
        .buttonStyle(.plain)
    }
}

private struct IconPick: Identifiable {
    let id = UUID()
    let current: String
    let chosen: (String) -> Void
}

private struct IconPickerDialog: View {
    let names: [String]
    let chosen: String
    let onDismiss: () -> Void
    let onPick: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        NavigationStack {
            ScrollView {
                LazyVGrid(columns: [GridItem(.adaptive(minimum: 44))]) {
                    ForEach(names, id: \.self) { name in
                        Button { onPick(name) } label: {
                            ValueIcon(name: name, tint: theme.colors.onSurface, size: 24)
                                .padding(6)
                                .border(name == chosen ? theme.colors.primary : SwiftUI.Color.clear, width: 2)
                        }
                        .buttonStyle(.plain)
                        .padding(4)
                    }
                }
                .padding(Space.s4)
            }
            .navigationTitle("Select icon")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel", action: onDismiss) }
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
    }
}

private struct IconButtonFor: View {
    let name: String
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            ValueIcon(name: name, tint: theme.colors.onSurface, size: 24)
                .padding(6)
                .border(theme.colors.outline, width: 1)
        }
        .buttonStyle(.plain)
    }
}

struct ValueDisplayDialog<Extra: View>: View {
    let label: String
    let initial: ValueDisplay
    let onDismiss: () -> Void
    let extra: () -> Extra
    let onDone: (ValueDisplay) -> Void

    init(label: String, initial: ValueDisplay, onDismiss: @escaping () -> Void, @ViewBuilder extra: @escaping () -> Extra, onDone: @escaping (ValueDisplay) -> Void) {
        self.label = label
        self.initial = initial
        self.onDismiss = onDismiss
        self.extra = extra
        self.onDone = onDone
    }

    var body: some View {
        SwiftUI.Color.clear
            .frame(width: 0, height: 0)
            .accessibilityHidden(true)
            .queuedSheet(isPresented: Binding(get: { true }, set: { shown in if !shown { onDismiss() } })) {
                ValueDisplayForm(label: label, initial: initial, onDismiss: onDismiss, extra: extra, onDone: onDone)
            }
    }
}

extension ValueDisplayDialog where Extra == EmptyView {
    init(label: String, initial: ValueDisplay, onDismiss: @escaping () -> Void, onDone: @escaping (ValueDisplay) -> Void) {
        self.init(label: label, initial: initial, onDismiss: onDismiss, extra: { EmptyView() }, onDone: onDone)
    }
}

private struct ValueDisplayForm<Extra: View>: View {
    let label: String
    let onDismiss: () -> Void
    let extra: () -> Extra
    let onDone: (ValueDisplay) -> Void
    @State private var display: ValueDisplay
    @State private var picking: IconPick?
    @Environment(\.theme) private var theme
    private let names = iconNames()

    init(label: String, initial: ValueDisplay, onDismiss: @escaping () -> Void, extra: @escaping () -> Extra, onDone: @escaping (ValueDisplay) -> Void) {
        self.label = label
        self.onDismiss = onDismiss
        self.extra = extra
        self.onDone = onDone
        _display = State(initialValue: initial)
    }

    private var firstIcon: String { names.first ?? "" }

    private func pick(_ current: String, _ chosen: @escaping (String) -> Void) {
        picking = IconPick(current: current, chosen: chosen)
    }

    var body: some View {
        NavigationStack {
            ScrollView {
                VStack(alignment: .leading, spacing: Space.s2) {
                    Text(label).font(.labelMedium)
                    extra()
                    HStack(spacing: Space.s2) {
                        Picker("", selection: Binding(
                            get: { display.showIcon },
                            set: { icon in
                                display = icon
                                    ? display.with { $0.showIcon = true; $0.icon = display.icon.ifBlank(firstIcon) }
                                    : display.with { $0.showIcon = false }
                            }
                        )) {
                            Text("Icon").tag(true)
                            Text("Text").tag(false)
                        }
                        .pickerStyle(.segmented)
                        .fixedSize()
                        if display.showIcon {
                            IconButtonFor(name: display.icon) { pick(display.icon) { name in display = display.with { $0.icon = name } } }
                        }
                    }
                    if !display.showIcon {
                        TextField("Text", text: Binding(get: { display.text }, set: { text in display = display.with { $0.text = text } }))
                            .textFieldStyle(.roundedBorder)
                    }
                    Toggle("Show units", isOn: Binding(get: { display.showUnits }, set: { on in display = display.with { $0.showUnits = on } }))
                    Text("Value range").font(.titleSmall)
                    Text("Change the color, opacity or icon when the value crosses a threshold").font(.bodySmall)
                    Picker("", selection: Binding(
                        get: { display.rangeType },
                        set: { type in if display.rangeType != type { display = withRangeType(display, type, firstIcon) } }
                    )) {
                        ForEach(RangeType.allCases, id: \.self) { Text($0.label).tag($0) }
                    }
                    .pickerStyle(.segmented)
                    if display.rangeType != .None {
                        Text(rangeHelp(display.rangeType)).font(.bodySmall)
                        ForEach(0...display.values.count, id: \.self) { row in
                            rangeRow(row)
                        }
                        Button("Add row") { display = withRow(display, firstIcon) }
                    }
                }
                .padding(Space.s4)
            }
            .navigationTitle("Telemetry display")
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) { Button("Cancel", action: onDismiss) }
                ToolbarItem(placement: .confirmationAction) { Button("Done") { onDone(display) } }
            }
        }
        .presentationDetents([.medium, .large])
        .presentationDragIndicator(.visible)
        .queuedSheet(item: $picking) { request in
            IconPickerDialog(names: names, chosen: request.current, onDismiss: { picking = nil }) { name in
                request.chosen(name)
                picking = nil
            }
        }
    }

    @ViewBuilder
    private func rangeRow(_ row: Int) -> some View {
        HStack(spacing: 6) {
            if display.values.indices.contains(row) {
                DecimalField(label: "≤", value: display.values[row]) { v in
                    display = display.with { $0.values = replacing(display.values, row, v) }
                }
            } else {
                Text("above").font(.bodySmall).frame(width: 96, alignment: .leading)
            }
            RangeCell(display: display, row: row, onChange: { display = $0 }, onPick: pick)
            if row < display.values.count && display.values.count > 1 {
                Button("✕") { display = withoutRow(display, row) }
            }
        }
    }
}

private struct DecimalField: View {
    let label: String
    let value: Double?
    let onValue: (Double) -> Void
    @State private var typed: String?

    var body: some View {
        TextField(label, text: Binding(
            get: { typed ?? value.map { String($0) } ?? "" },
            set: { text in
                typed = text
                Double(text).map(onValue)
            }
        ))
        .keyboardType(.numbersAndPunctuation)
        .textFieldStyle(.roundedBorder)
        .frame(width: 96)
        .onChange(of: value) { _, next in
            if typed.flatMap(Double.init) != next { typed = nil }
        }
    }
}

func rangeHelp(_ type: RangeType) -> String {
    switch type {
    case .Color: "Specify the color you want to apply based on value ranges. The color will be applied to the icon if available, otherwise to the value itself."
    case .Opacity: "Specify the icon opacity you want based on value ranges."
    case .Icon: "Specify the icon you want to display based on value ranges."
    case .None: ""
    }
}

private struct RangeCell: View {
    let display: ValueDisplay
    let row: Int
    let onChange: (ValueDisplay) -> Void
    let onPick: (String, @escaping (String) -> Void) -> Void

    var body: some View {
        switch display.rangeType {
        case .Color:
            HStack(spacing: Space.s1) {
                ForEach(RANGE_COLOURS, id: \.self) { colour in
                    Swatch(colour: colour, chosen: display.colours.indices.contains(row) && display.colours[row] == colour) {
                        onChange(display.with { $0.colours = replacing(display.colours, row, colour) })
                    }
                }
            }
        case .Opacity:
            DecimalField(label: "Opacity", value: display.opacities.indices.contains(row) ? display.opacities[row] : nil) { v in
                onChange(display.with { $0.opacities = replacing(display.opacities, row, v) })
            }
        case .Icon:
            if display.icons.indices.contains(row) {
                let current = display.icons[row]
                IconButtonFor(name: current) {
                    onPick(current) { name in onChange(display.with { $0.icons = replacing(display.icons, row, name) }) }
                }
            }
        case .None:
            EmptyView()
        }
    }
}

struct ValueLabel: View {
    let display: ValueDisplay
    let raw: Double?
    let label: String
    let fallback: Color

    var body: some View {
        let tint = displayColour(display, raw).map(argbColor) ?? fallback
        let opacity = displayOpacity(display, raw)
        if let icon = displayIcon(display, raw) {
            ValueIcon(name: icon, tint: tint, size: 16).opacity(opacity)
        } else if !iconShown(display) {
            Text(display.text.ifBlank(label)).font(.labelSmall).foregroundStyle(tint).opacity(opacity)
        }
    }
}
