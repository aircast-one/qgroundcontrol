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

enum ActionKind { case Hover, Camera, Gimbal, Mode, Turn }

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

func activeActions(_ hold: WaypointHold?, _ yaw: WaypointYaw?, _ choices: CameraChoices?, _ extras: CameraExtras?) -> [WaypointAction] {
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
    ].compactMap { $0 }
}

func offeredActions(_ hold: WaypointHold?, _ yaw: WaypointYaw?, _ choices: CameraChoices?, _ extras: CameraExtras?) -> [ActionOffer] {
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
    ]
    return (hover + camera + rest).compactMap { $0 }
}

struct WaypointActions: View {
    let index: Int
    let hold: WaypointHold?
    let yaw: WaypointYaw?
    @Environment(\.theme) private var theme
    @State private var revision = 0
    @State private var camera: JSON?
    @State private var editing: ActionKind?

    var body: some View {
        let choices = cameraChoices(camera)
        let extras = cameraExtras(camera)
        let active = activeActions(hold, yaw, choices, extras)
        let offers = offeredActions(hold, yaw, choices, extras)
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
                                    Text("Add")
                                }
                                .font(.labelLarge)
                                .foregroundStyle(theme.colors.primary)
                                .padding(.horizontal, 12)
                                .frame(minHeight: 40)
                                .contentShape(Rectangle())
                            }
                        }
                    }
                    if active.isEmpty {
                        Text("Tap Add to take a photo, film, hover or turn here.")
                            .font(.bodySmall)
                            .foregroundStyle(theme.colors.onSurfaceVariant)
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
