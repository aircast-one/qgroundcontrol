import SwiftUI

private let CONSOLE_ROOT = "mavlinkConsole"
private let CONSOLE_VIEW = "view.mavlinkConsole"

let CONSOLE_OPEN = "\(CONSOLE_ROOT).open"
let CONSOLE_EMPTY_TEXT = "> "

struct TextRange: Equatable {
    let start: Int
    let end: Int

    init(_ index: Int) {
        start = index
        end = index
    }
}

struct TextFieldValue: Equatable {
    let text: String
    var selection: TextRange

    init(_ text: String, _ selection: TextRange? = nil) {
        self.text = text
        self.selection = selection ?? TextRange(text.count)
    }
}

func splitCompleteLines(_ field: TextFieldValue) -> (String?, TextFieldValue) {
    guard let cut = field.text.lastIndex(of: "\n") else { return (nil, field) }
    let at = field.text.distance(from: field.text.startIndex, to: cut)
    let leftover = String(field.text[field.text.index(after: cut)...])
    return (String(field.text[..<cut]), TextFieldValue(leftover, TextRange(min(max(field.selection.end - at - 1, 0), leftover.count))))
}

func shouldFollowTail(_ lastVisibleIndex: Int?, _ count: Int) -> Bool {
    lastVisibleIndex.map { $0 >= count - 2 } ?? true
}

struct ConsoleScreen: View {
    @Environment(\.theme) private var theme
    @QgcPath(CONSOLE_VIEW) private var consoleJson
    @State private var field = ""
    @State private var visible: Set<Int> = []
    @State private var scrollRequest = 0

    var body: some View {
        let lines = consoleLines(consoleJson)
        VStack(spacing: 0) {
            ScrollViewReader { proxy in
                ScrollView {
                    LazyVStack(alignment: .leading, spacing: 4) {
                        if lines.isEmpty {
                            Text(CONSOLE_EMPTY_TEXT)
                                .font(.bodyMedium)
                                .monospaced()
                                .foregroundStyle(theme.colors.onSurface)
                        }
                        ForEach(Array(lines.enumerated()), id: \.offset) { at, line in
                            Text(consoleLineStyled(line, theme.aircast.warning, theme.colors.error))
                                .font(.bodyMedium)
                                .monospaced()
                                .foregroundStyle(isPromptLine(line) ? theme.colors.primary : theme.colors.onSurface)
                                .frame(maxWidth: .infinity, alignment: .leading)
                                .textSelection(.enabled)
                                .id(at)
                                .onAppear { visible.insert(at) }
                                .onDisappear { visible.remove(at) }
                        }
                    }
                    .padding(16)
                }
                .background(theme.colors.surfaceContainerLowest)
                .onChange(of: lines.count) { before, now in
                    if now > 0 && shouldFollowTail(visible.max(), before) { proxy.scrollTo(now - 1, anchor: .bottom) }
                }
                .onChange(of: scrollRequest) {
                    if !lines.isEmpty { proxy.scrollTo(lines.count - 1, anchor: .bottom) }
                }
            }

            HStack(spacing: 8) {
                TextField("Enter commands here...", text: Binding(get: { field }, set: edit), axis: .vertical)
                    .lineLimit(1...4)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.send)
                    .padding(.horizontal, 16)
                    .padding(.vertical, 12)
                    .background(theme.colors.surfaceContainerHighest, in: Capsule())
                ForEach([("historyUp", "\u{2191}"), ("historyDown", "\u{2193}")], id: \.0) { step, arrow in
                    Button(arrow) { recall(step) }
                        .buttonStyle(.borderless)
                        .frame(width: 40, height: 40)
                }
                Button(action: send) {
                    Image(.send)
                        .foregroundStyle(theme.colors.onPrimaryContainer)
                        .frame(width: 40, height: 40)
                        .background(theme.colors.primaryContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
                }
                .accessibilityLabel("Send")
            }
            .padding(.horizontal, 16)
            .padding(.vertical, 10)
            .background(theme.colors.surfaceContainerLow)
        }
        .task { offMainInOrder { Qgc.invoke(CONSOLE_OPEN) } }
    }

    private func sendText(_ toSend: String) {
        offMainInOrder { Qgc.invoke("\(CONSOLE_ROOT).sendCommand", toSend) }
    }

    private func send() {
        let toSend = field
        field = ""
        sendText(toSend)
        scrollRequest += 1
    }

    private func edit(_ next: String) {
        let (complete, leftover) = splitCompleteLines(TextFieldValue(next))
        field = leftover.text
        guard let complete else { return }
        sendText(complete)
        scrollRequest += 1
    }

    private func recall(_ step: String) {
        let current = field
        Task {
            let recalled = await offMain { () -> String? in
                if case .string(let text) = Qgc.invokeResult("\(CONSOLE_ROOT).\(step)", current) { return text }
                return nil
            }
            field = recalled ?? current
        }
    }
}

func isPromptLine(_ line: String) -> Bool {
    line.range(of: "^\\w*sh> ", options: .regularExpression) != nil
}

func consoleLineStyled(_ line: String, _ warning: Color, _ error: Color) -> AttributedString {
    var styled = AttributedString(line)
    if let (prefix, color) = [("WARN", warning), ("ERROR", error)].first(where: { line.hasPrefix($0.0) }),
       let range = styled.range(of: prefix) {
        styled[range].foregroundColor = color
    }
    return styled
}

func consoleLines(_ view: JSON?) -> [String] {
    view?["lines"].arrayOrNil?.map(\.string) ?? []
}
