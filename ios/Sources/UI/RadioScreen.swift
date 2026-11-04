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
                Text(stick.title).frame(maxWidth: .infinity, alignment: .leading)
                Text(stickReading(stick))
                    .font(.subheadline)
                    .foregroundStyle(stick.mapped ? Color.secondary : theme.colors.error)
            }
            if stick.mapped { PwmBar(fraction: stick.fraction) }
        }
        .padding(.vertical, 2)
    }
}

// The running calibration's section: the step to do, the stick diagram, and Skip / Cancel (Next is the page's
// primary action in the navigation bar).
private struct CalibrationStep: View {
    let cal: RadioCalibration
    let onAction: (String) -> Void

    var body: some View {
        Section {
            Text(calibrationStep(cal.statusText)).font(.headline)
            if cal.running {
                StickDiagram(positions: cal.stickPositions, single: false)
                    .frame(maxWidth: .infinity)
                    .padding(.vertical, Space.s2)
            }
            if cal.skipEnabled {
                Button("Skip") { onAction("skipButtonClicked") }
            }
            if cal.cancelEnabled {
                Button("Cancel calibration") { onAction("cancelButtonClicked") }
            }
        } header: {
            SectionHeader(text: "Calibration")
        }
    }
}

private struct ConfirmDialog: View {
    let prompt: RadioPrompt
    let onDismiss: () -> Void
    let onConfirm: (Int?) -> Void
    @State private var choice: Int?
    @Environment(\.theme) private var theme

    var body: some View {
        SetupDialog(
            title: prompt.title,
            cancel: DialogButton(title: "Cancel", action: onDismiss),
            primary: DialogButton(title: "OK") {
                onConfirm(choice)
                onDismiss()
            }
        ) {
            Text(prompt.body)
            ForEach(Array(prompt.choices.enumerated()), id: \.offset) { index, label in
                Button { choice = index } label: {
                    HStack(spacing: Space.s2) {
                        RadioIndicator(selected: index == choice)
                        Text(label).foregroundStyle(theme.colors.onSurface)
                    }
                    .padding(.vertical, Space.s2)
                    .frame(maxWidth: .infinity, alignment: .leading)
                    .contentShape(Rectangle())
                }
                .buttonStyle(.plain)
            }
        }
        .onChange(of: prompt, initial: true) { _, shown in choice = shown.choices.indices.last }
    }
}

let RADIO_SWITCHES_PAGE = "Radio Switches"
let RADIO_ENTER_ACTIONS = ["start", "open"]
let RADIO_LEAVE_ACTIONS = ["cancelButtonClicked", "close"]

private func radioAction(_ action: String) {
    offMain { _ = Qgc.invoke(radioCalAction(action)) }
}

private func radioInvoke(_ action: String, _ choice: Int?) {
    offMain {
        let path = radioCalAction(action)
        _ = choice.map { Qgc.invoke(path, $0) } ?? Qgc.invoke(path)
    }
}

private let RADIO_MONITOR_NOTE = "Move each stick and switch \u{2014} every channel you use should move here. "
    + "Calibration asks you to hold each stick at its extremes in turn and "
    + "rewrites the mapping, so do it with the propellers off."

private struct SwitchLoad: Equatable {
    let reads: Int
    let connected: Bool?
}

struct RadioScreen: View {
    @QgcPath(RADIO_VIEW) private var json
    @State private var switchReads = 0
    @State private var switches: [ParameterRows] = []
    @State private var reversedSeen = false
    @State private var prompting = false
    @State private var refusing = false
    @State private var prompt: RadioPrompt?
    @Environment(\.theme) private var theme

    var body: some View {
        let view = radioView(json)
        ZStack(alignment: .top) {
            if let view, view.connected {
                screen(view)
            } else {
                RadioNotice(text: "Connect a vehicle to check its radio.")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
        .onAppear { offMainInOrder { RADIO_ENTER_ACTIONS.map(radioCalAction).forEach { Qgc.invoke($0) } } }
        .onDisappear { offMainInOrder { RADIO_LEAVE_ACTIONS.map(radioCalAction).forEach { Qgc.invoke($0) } } }
        .task(id: SwitchLoad(reads: switchReads, connected: view?.connected)) {
            let read = await offMain { readPage(RADIO_SWITCHES_PAGE) }
            guard !Task.isCancelled else { return }
            switches = read
        }
    }

    // Calibrate: refused while the vehicle says it is not ready, asked first when it has a start prompt.
    private func start(_ view: RadioView) {
        if view.notReady != nil {
            refusing = true
        } else if view.startPrompt != nil {
            prompting = true
        } else {
            radioAction("nextButtonClicked")
        }
    }

    private func screen(_ view: RadioView) -> some View {
        let reversed = view.calibration.throttleReversed
        let running = view.calibration.running
        let asked = prompt
        return NativeForm {
            if view.channelCount == 0 {
                Section {
                    Text("No transmitter signal. Turn the transmitter on and check the receiver is bound.")
                        .foregroundStyle(theme.aircast.warning)
                }
            }
            if running {
                CalibrationStep(cal: view.calibration, onAction: radioAction)
            } else {
                Section {
                    Picker("Transmitter mode", selection: Binding(get: { view.transmitterMode }, set: { mode in
                        offMain { _ = Qgc.set("\(RADIO_CAL).transmitterMode", mode) }
                    })) {
                        ForEach(1...4, id: \.self) { Text("Mode \($0)").tag($0) }
                    }
                    .pickerStyle(.segmented)
                    if !view.joystickMode {
                        Toggle("Centered throttle", isOn: Binding(get: { view.centeredThrottle }, set: { centered in
                            offMain { _ = Qgc.set("\(RADIO_CAL).centeredThrottle", centered) }
                        }))
                    }
                } header: {
                    SectionHeader(text: "Transmitter mode")
                } footer: {
                    Text([view.summary, "The mode sets which stick is the throttle."].filter { !$0.isBlank }.joined(separator: "\n"))
                }
                Section {
                    ForEach(RADIO_PROMPTS, id: \.action) { entry in
                        Button(entry.title) { prompt = entry }
                    }
                } header: {
                    SectionHeader(text: "Additional setup")
                }
            }
            Section {
                ForEach(view.sticks, id: \.title) { AttitudeRow(stick: $0) }
            } header: {
                SectionHeader(text: "Attitude controls")
            }
            ForEach(switches, id: \.title) { section in
                Section {
                    ForEach(section.facts, id: \.path) { fact in
                        FactRow(fact: fact, onWrite: { switchReads += 1 })
                    }
                } header: {
                    SectionHeader(text: sentenceCase(section.title))
                }
            }
            Section {
                ForEach(view.channels, id: \.label) { channel in
                    HStack(spacing: Space.s3) {
                        Text(channel.label).font(.system(size: 12, design: .monospaced)).frame(width: 28, alignment: .leading)
                        PwmBar(fraction: channel.fraction)
                        Text(channel.valueText).font(.system(size: 12, design: .monospaced)).frame(width: 48, alignment: .leading)
                    }
                }
            } header: {
                SectionHeader(text: "Raw channel monitor")
            } footer: {
                Text(RADIO_MONITOR_NOTE)
            }
        }
        .pagePrimaryAction(view.calibration.nextText.ifBlank(running ? "Next" : "Calibrate"), enabled: view.calibration.nextEnabled) {
            if running { radioAction("nextButtonClicked") } else { start(view) }
        }
        .sheet(isPresented: presented($prompt)) {
            if let asked {
                ConfirmDialog(prompt: asked, onDismiss: { prompt = nil }, onConfirm: { radioInvoke(asked.action, $0) })
            }
        }
        .alert(view.startPrompt?.first ?? "", isPresented: Binding(get: { prompting && view.startPrompt != nil }, set: { if !$0 { prompting = false } })) {
            Button("Ok") {
                prompting = false
                radioAction("nextButtonClicked")
            }
            Button("Cancel", role: .cancel) { prompting = false }
        } message: {
            Text(view.startPrompt?.second ?? "")
        }
        .alert(view.notReady?.first ?? "", isPresented: Binding(get: { refusing && view.notReady != nil }, set: { if !$0 { refusing = false } })) {
            Button("Ok") { refusing = false }
        } message: {
            Text(view.notReady?.second ?? "")
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
