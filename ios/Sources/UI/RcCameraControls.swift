import SwiftUI

private let FLY_VIEW_SETTINGS = "settings.flyViewSettings"
private let RAIL_SLIDER_LENGTH: CGFloat = 140
private let RAIL_SLIDER_THICKNESS: CGFloat = 40
private let GIMBAL_VIEW = "view.gimbalIndicator"

private extension View {
    func railSlider() -> some View {
        frame(width: RAIL_SLIDER_LENGTH)
            .rotationEffect(.degrees(-90))
            .frame(width: RAIL_SLIDER_THICKNESS, height: RAIL_SLIDER_LENGTH)
    }
}

struct RcCameraChannels: Equatable {
    let tilt: Int
    let pan: Int
    let zoom: Int
    let light: Int
    let record: Int

    var any: Bool { [tilt, pan, zoom, light, record].contains { $0 > 0 } }
}

func rcCameraChannels(tilt: Int, pan: Int, zoom: Int, light: Int, record: Int) -> RcCameraChannels {
    RcCameraChannels(tilt: tilt, pan: pan == tilt ? 0 : pan, zoom: zoom, light: light, record: record)
}

func cameraRecording(recordChannel: Int, channelRecording: Bool, streamRecording: Bool) -> Bool {
    streamRecording || (recordChannel > 0 && channelRecording)
}

private func send(_ channel: Int, _ pwm: Int) { cameraRcControls.hold(channel, pwm) }

private func uptimeMs() -> Int64 { Int64(ProcessInfo.processInfo.systemUptime * 1000) }

@propertyWrapper
private struct ChannelSetting: DynamicProperty {
    @QgcValue private var value: JSON

    init(_ name: String) { _value = QgcValue(settingControl("\(FLY_VIEW_SETTINGS).\(name)")) }

    var wrappedValue: Int {
        if case .number(let number) = value, number.isFinite { return Int(number) }
        return 0
    }
}

private struct PwmSlider: View {
    let label: String
    let channel: Int
    let pwm: Int
    let onPwm: (Int) -> Void
    @State private var lastSent: Int64 = 0

    var body: some View {
        VStack(spacing: 0) {
            Slider(
                value: Binding(get: { Double(pwm) }, set: { raw in
                    let next = Int(raw)
                    onPwm(next)
                    let now = uptimeMs()
                    if rcSendDue(now, lastSent, false) {
                        lastSent = now
                        send(channel, next)
                    }
                }),
                in: Double(PWM_MIN)...Double(PWM_MAX),
                onEditingChanged: { editing in if !editing { send(channel, pwm) } }
            )
            .railSlider()
            Text(label).font(.labelMedium)
        }
        .onChange(of: channel) { lastSent = 0 }
    }
}

private let gimbalSends = DispatchQueue(label: "one.aircast.gimbal-sends", qos: .userInitiated)

private func sendTilt(_ pitch: Double) {
    gimbalSends.async { _ = gimbalRefusal(Qgc.call("gimbal.pitch", pitch)) }
}

private let GIMBAL_REFUSAL_MS = 4000
let GIMBAL_TILT_MIN = -90.0
let GIMBAL_TILT_MAX = 30.0
private let GIMBAL_IDLE_MS = 3000
private let GIMBAL_TRACK_ALPHA = 0.8
private let GIMBAL_REST_ALPHA = 0.25

private struct TiltWake: Equatable {
    let shown: Double
    let dragging: Double?
    let wake: Int
}

private struct GimbalTiltSlider: View {
    let pitch: Double?
    @State private var dragging: Double?
    @State private var lastSent: Int64 = 0
    @State private var active = true
    @State private var wake = 0

    var body: some View {
        let shown = dragging ?? min(max(pitch ?? 0, GIMBAL_TILT_MIN), GIMBAL_TILT_MAX)
        VStack(spacing: 0) {
            Text("\(Int(shown.rounded()))\u{00B0}")
                .font(.labelMedium)
                .onTapGesture { wake += 1 }
                .accessibilityAddTraits(.isButton)
                .accessibilityHint("Tilt the gimbal")
            ZStack {
                if !active {
                    Color.clear
                        .contentShape(Rectangle())
                        .onTapGesture { wake += 1 }
                        .accessibilityHint("Tilt the gimbal")
                }
                if active {
                    Slider(
                        value: Binding(get: { shown }, set: { next in
                            dragging = next
                            let now = uptimeMs()
                            if rcSendDue(now, lastSent, false) {
                                lastSent = now
                                sendTilt(next)
                            }
                        }),
                        in: GIMBAL_TILT_MIN...GIMBAL_TILT_MAX,
                        onEditingChanged: { editing in
                            guard !editing else { return }
                            dragging.map(sendTilt)
                            dragging = nil
                        }
                    )
                    .tint(Color.white.opacity(GIMBAL_TRACK_ALPHA))
                    .background(Capsule().fill(Color.white.opacity(GIMBAL_REST_ALPHA)).frame(height: 4))
                    .railSlider()
                    .accessibilityLabel("Gimbal tilt")
                    .transition(.opacity)
                }
            }
            .frame(width: RAIL_SLIDER_THICKNESS, height: RAIL_SLIDER_LENGTH)
            .animation(.default, value: active)
        }
        .task(id: TiltWake(shown: shown, dragging: dragging, wake: wake)) {
            active = true
            guard dragging == nil else { return }
            try? await Task.sleep(for: .milliseconds(GIMBAL_IDLE_MS))
            if !Task.isCancelled { active = false }
        }
    }
}

struct RcCameraControls: View {
    @Environment(\.theme) private var theme
    @HasVehicle private var hasVehicle
    @ChannelSetting("gimbalTiltChannel") private var tiltChannel
    @ChannelSetting("gimbalPanChannel") private var panChannel
    @ChannelSetting("cameraZoomChannel") private var zoomChannel
    @ChannelSetting("cameraLightChannel") private var lightChannel
    @ChannelSetting("cameraRecordChannel") private var recordChannel
    @QgcPath(GIMBAL_VIEW) private var gimbalJson
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @QgcBool(VIDEO_RECORDING_STATE) private var streamRecording
    @State private var tilt = PWM_CENTER
    @State private var pan = PWM_CENTER
    @State private var zoom = PWM_CENTER
    @State private var lightOn = false
    @State private var channelRecording = false
    @State private var gimbalRefused: String?
    @State private var options = false

    var body: some View {
        let channels = rcCameraChannels(tilt: tiltChannel, pan: panChannel, zoom: zoomChannel, light: lightChannel, record: recordChannel)
        let gimbalManager = gimbalJson?["shown"].bool == true
        let vehicleId = activeVehicleId(vehiclesJson)
        ZStack {
            if hasVehicle && (channels.any || gimbalManager) {
                controls(channels, gimbalManager)
            }
        }
        .onChange(of: vehicleId) {
            cameraRcControls.release()
            tilt = PWM_CENTER
            pan = PWM_CENTER
            zoom = PWM_CENTER
            lightOn = false
            channelRecording = false
        }
        .onDisappear { cameraRcControls.release() }
        .task(id: gimbalRefused) {
            guard gimbalRefused != nil else { return }
            try? await Task.sleep(for: .milliseconds(GIMBAL_REFUSAL_MS))
            if !Task.isCancelled { gimbalRefused = nil }
        }
    }

    private func controls(_ channels: RcCameraChannels, _ gimbalManager: Bool) -> some View {
        let gimbal = gimbalIndicator(gimbalJson)
        let recording = cameraRecording(recordChannel: channels.record, channelRecording: channelRecording, streamRecording: streamRecording)
        let rcGimbal = !gimbalManager && (channels.tilt > 0 || channels.pan > 0)
        return VStack(spacing: 4) {
            HStack(spacing: 4) {
                if let gimbal { GimbalTiltSlider(pitch: gimbal.pitchDegrees) }
                if !gimbalManager && channels.tilt > 0 { PwmSlider(label: "Tilt", channel: channels.tilt, pwm: tilt) { tilt = $0 } }
                if !gimbalManager && channels.pan > 0 { PwmSlider(label: "Pan", channel: channels.pan, pwm: pan) { pan = $0 } }
                if channels.zoom > 0 { PwmSlider(label: "Zoom", channel: channels.zoom, pwm: zoom) { zoom = $0 } }
            }
            if channels.record > 0 {
                CameraChip(label: "Record", selected: recording) {
                    let next = !recording
                    channelRecording = next
                    send(channels.record, next ? PWM_MAX : PWM_MIN)
                    offMainInOrder { VideoCommands.setRecording(next) }
                }
            }
            if channels.light > 0 || gimbal != nil || rcGimbal {
                Button { options = true } label: {
                    Image(.tune).font(.system(size: 20)).frame(width: 48, height: 48).contentShape(Rectangle())
                }
                .buttonStyle(.plain)
                .accessibilityLabel("Gimbal options")
            }
            if let gimbalRefused {
                Text(gimbalRefused).font(.labelSmall).foregroundStyle(theme.colors.error)
            }
        }
        .padding(4)
        .background(OpenOnRequest(name: "gimbal") { options = true })
        .background {
            if options {
                AircastSheet(onDismissRequest: { options = false }) {
                    optionsSheet(channels, gimbal, rcGimbal)
                }
            }
        }
    }

    private func optionsSheet(_ channels: RcCameraChannels, _ gimbal: GimbalIndicatorState?, _ rcGimbal: Bool) -> some View {
        VStack(alignment: .leading, spacing: 8) {
            Text("Gimbal").font(.titleLarge)
            HStack(spacing: 8) {
                if channels.light > 0 {
                    CameraChip(label: "Light", selected: lightOn) {
                        lightOn.toggle()
                        send(channels.light, lightOn ? PWM_MAX : PWM_MIN)
                    }
                }
                if let gimbal, gimbal.yawLockOffered {
                    CameraChip(label: gimbal.yawLockLabel, selected: gimbal.yawLocked) {
                        let locked = gimbal.yawLocked
                        Task { gimbalRefused = await offMain { gimbalRefusal(Qgc.call("gimbal.yawLock", !locked)) } }
                    }
                }
                if gimbal != nil {
                    Button("Recenter") { Task { gimbalRefused = await offMain { gimbalRefusal(Qgc.call("gimbal.center")) } } }
                        .buttonStyle(.borderless)
                }
                if rcGimbal {
                    Button("Recenter") {
                        tilt = PWM_CENTER
                        pan = PWM_CENTER
                        send(channels.tilt, PWM_CENTER)
                        send(channels.pan, PWM_CENTER)
                    }
                    .buttonStyle(.borderless)
                }
            }
            if let gimbalRefused {
                Text(gimbalRefused).font(.bodySmall).foregroundStyle(theme.colors.error)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, 16)
        .padding(.vertical, 8)
    }
}
