import SwiftUI

let SYSLINK_SCREEN = "syslink"
private let SYSLINK_VIEW = "view.syslink"
private let SYSLINK_POLL_MS = 1000

struct Syslink: Equatable {
    let channel: Int
    let channelHint: String
    let address: String
    let addressHint: String
    let rate: Int
    let rates: [String]
}

func syslink(_ view: JSON?) -> Syslink? {
    guard let it = view, it["available"].bool else { return nil }
    return Syslink(
        channel: it["channel"].int(0),
        channelHint: it["channelHint"].string,
        address: it["address"].string,
        addressHint: it["addressHint"].string,
        rate: it["rate"].int(-1),
        rates: it["rates"].array.map(\.string)
    )
}

func hexAddress(_ typed: String) -> Bool {
    typed.count <= 10 && typed.allSatisfy { $0.isASCII && $0.isHexDigit }
}

private func refused(_ path: String, _ args: [Any?]) -> String? {
    refusal(Qgc.call(path, arguments: args))
}

struct SyslinkScreen: View {
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var read: Syslink?
    @State private var refusal: String?

    var body: some View {
        ZStack(alignment: .topLeading) {
            Color.clear
            if let radio = read {
                screen(radio)
            } else {
                Text("This vehicle has no Syslink radio.").padding(16)
            }
        }
        .task(id: revision) {
            let radio = await offMain { syslink(Qgc.get(SYSLINK_VIEW)) }
            read = radio
            try? await Task.sleep(for: .milliseconds(SYSLINK_POLL_MS))
            if !Task.isCancelled { revision += 1 }
        }
    }

    private func screen(_ radio: Syslink) -> some View {
        ScrollView {
            VStack(alignment: .leading, spacing: 8) {
                SectionHeader(text: "Radio Settings")
                RadioField(label: "Channel", value: String(radio.channel), hint: radio.channelHint, keyboard: .numbersAndPunctuation, accept: { $0.allSatisfy { $0.isASCII && $0.isNumber } }) { typed in
                    if let channel = Int(typed) { act("syslink.setChannel", channel) }
                }
                RadioField(label: "Address", value: radio.address, hint: radio.addressHint, keyboard: .asciiCapable, accept: hexAddress) { act("syslink.setAddress", $0) }
                ChoiceField(label: "Data Rate", value: radio.rates.indices.contains(radio.rate) ? radio.rates[radio.rate] : "", options: radio.rates) { act("syslink.setRate", $0) }
                if let refusal {
                    Text(refusal).foregroundStyle(theme.colors.error)
                }
                Button("Restore defaults") { act("syslink.resetDefaults") }.buttonStyle(.bordered)
            }
            .padding(.horizontal, 20)
            .padding(.vertical, 12)
        }
    }

    private func act(_ path: String, _ args: Any...) {
        Task {
            refusal = await offMain { refused(path, args) }
            read = await offMain { syslink(Qgc.get(SYSLINK_VIEW)) }
        }
    }
}

private struct RadioField: View {
    let label: String
    let value: String
    let hint: String
    let keyboard: UIKeyboardType
    let accept: (String) -> Bool
    let onDone: (String) -> Void
    @Environment(\.theme) private var theme
    @State private var typed = ""
    @FocusState private var focused: Bool

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            HStack {
                Text(label).frame(maxWidth: .infinity, alignment: .leading)
                TextField("", text: Binding(get: { typed }, set: { if accept($0) { typed = $0 } }))
                    .textFieldStyle(.roundedBorder)
                    .keyboardType(keyboard)
                    .textInputAutocapitalization(.never)
                    .autocorrectionDisabled()
                    .submitLabel(.done)
                    .focused($focused)
                    .onSubmit { focused = false }
                    .frame(width: 180)
            }
            Text(hint).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
        }
        .onChange(of: value, initial: true) { typed = value }
        .onChange(of: focused) { if !focused && typed != value { onDone(typed) } }
    }
}
