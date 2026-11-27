import SwiftUI

struct WaypointYaw: Equatable {
    var degrees: Double?
    var units: String
    var path: String
}

func waypointYaw(_ view: JSON?) -> WaypointYaw? {
    guard let yaw = view?["yaw"], yaw.object != nil else { return nil }
    let made = WaypointYaw(
        degrees: yaw["value"].double.flatMap { $0.isNaN ? nil : $0 },
        units: yaw["units"].string.ifBlank("deg"),
        path: yaw["path"].string
    )
    return made.path.isBlank ? nil : made
}

struct WaypointDrop: Equatable {
    var index: Int?
    var wait: Double
    var waitOffered: Bool
    var servo: Bool
    var output: Double?
    var pwm: Double?
    var bay: Int?
}

func waypointDrop(_ view: JSON?) -> WaypointDrop? {
    guard let drop = view?["drop"], drop.object != nil else { return nil }
    return WaypointDrop(
        index: drop["index"].isNull ? nil : drop["index"].int(0),
        wait: drop["wait"].nonNanDouble ?? 0,
        waitOffered: drop["waitOffered"].bool(false),
        servo: drop["servo"].bool(false),
        output: drop["output"].nonNanDouble,
        pwm: drop["pwm"].nonNanDouble,
        bay: drop["bay"].isNull ? nil : drop["bay"].int(0)
    )
}

let PLACE_DROP = "core.plan.placeDrop"
let REMOVE_DROP = "core.plan.removeDrop"
let SET_DROP_RELEASE = "core.plan.setDropRelease"
private let OUTPUT_RANGE = 1.0...32.0
private let PWM_RANGE = 500.0...2500.0
private let WAIT_RANGE = 0.0...600.0
private let PWM_STEP = 50.0
private let DROP_NOTE = "Photos and camera moves happen on arrival; the drone then waits this long over the point and drops. The servo and PWM belong to this drop, and the next drop starts from them."
private let PLANE_DROP_NOTE = "A plane flies through its drop without stopping. The servo and PWM belong to this drop, and the next drop starts from them."

func dropText(_ drop: WaypointDrop) -> String {
    let release = drop.output.flatMap { output in drop.pwm.map { pwm in "Servo \(trimmedNumber(output)) to \(trimmedNumber(pwm))" } } ?? "Gripper"
    return [drop.bay.map { "Bay \($0)" } ?? "", release, drop.wait > 0 ? "after \(trimmedNumber(drop.wait)) s" : ""]
        .filter { !$0.isEmpty }
        .joined(separator: " · ")
}

enum ActionKind { case Hover, Camera, Gimbal, Mode, Turn, Drop }

struct WaypointAction: Equatable {
    let kind: ActionKind
    let title: String
    let value: String
    let icon: Icon
}

struct ActionOffer: Equatable {
    let title: String
    let icon: Icon
    let kind: ActionKind
    var choice: Int = -1
}

private let DEFAULT_HOVER_SECONDS = 5.0
private let NO_CAMERA_ACTION = 0
private let HOVER_RANGE = 0.0...600.0
private let PITCH_RANGE = -90.0...30.0
private let HEADING_RANGE = -180.0...180.0
private let INTERVAL_RANGE = 1.0...3600.0
private let ANGLE_STEP = 5.0
private let DROP_TITLE = "Drop payload"
private let DROP_SUBJECT = "The drop"

private func mentions(_ label: String, _ word: String) -> Bool { label.range(of: word, options: .caseInsensitive) != nil }

func cameraIcon(_ label: String) -> Icon {
    let video = mentions(label, "video")
    let stop = mentions(label, "stop")
    return video && stop ? .planVideoOff : video ? .planVideo : stop ? .planStop : .planPhoto
}

func degreesText(_ value: Double) -> String { "\(trimmedNumber(value))°" }

func cameraInterval(_ extras: CameraExtras?) -> String {
    extras?.intervalTime.map { "every \(trimmedNumber($0)) s" }
        ?? extras.flatMap { camera in camera.intervalDistance.map { "every \(trimmedNumber($0)) \(camera.distanceUnits)" } }
        ?? ""
}

func startingChoice(_ labels: [String], _ subject: String) -> Int? {
    labels.firstIndex { mentions($0, subject) && !mentions($0, "stop") }.flatMap { $0 > NO_CAMERA_ACTION ? $0 : nil }
}

func activeActions(_ hold: WaypointHold?, _ yaw: WaypointYaw?, _ choices: CameraChoices?, _ extras: CameraExtras?, _ drop: WaypointDrop? = nil) -> [WaypointAction] {
    [
        hold.flatMap { $0.seconds > 0 ? WaypointAction(kind: .Hover, title: "Hover", value: "\(trimmedNumber($0.seconds)) \($0.units)", icon: .planTimer) : nil },
        choices.flatMap { picked -> WaypointAction? in
            guard picked.chosen > NO_CAMERA_ACTION else { return nil }
            let label = picked.labels.indices.contains(picked.chosen) ? picked.labels[picked.chosen] : ""
            return WaypointAction(kind: .Camera, title: sentenceCase(label), value: cameraInterval(extras), icon: cameraIcon(label))
        },
        extras.flatMap { $0.commandsGimbal ? WaypointAction(kind: .Gimbal, title: "Gimbal", value: "\(degreesText($0.pitch)) / \(degreesText($0.yaw))", icon: .planGimbal) : nil },
        extras.flatMap { camera in
            camera.modeSupported && camera.commandsMode
                ? WaypointAction(kind: .Mode, title: "Camera mode", value: CAMERA_MODES.indices.contains(camera.mode) ? CAMERA_MODES[camera.mode] : "", icon: .planTune)
                : nil
        },
        yaw?.degrees.map { WaypointAction(kind: .Turn, title: "Turn aircraft", value: degreesText($0), icon: .planTurn) },
        drop.flatMap { placed in placed.index.map { _ in WaypointAction(kind: .Drop, title: DROP_TITLE, value: dropText(placed), icon: .planDrop) } },
    ].compactMap { $0 }
}

func offeredActions(_ hold: WaypointHold?, _ yaw: WaypointYaw?, _ choices: CameraChoices?, _ extras: CameraExtras?, _ drop: WaypointDrop? = nil) -> [ActionOffer] {
    let hover: [ActionOffer?] = [hold.flatMap { $0.seconds <= 0 ? ActionOffer(title: "Hover", icon: .planTimer, kind: .Hover) : nil }]
    let camera: [ActionOffer?] = choices.map { camera in
        camera.chosen <= NO_CAMERA_ACTION ? [
            startingChoice(camera.labels, "photo").map { ActionOffer(title: "Photo", icon: .planPhoto, kind: .Camera, choice: $0) },
            startingChoice(camera.labels, "video").map { ActionOffer(title: "Video", icon: .planVideo, kind: .Camera, choice: $0) },
        ] : []
    } ?? []
    let rest: [ActionOffer?] = [
        extras.flatMap { !$0.commandsGimbal ? ActionOffer(title: "Tilt camera", icon: .planGimbal, kind: .Gimbal) : nil },
        extras.flatMap { $0.modeSupported && !$0.commandsMode ? ActionOffer(title: "Camera mode", icon: .planTune, kind: .Mode) : nil },
        yaw.flatMap { $0.degrees == nil ? ActionOffer(title: "Turn aircraft", icon: .planTurn, kind: .Turn) : nil },
        drop.flatMap { $0.index == nil ? ActionOffer(title: DROP_TITLE, icon: .planDrop, kind: .Drop) : nil },
    ]
    return (hover + camera + rest).compactMap { $0 }
}

private let OFFER_VERBS = [("Photo", "take a photo"), ("Video", "film"), ("Hover", "hover"), ("Turn aircraft", "turn"), (DROP_TITLE, "drop")]

func addActionsHint(_ offers: [ActionOffer]) -> String {
    let verbs = OFFER_VERBS.filter { verb in offers.contains { $0.title == verb.0 } }.map { $0.1 }
    guard let last = verbs.last else { return "Tap Add action to give the camera something to do here." }
    return "Tap Add action to \(verbs.count == 1 ? last : "\(verbs.dropLast().joined(separator: ", ")) or \(last)") here."
}

struct WaypointActions: View {
    let index: Int
    let hold: WaypointHold?
    let yaw: WaypointYaw?
    var drop: WaypointDrop? = nil
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var camera: JSON?
    @State private var editing: ActionKind?
    @State private var refusal: String?

    var body: some View {
        let choices = cameraChoices(camera)
        let extras = cameraExtras(camera)
        let active = activeActions(hold, yaw, choices, extras, drop)
        let offers = offeredActions(hold, yaw, choices, extras, drop)
        VStack(alignment: .leading, spacing: 0) {
            if !active.isEmpty || !offers.isEmpty {
                VStack(alignment: .leading, spacing: 0) {
                    HStack {
                        Text("Actions").font(.titleSmall).frame(maxWidth: .infinity, alignment: .leading)
                        if !offers.isEmpty {
                            Menu {
                                ForEach(offers, id: \.title) { offer in
                                    Button { add(offer) } label: { Label(offer.title, systemImage: offer.icon.rawValue) }
                                }
                            } label: {
                                HStack(spacing: Space.s1) {
                                    Image(.add).font(.system(size: 16))
                                    Text("Add action")
                                }
                                .font(.labelLarge)
                                .foregroundStyle(theme.colors.onSecondaryContainer)
                                .padding(.horizontal, 16)
                                .frame(minHeight: 40)
                                .background(theme.colors.secondaryContainer, in: Capsule())
                                .contentShape(Capsule())
                            }
                        }
                    }
                    if active.isEmpty {
                        Text(addActionsHint(offers))
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                    }
                    if let refusal {
                        Text(refusal)
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.error)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                    PlanFlowRow(spacing: 8, lineSpacing: 8) {
                        ForEach(active, id: \.kind) { action in
                            ActionChip(action: action, onClick: { editing = action.kind }, onRemove: { remove(action.kind) })
                        }
                    }
                }
                .padding(.vertical, 4)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .task(id: [index, revision]) {
            let at = index
            camera = await offMain { ItemCameraBridge.read(at) }
        }
        .background {
            if let kind = editing {
                editor(kind, active, offers, choices, extras)
            }
        }
    }

    private func editor(_ kind: ActionKind, _ active: [WaypointAction], _ offers: [ActionOffer], _ choices: CameraChoices?, _ extras: CameraExtras?) -> some View {
        PlanDialog(title: active.first { $0.kind == kind }?.title ?? offers.first { $0.kind == kind }?.title ?? "", onDismiss: { editing = nil }) {
            VStack(alignment: .leading, spacing: 0) {
                switch kind {
                case .Hover:
                    if let hold {
                        SettingStepper(label: "Hover", value: hold.seconds, unit: hold.units, step: 1.0, onSet: setHold, range: HOVER_RANGE, slider: true)
                    }
                case .Turn:
                    SettingStepper(label: "Heading", value: yaw?.degrees ?? 0.0, unit: "°", step: ANGLE_STEP, onSet: { setYaw($0) }, range: HEADING_RANGE, slider: true)
                case .Gimbal:
                    if let extras {
                        SettingStepper(label: "Pitch", value: extras.pitch, unit: "°", step: ANGLE_STEP, onSet: { setCamera("gimbalPitch", $0) }, range: extras.pitchRange ?? PITCH_RANGE, slider: true)
                        SettingStepper(label: "Yaw", value: extras.yaw, unit: "°", step: ANGLE_STEP, onSet: { setCamera("gimbalYaw", $0) }, range: extras.yawRange ?? HEADING_RANGE, slider: true)
                    }
                case .Drop:
                    if let drop, drop.index != nil {
                        HStack(spacing: Space.s2) {
                            CameraChip(label: "Gripper", selected: !drop.servo) { releaseKind(false) }
                            CameraChip(label: "Servo", selected: drop.servo) { releaseKind(true) }
                        }
                        if let output = drop.output, let pwm = drop.pwm {
                            SettingStepper(label: "Servo output", value: output, unit: "", step: 1.0, onSet: { setRelease(["output": $0]) }, range: OUTPUT_RANGE)
                            SettingStepper(label: "Release PWM", value: pwm, unit: "µs", step: PWM_STEP, onSet: { setRelease(["pwm": $0]) }, range: PWM_RANGE)
                        }
                        if drop.waitOffered {
                            SettingStepper(label: "Wait before dropping", value: drop.wait, unit: "s", step: 1.0, onSet: { placeDrop($0) }, range: WAIT_RANGE)
                        }
                        Text(drop.waitOffered ? DROP_NOTE : PLANE_DROP_NOTE)
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
                            .fixedSize(horizontal: false, vertical: true)
                    }
                case .Mode:
                    HStack(spacing: Space.s2) {
                        ForEach(Array(CAMERA_MODES.enumerated()), id: \.offset) { at, label in
                            CameraChip(label: label, selected: extras?.mode == at) { setCamera("cameraMode", at) }
                        }
                    }
                case .Camera:
                    ForEach(Array((choices?.labels ?? []).enumerated()).dropFirst(), id: \.offset) { at, label in
                        Button { choose(at) } label: {
                            HStack(spacing: Space.s3) {
                                RadioIndicator(selected: choices?.chosen == at)
                                Text(sentenceCase(label)).font(.bodyLarge).foregroundStyle(theme.colors.onSurface)
                            }
                            .frame(maxWidth: .infinity, minHeight: 48, alignment: .leading)
                            .contentShape(Rectangle())
                        }
                        .buttonStyle(.plain)
                    }
                    if let seconds = extras?.intervalTime {
                        SettingStepper(label: "Every", value: seconds, unit: "s", step: 1.0, onSet: { setCamera("cameraPhotoIntervalTime", $0) }, range: INTERVAL_RANGE)
                    }
                    if let extras, let distance = extras.intervalDistance {
                        SettingStepper(label: "Every", value: distance, unit: extras.distanceUnits, step: 1.0, onSet: { setCamera("cameraPhotoIntervalDistance", $0) }, range: INTERVAL_RANGE)
                    }
                }
            }
        } buttons: {
            Button("Done") { editing = nil }
        }
    }

    private func add(_ offer: ActionOffer) {
        switch offer.kind {
        case .Hover: setHold(DEFAULT_HOVER_SECONDS)
        case .Camera: choose(offer.choice)
        case .Gimbal: setCamera("specifyGimbal", true)
        case .Mode: setCamera("specifyCameraMode", true)
        case .Turn: setYaw(0.0)
        case .Drop:
            let waypoint = index
            refusing(opens: .Drop) { Qgc.refusalOf(PLACE_DROP, waypoint, subject: DROP_SUBJECT) }
            return
        }
        editing = offer.kind
    }

    private func remove(_ kind: ActionKind) {
        switch kind {
        case .Hover: setHold(0.0)
        case .Camera: choose(NO_CAMERA_ACTION)
        case .Gimbal: setCamera("specifyGimbal", false)
        case .Mode: setCamera("specifyCameraMode", false)
        case .Turn: setYaw(nil)
        case .Drop: removeDrop()
        }
    }

    private func placeDrop(_ wait: Double) {
        let waypoint = index
        refusing(opens: nil) { Qgc.refusalOf(PLACE_DROP, waypoint, wait, subject: DROP_SUBJECT) }
    }

    private func releaseKind(_ servo: Bool) {
        guard servo != drop?.servo else { return }
        setRelease(["servo": servo])
    }

    private func setRelease(_ change: [String: any Sendable]) {
        let waypoint = index
        refusing(opens: nil) { Qgc.refusalOf(SET_DROP_RELEASE, waypoint, change, subject: DROP_SUBJECT) }
    }

    private func removeDrop() {
        let waypoint = index
        refusing(opens: nil) { Qgc.refusalOf(REMOVE_DROP, waypoint, subject: DROP_SUBJECT) }
    }

    private func refusing(opens: ActionKind?, _ work: @escaping @Sendable () -> String?) {
        Task { @MainActor in
            refusal = await offMain(work)
            if refusal == nil, let opens { editing = opens }
            revision += 1
        }
    }

    private func write(_ work: @escaping @Sendable () -> Bool) {
        Task { @MainActor in
            let _: Bool = await offMain(work)
            revision += 1
        }
    }

    private func setHold(_ seconds: Double) {
        guard let path = hold?.path else { return }
        write { setOk(path, seconds) }
    }

    private func setYaw(_ degrees: Double?) {
        guard let path = yaw?.path else { return }
        write { setOk(path, degrees) }
    }

    private func setCamera(_ member: String, _ value: Any) {
        let at = index
        write { ItemCameraBridge.set(at, member, value) }
    }

    private func choose(_ choice: Int) {
        let at = index
        write { ItemCameraBridge.chooseAction(at, choice) }
    }
}

private struct ActionChip: View {
    let action: WaypointAction
    let onClick: () -> Void
    let onRemove: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(spacing: 0) {
            Button(action: onClick) {
                HStack(spacing: Space.s2) {
                    Image(action.icon).font(.system(size: 16)).frame(width: 20, height: 20)
                    VStack(alignment: .leading, spacing: 0) {
                        Text(action.title).font(.labelLarge)
                        if !action.value.isBlank {
                            Text(action.value).font(.labelSmall)
                        }
                    }
                }
                .padding(.leading, 12)
                .padding(.vertical, 6)
                .contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            Button(action: onRemove) {
                Image(.close).font(.system(size: 14)).frame(width: 40, height: 40).contentShape(Rectangle())
            }
            .buttonStyle(.plain)
            .accessibilityLabel("Remove \(action.title)")
        }
        .foregroundStyle(theme.colors.onSecondaryContainer)
        .background(theme.colors.secondaryContainer, in: RoundedRectangle(cornerRadius: 14))
    }
}
