import SwiftUI

private let CAL = "sensorsCal"
let SENSOR_FACTORY_RESET = "sensorSettings.factoryReset"

private let CAL_START_MS = 5000

func calibrationFailure(_ name: String, _ started: Bool) -> String? {
    started ? nil : "\(name) calibration did not start."
}

private func calibrationStatus() -> String {
    calibrationState(Qgc.get(CALIBRATION))?.statusText ?? ""
}

private func calibrationRunning() -> Bool {
    calibrationState(Qgc.get(CALIBRATION))?.inProgress == true
}

func calibrationBegan(_ running: Bool, _ statusBefore: String, _ statusNow: String) -> Bool {
    running || statusNow != statusBefore
}

struct RoutineCopy: Equatable {
    var instruction: String
    var warning: String = ""
}

func routineCopy(_ routine: CalibrationRoutine) -> RoutineCopy {
    RoutineCopy(instruction: routine.dialogHelp.ifBlank(routine.description), warning: routine.warning)
}

private struct SensorsNotice: View {
    let text: String

    var body: some View {
        Text(text)
            .font(.bodyLarge)
            .multilineTextAlignment(.center)
            .frame(maxWidth: .infinity)
            .padding(Space.s6)
    }
}

let ACCEL_ROUTINE = "accelerometer"
let CALIBRATION_COMPLETE = "Calibration complete"
private let MUST_REBOOT = "YOU MUST REBOOT YOUR VEHICLE AFTER EACH CALIBRATION."
private let COMPASS_QUALITY = "Shown in the indicator bars is the quality of the calibration for each compass.\n\n"
    + "- Green indicates a well functioning compass.\n"
    + "- Yellow indicates a questionable compass or calibration.\n"
    + "- Red indicates a compass which should not be used.\n\n"

let PX4_COMPASS_COMPLETE = "Compass calibration complete"
let PX4_REBOOT = "Reboot the vehicle prior to flight."

func postCalibrationTitle(_ routine: String?, _ px4: Bool) -> String {
    px4 && routine == COMPASS_ROUTINE ? PX4_COMPASS_COMPLETE : CALIBRATION_COMPLETE
}

func postCalibrationPrompt(_ routine: String?, _ helpText: String, _ completed: String, _ px4: Bool) -> String? {
    if px4 { return helpText == CALIBRATION_COMPLETE && routine == COMPASS_ROUTINE ? PX4_REBOOT : nil }
    if completed == COMPASS_ROUTINE { return COMPASS_QUALITY + MUST_REBOOT }
    if completed == ACCEL_ROUTINE { return MUST_REBOOT }
    return nil
}

private struct StartDialog: View {
    let calibration: CalibrationRoutine
    let fast: FastCompass?
    let onConfirm: (String, [Any]) -> Void
    let onDismiss: () -> Void
    @State private var simple = false
    @State private var fastChoice: FastCompassChoice?
    @Environment(\.theme) private var theme

    private var offersFast: FastCompass? { calibration.id == COMPASS_ROUTINE ? fast : nil }
    private var offersSimple: Bool { calibration.id == ACCEL_ROUTINE && !calibration.arguments.isEmpty }
    private var orientationFirst: Bool { calibration.id == ACCEL_ROUTINE || calibration.id == COMPASS_ROUTINE }

    var body: some View {
        let copy = routineCopy(calibration)
        SetupDialog(title: calibration.dialogTitle.ifBlank(calibration.title)) {
            if orientationFirst {
                SensorSettingsBlock(
                    calibrating: true,
                    showCompasses: calibration.id == COMPASS_ROUTINE,
                    onSimpleAccel: offersSimple ? { simple = $0 } : nil
                )
            }
            Text(copy.instruction)
            if let offered = offersFast, let choice = fastChoice {
                FastCompassBlock(fast: offered, choice: choice) { fastChoice = $0 }
            }
            if !copy.warning.isBlank {
                Text(copy.warning).font(.bodyMedium).foregroundStyle(theme.colors.error)
            }
        } buttons: {
            Button("Cancel", action: onDismiss)
            Button("OK") {
                confirm()
                onDismiss()
            }
            .buttonStyle(.borderedProminent)
        }
        .onChange(of: offersFast != nil, initial: true) { _, offered in
            fastChoice = offered ? offersFast.map { initialFastCompassChoice($0) } : nil
        }
    }

    private func confirm() {
        if let offered = offersFast, let choice = fastChoice, choice.enabled {
            onConfirm(offered.invocation, fastCompassArguments(offered, choice).map(\.any))
        } else if offersSimple {
            onConfirm(calibration.invocation, [simple])
        } else {
            onConfirm(calibration.invocation, calibration.arguments)
        }
    }
}

func positionsText(_ sides: [CalibrationSide]) -> String? {
    let shown = sides.filter(\.visible)
    let reached = shown.filter { $0.stage == "done" || $0.stage == "inProgress" }.count
    return shown.isEmpty ? nil : "\(max(reached, 1)) of \(shown.count) positions"
}

func sideImage(_ key: String, _ rotating: Bool) -> String {
    let base: String = switch key {
    case "UpsideDown": "cal_vehicle_upside_down"
    case "Left": "cal_vehicle_left"
    case "Right": "cal_vehicle_right"
    case "NoseDown": "cal_vehicle_nose_down"
    case "TailDown": "cal_vehicle_tail_down"
    default: "cal_vehicle_down"
    }
    return rotating ? base + "_rotate" : base
}

func sideStateText(_ side: CalibrationSide) -> String {
    switch side.stage {
    case "inProgress": side.rotate ? "Rotate" : "Hold still"
    case "done": "Done"
    default: "Pending"
    }
}

private struct OrientationGrid: View {
    let sides: [CalibrationSide]
    let px4: Bool
    @Environment(\.theme) private var theme

    var body: some View {
        LazyVGrid(columns: [GridItem(.adaptive(minimum: 104, maximum: 104), spacing: Space.s2)], alignment: .leading, spacing: Space.s2) {
            ForEach(sides.filter(\.visible), id: \.key) { side in
                let current = side.stage == "inProgress"
                let done = side.stage == "done"
                let tint = current ? theme.colors.primary : done ? theme.aircast.success : theme.colors.onSurface.opacity(0.5)
                VStack(spacing: 0) {
                    Image(sideImage(side.key, px4 && current && side.rotate))
                        .resizable()
                        .scaledToFit()
                        .frame(height: 72)
                        .accessibilityLabel(side.title)
                    Text(side.title).font(.labelSmall)
                    Text(sideStateText(side)).font(.labelMedium).fontWeight(current ? .bold : nil).foregroundStyle(tint)
                }
                .padding(6)
                .frame(width: 104)
                .overlay(RoundedRectangle(cornerRadius: Corner.medium).stroke(current || done ? tint : theme.colors.outlineVariant, lineWidth: 2))
            }
        }
    }
}

private let CALIBRATION_RING: CGFloat = 180

func routineIcon(_ id: String) -> Icon {
    switch id {
    case "accelerometer": .vibration
    case "compass", "compassMot": .explore
    case "levelHorizon": .straighten
    case "gyro": .sensors
    case "pressure": .height
    case "airspeed": .speed
    default: .build
    }
}

private struct RunningCalibration: View {
    let name: String
    let state: CalibrationState
    @Environment(\.theme) private var theme

    var body: some View {
        let progress = min(max(state.progress, 0), 1)
        ScrollView {
            VStack(alignment: .leading, spacing: Space.s4) {
                if state.showsSides, let positions = positionsText(state.sides) {
                    Text(positions).font(.labelLarge).foregroundStyle(theme.colors.primary)
                }
                Text(runningTitle(name)).font(.titleLarge)
                if !state.helpText.isBlank {
                    Text(state.helpText).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                ZStack {
                    Circle().stroke(theme.colors.surfaceContainerHighest, lineWidth: 10)
                    Circle()
                        .trim(from: 0, to: progress)
                        .stroke(theme.colors.primary, style: StrokeStyle(lineWidth: 10, lineCap: .round))
                        .rotationEffect(.degrees(-90))
                    VStack(spacing: 0) {
                        Image(.sensors).font(.system(size: 40)).foregroundStyle(theme.colors.primary)
                        Text("\(Int(state.progress * 100))%").font(.headlineMedium)
                    }
                }
                .frame(width: CALIBRATION_RING, height: CALIBRATION_RING)
                .frame(maxWidth: .infinity)
                .padding(.vertical, Space.s2)
                if state.showsSides {
                    OrientationGrid(sides: state.sides, px4: state.px4)
                }
                if !state.statusText.isBlank {
                    Text(state.statusText).font(.bodyMedium).foregroundStyle(theme.colors.onSurfaceVariant)
                }
                HStack(spacing: Space.s2) {
                    Spacer(minLength: 0)
                    Button("Cancel") { offMain { _ = Qgc.invoke("\(CAL).cancelCalibration") } }
                        .disabled(!state.cancelEnabled)
                    Button("Next") { offMain { _ = Qgc.invoke("\(CAL).nextClicked") } }
                        .buttonStyle(.borderedProminent)
                        .disabled(!state.nextEnabled)
                }
                if state.waitingForCancel {
                    Text(CANCEL_WAIT_TITLE).font(.titleSmall)
                    Text(CANCEL_WAIT_TEXT).font(.bodyMedium)
                }
            }
            .padding(Space.s4)
        }
    }
}

private func calibrationStarted(_ before: String) async -> Bool {
    let deadline = Date().addingTimeInterval(Double(CAL_START_MS) / 1000)
    while Date() < deadline, !Task.isCancelled {
        if await offMain({ calibrationBegan(calibrationRunning(), before, calibrationStatus()) }) { return true }
        try? await Task.sleep(for: .milliseconds(150))
    }
    return false
}

struct SensorsScreen: View {
    @HasVehicle private var hasVehicle
    @QgcPath(CALIBRATION) private var json
    @QgcPath(SENSOR_HEALTH) private var healthJson
    @State private var pending: CalibrationRoutine?
    @State private var showSettings = false
    @State private var confirmFactoryReset = false
    @State private var runningName = ""
    @State private var ranRoutine: String?
    @State private var rebootPrompt: String?
    @State private var wasInProgress = false
    @State private var notice: String?
    @State private var stillRunning = false
    @Environment(\.theme) private var theme

    var body: some View {
        let state = calibrationState(json)
        Group {
            if !hasVehicle {
                SensorsNotice(text: "Connect a vehicle to calibrate its sensors.")
            } else if let state {
                screen(state)
            } else {
                SensorsNotice(text: "Reading the vehicle's calibration state.")
            }
        }
        .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .top)
    }

    @ViewBuilder
    private func screen(_ state: CalibrationState) -> some View {
        Group {
            if state.inProgress {
                RunningCalibration(name: runningName, state: state)
            } else {
                overview(state)
            }
        }
        .background { BlocksNavigation(blocking: state.inProgress && state.px4, reason: CALIBRATION_BLOCK) }
        .sheet(isPresented: presented($rebootPrompt)) { rebootDialog(state) }
        .sheet(isPresented: $showSettings) {
            SetupDialog(title: state.settingsDialogTitle.ifBlank(state.settingsTitle)) {
                SensorSettingsBlock(calibrating: false, showCompasses: true)
            } buttons: {
                Button("OK") { showSettings = false }.buttonStyle(.borderedProminent)
            }
        }
        .sheet(isPresented: presented($pending)) {
            if let calibration = pending {
                StartDialog(
                    calibration: calibration,
                    fast: state.fastCompass,
                    onConfirm: { invocation, arguments in start(calibration, invocation, arguments) },
                    onDismiss: { pending = nil }
                )
            }
        }
        .onChange(of: state.inProgress, initial: true) { _, inProgress in
            if wasInProgress && !inProgress {
                rebootPrompt = postCalibrationPrompt(ranRoutine, state.helpText, state.completed, state.px4)
            }
            wasInProgress = inProgress
            stillRunning = inProgress
        }
        .onDisappear {
            if stillRunning { offMain { _ = Qgc.invoke("\(CAL).cancelCalibration") } }
        }
        .alert("Factory reset", isPresented: $confirmFactoryReset) {
            Button("Cancel", role: .cancel) {}
            Button("Reset", role: .destructive) {
                Task { notice = await offMain { Qgc.refusalOf(SENSOR_FACTORY_RESET) } }
            }
        } message: {
            Text("Reset every parameter on the vehicle to its factory default?")
        }
    }

    @ViewBuilder
    private func rebootDialog(_ state: CalibrationState) -> some View {
        if let prompt = rebootPrompt {
            SetupDialog(title: postCalibrationTitle(ranRoutine, state.px4)) {
                Text(prompt)
                if !state.px4 && state.completed == COMPASS_ROUTINE {
                    ForEach(state.compassResults, id: \.compass) { CompassFitnessBar(result: $0) }
                }
                if state.px4 && ranRoutine == COMPASS_ROUTINE { CompassOrientations() }
            } buttons: {
                Button("Close") { rebootPrompt = nil }
                Button("Reboot vehicle") {
                    rebootPrompt = nil
                    offMain { _ = Qgc.invoke(REBOOT_VEHICLE) }
                }
                .buttonStyle(.borderedProminent)
            }
        }
    }

    private func start(_ calibration: CalibrationRoutine, _ invocation: String, _ arguments: [Any]) {
        runningName = calibration.title
        ranRoutine = calibration.id
        notice = nil
        let sent = arguments.map { Optional($0) }
        Task {
            let before = await offMain { calibrationStatus() }
            let dispatched = await offMain { Qgc.call(invocation, arguments: sent)?["ok"].bool ?? false }
            let started = dispatched ? await calibrationStarted(before) : false
            notice = calibrationFailure(calibration.title, started)
        }
    }

    private func overview(_ state: CalibrationState) -> some View {
        let health = sensorHealth(healthJson)
        return ScrollView {
            LazyVStack(alignment: .leading, spacing: 0) {
                if let reading = health, reading.available, !reading.sensors.isEmpty {
                    SectionHeader(text: "Sensor health")
                    let line = healthSummary(reading)
                    if !line.isBlank {
                        Text(line)
                            .font(.bodyMedium)
                            .foregroundStyle(theme.colors.error)
                            .padding(.horizontal, Space.s5)
                            .padding(.vertical, Space.s1)
                    }
                    ForEach(reading.sensors, id: \.name) { sensor in
                        SetupRow(
                            title: sensor.name,
                            status: sensor.label,
                            state: sensor.state == "healthy" ? .Done : sensor.state == "unhealthy" ? .NeedsAttention : .Neutral,
                            icon: .sensors
                        )
                    }
                }
                SectionHeader(text: "Calibration")
                if let message = notice {
                    Text(message)
                        .font(.bodySmall)
                        .foregroundStyle(theme.colors.error)
                        .padding(.horizontal, Space.s5)
                        .padding(.vertical, Space.s2)
                }
                ForEach(state.routines, id: \.id) { routine in
                    SetupRow(
                        title: sentenceCase(routine.title),
                        status: routine.spinsPropeller ? "Spins the motors" : routine.status,
                        state: routine.status == "Not calibrated" ? .NeedsAttention : routine.status == "Calibrated" ? .Done : .Neutral,
                        onClick: routine.enabled ? { pending = routine } : nil,
                        icon: routineIcon(routine.id)
                    )
                }
                SetupRow(title: sentenceCase(state.settingsTitle), status: "", state: .Neutral, onClick: { showSettings = true }, icon: .tune)
                if state.px4 {
                    SetupRow(title: "Factory reset", status: "", state: .NeedsAttention, onClick: { confirmFactoryReset = true }, icon: .delete)
                }
                if !state.statusText.isBlank {
                    SectionHeader(text: "Last calibration")
                    Text(state.statusText)
                        .font(.system(size: 14, design: .monospaced))
                        .padding(.horizontal, Space.s5)
                }
                FootNote(
                    text: "Calibrate where the aircraft will fly, away from metal, with the "
                        + "propellers off. CompassMot is the exception and says so when you "
                        + "open it: it runs the motors, with the propellers inverted."
                )
            }
        }
    }
}

private struct CompassFitnessBar: View {
    let result: CompassResult

    var body: some View {
        VStack(alignment: .leading, spacing: 2) {
            Text("Compass \(result.compass)").font(.labelMedium)
            GeometryReader { geometry in
                let width = geometry.size.width
                let fraction = { (part: Double) in result.range > 0 && part.isFinite ? max(part / result.range, 0) : 0 }
                ZStack(alignment: .leading) {
                    HStack(spacing: 0) {
                        Color(hex: 0x008000).frame(width: width * fraction(result.green))
                        Color.yellow.frame(width: width * fraction(result.yellow - result.green))
                        Color.red.frame(width: width * fraction(result.range - result.yellow))
                    }
                    Circle()
                        .fill(Color.white)
                        .overlay(Circle().stroke(Color.black, lineWidth: 1))
                        .frame(width: 11, height: 11)
                        .offset(x: width * (result.position.isFinite ? result.position : 0) - 5.5)
                }
            }
            .frame(height: 16)
        }
    }
}
