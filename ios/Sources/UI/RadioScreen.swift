import SwiftUI

private struct RadioNotice: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.bodyLarge)
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            .padding(Space.s6)
    }
}

struct PwmBar: View {
    let fraction: Float
    @Environment(\.theme) private var theme

    var body: some View {
        GeometryReader { geometry in
            ZStack(alignment: .leading) {
                theme.colors.surfaceVariant
                theme.colors.primary.frame(width: geometry.size.width * CGFloat(min(max(fraction, 0), 1)))
            }
        }
        .frame(height: 8)
        .clipShape(RoundedRectangle(cornerRadius: 4))
    }
}

private struct AttitudeRow: View {
    let stick: RadioStick
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 6) {
            HStack {
                Text(stick.title).font(.bodyLarge).frame(maxWidth: .infinity, alignment: .leading)
                Text(stickReading(stick))
                    .font(.labelMedium)
                    .foregroundStyle(stick.mapped ? theme.colors.onSurfaceVariant : theme.colors.error)
            }
            if stick.mapped { PwmBar(fraction: stick.fraction) }
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }
}

private struct CalibrationStep: View {
    let cal: RadioCalibration
    let onAction: (String) -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            Text(calibrationStep(cal.statusText))
                .font(.titleMedium)
                .frame(maxWidth: .infinity, alignment: .leading)
                .padding(.bottom, Space.s4)
            if cal.running { StickDiagram(positions: cal.stickPositions, single: false) }
            HStack(spacing: Space.s2) {
                Button(cal.nextText.ifBlank("Next")) { onAction("nextButtonClicked") }
                    .buttonStyle(.filled)
                    .disabled(!cal.nextEnabled)
                if cal.skipEnabled {
                    Button("Skip") { onAction("skipButtonClicked") }.buttonStyle(.bordered)
                }
                Spacer(minLength: 0)
                if cal.cancelEnabled {
                    Button("Cancel") { onAction("cancelButtonClicked") }.buttonStyle(.borderless)
                }
            }
            .frame(minHeight: 48)
        }
        .padding(Space.s4)
        .background(theme.colors.primaryContainer, in: RoundedRectangle(cornerRadius: Corner.medium))
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }
}

private struct ModeRow: View {
    let mode: Int
    let centeredThrottle: Bool?
    let onPick: (Int) -> Void
    let onCentered: (Bool) -> Void

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            Text("Transmitter mode — which stick is the throttle").font(.bodySmall)
            Picker("Transmitter mode", selection: Binding(get: { mode }, set: onPick)) {
                ForEach(1...4, id: \.self) { Text("Mode \($0)").tag($0) }
            }
            .pickerStyle(.segmented)
            if let centered = centeredThrottle {
                Toggle("Centered throttle", isOn: Binding(get: { centered }, set: onCentered)).font(.bodyMedium)
            }
        }
        .padding(.horizontal, Space.s5)
        .padding(.vertical, Space.s1)
    }
}

private struct ConfirmDialog: View {
    let prompt: RadioPrompt
    let onDismiss: () -> Void
    let onConfirm: (Int?) -> Void
    @State private var choice: Int?
    @Environment(\.theme) private var theme

    var body: some View {
        SetupDialog(title: prompt.title) {
            Text(prompt.body)
            ForEach(Array(prompt.choices.enumerated()), id: \.offset) { index, label in
                Button { choice = index } label: {
                    HStack(spacing: Space.s2) {
                        Image(systemName: index == choice ? "largecircle.fill.circle" : "circle")
                            .foregroundStyle(index == choice ? theme.colors.primary : theme.colors.onSurfaceVariant)
                        Text(label).foregroundStyle(theme.colors.onSurface)
                    }
                    .padding(.vertical, Space.s2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
            Button("Ok") {
                onConfirm(choice)
                onDismiss()
            }
            .buttonStyle(.filled)
        }
        .onChange(of: prompt, initial: true) { _, shown in choice = shown.choices.indices.last }
    }
}

private struct AdditionalSetup: View {
    let onInvoke: (String, Int?) -> Void
    @State private var prompt: RadioPrompt?

    var body: some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Divider()
            ViewThatFits(in: .horizontal) {
                HStack(spacing: Space.s2) { buttons }
                VStack(alignment: .leading, spacing: Space.s2) { buttons }
            }
        }
        .padding(Space.s1)
        .sheet(isPresented: presented($prompt)) {
            if let asked = prompt {
                ConfirmDialog(prompt: asked, onDismiss: { prompt = nil }, onConfirm: { onInvoke(asked.action, $0) })
            }
        }
    }

    private var buttons: some View {
        ForEach(RADIO_PROMPTS, id: \.action) { entry in
            Button(entry.title) { prompt = entry }.buttonStyle(.bordered)
        }
    }
}

private struct CalibrationStart: View {
    let view: RadioView
    let onAction: (String) -> Void
    let onMode: (Int) -> Void
    let onCentered: (Bool) -> Void
    let onInvoke: (String, Int?) -> Void
    @State private var prompting = false
    @State private var refusing = false
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            ModeRow(mode: view.transmitterMode, centeredThrottle: view.joystickMode ? nil : view.centeredThrottle, onPick: onMode, onCentered: onCentered)
            HStack(spacing: Space.s3) {
                Button(view.calibration.nextText.ifBlank("Calibrate")) {
                    if view.notReady != nil {
                        refusing = true
                    } else if view.startPrompt != nil {
                        prompting = true
                    } else {
                        onAction("nextButtonClicked")
                    }
                }
                .buttonStyle(.filled)
                .disabled(!view.calibration.nextEnabled)
                Text(view.summary).font(.bodySmall).foregroundStyle(theme.colors.onSurfaceVariant)
            }
            .padding(.horizontal, Space.s1)
            .padding(.vertical, Space.s2)
            AdditionalSetup(onInvoke: onInvoke)
        }
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
        .alert(view.startPrompt?.first ?? "", isPresented: Binding(get: { prompting && view.startPrompt != nil }, set: { if !$0 { prompting = false } })) {
            Button("Ok") {
                prompting = false
                onAction("nextButtonClicked")
            }
        } message: {
            Text(view.startPrompt?.second ?? "")
        }
        .alert(view.notReady?.first ?? "", isPresented: Binding(get: { refusing && view.notReady != nil }, set: { if !$0 { refusing = false } })) {
            Button("Ok") { refusing = false }
        } message: {
            Text(view.notReady?.second ?? "")
        }
    }
}

let RADIO_SWITCHES_PAGE = "Radio Switches"
let RADIO_ENTER_ACTIONS = ["start", "open"]
let RADIO_LEAVE_ACTIONS = ["cancelButtonClicked", "close"]

private func radioAction(_ action: String) {
    offMain { _ = Qgc.invoke(radioCalAction(action)) }
}

struct RadioScreen: View {
    @QgcPath(RADIO_VIEW) private var json
    @State private var switchReads = 0
    @State private var switches: [ParameterRows] = []
    @State private var reversedSeen = false

    var body: some View {
        let view = radioView(json)
        Group {
            if let view, view.connected {
                screen(view)
            } else {
                RadioNotice(text: "Connect a vehicle to check its radio.")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .onAppear { offMainInOrder { RADIO_ENTER_ACTIONS.map(radioCalAction).forEach { Qgc.invoke($0) } } }
        .onDisappear { offMainInOrder { RADIO_LEAVE_ACTIONS.map(radioCalAction).forEach { Qgc.invoke($0) } } }
        .task(id: "\(switchReads)|\(String(describing: view?.connected))") {
            let read = await offMain { readPage(RADIO_SWITCHES_PAGE) }
            guard !Task.isCancelled else { return }
            switches = read
        }
    }

    private func screen(_ view: RadioView) -> some View {
        let reversed = view.calibration.throttleReversed
        return ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                if view.channelCount == 0 {
                    RadioNotice(text: "No transmitter signal. Turn the transmitter on and check the receiver is bound.")
                }
                if view.calibration.running {
                    CalibrationStep(cal: view.calibration, onAction: radioAction)
                } else {
                    CalibrationStart(
                        view: view,
                        onAction: radioAction,
                        onMode: { mode in offMain { _ = Qgc.set("\(RADIO_CAL).transmitterMode", mode) } },
                        onCentered: { centered in offMain { _ = Qgc.set("\(RADIO_CAL).centeredThrottle", centered) } },
                        onInvoke: { action, choice in
                            offMain {
                                let path = radioCalAction(action)
                                _ = choice.map { Qgc.invoke(path, $0) } ?? Qgc.invoke(path)
                            }
                        }
                    )
                }
                SectionHeader(text: "Attitude controls")
                ForEach(view.sticks, id: \.title) { AttitudeRow(stick: $0) }
                ForEach(switches, id: \.title) { section in
                    SectionHeader(text: sentenceCase(section.title))
                    ForEach(section.facts, id: \.path) { fact in
                        FactRow(fact: fact, onWrite: { switchReads += 1 })
                    }
                }
                SectionHeader(text: "Raw channel monitor")
                ForEach(view.channels, id: \.label) { channel in
                    HStack(spacing: Space.s3) {
                        Text(channel.label).font(.system(size: 12, design: .monospaced)).frame(width: 28, alignment: .leading)
                        PwmBar(fraction: channel.fraction)
                        Text(channel.valueText).font(.system(size: 12, design: .monospaced)).frame(width: 48, alignment: .leading)
                    }
                    .padding(.horizontal, Space.s5)
                    .padding(.vertical, 6)
                }
                FootNote(
                    text: "Move each stick and switch \u{2014} every channel you use should move here. "
                        + "Calibration asks you to hold each stick at its extremes in turn and "
                        + "rewrites the mapping, so do it with the propellers off."
                )
            }
        }
        .onChange(of: reversed, initial: true) { reversedSeen = false }
        .alert(THROTTLE_REVERSED_TITLE, isPresented: Binding(get: { reversed && !reversedSeen }, set: { if !$0 { reversedSeen = true } })) {
            Button("OK") { reversedSeen = true }
        } message: {
            Text(THROTTLE_REVERSED_TEXT)
        }
    }
}

func stickReading(_ stick: RadioStick) -> String {
    guard stick.mapped else { return "Not mapped" }
    return [
        stick.channel.map { "Ch \($0)" },
        stick.valueText.isBlank ? nil : "\(stick.valueText) \u{00b5}s",
        stick.reversed ? "reversed" : nil,
    ].compactMap { $0 }.joined(separator: " \u{00b7} ")
}
