import SwiftUI

private let GIMBAL_CENTER = "gimbal.center"
private let OFF_CENTRE_DEG = 2.0
private let RECENTER_SCRIM_ALPHA = 0.55
private let TILT_READOUT_WIDTH: CGFloat = 64
private let TILT_READOUT_HEIGHT: CGFloat = 32
private let RECENTER_GLYPH: CGFloat = 12

enum RecenterTarget { case Synthetic, Gimbal }

private func offCentre(_ degrees: Double?...) -> Bool { degrees.contains { $0.map { abs($0) > OFF_CENTRE_DEG } ?? false } }

func recenterTarget(syntheticAims: Bool, syntheticTilt: Double?, syntheticPan: Double?, gimbal: GimbalIndicatorState?) -> RecenterTarget? {
    switch (syntheticAims, gimbal) {
    case (true, _): offCentre(syntheticTilt.map { $0 - DEFAULT_TILT_DEG }, syntheticPan) ? .Synthetic : nil
    case (false, let gimbal?): offCentre(gimbal.pitchDegrees, gimbal.yawDegrees) ? .Gimbal : nil
    default: nil
    }
}

@MainActor
func recenter(_ target: RecenterTarget, onRefused: @escaping @MainActor (String?) -> Void) {
    switch target {
    case .Synthetic: offMainInOrder { _ = Qgc.invoke(SYNTHETIC_AIM, DEFAULT_TILT_DEG, 0.0) }
    case .Gimbal: Task { onRefused(await offMain { gimbalRefusal(Qgc.call(GIMBAL_CENTER)) }) }
    }
}

func tiltReadoutText(_ degrees: Double) -> String { "\(Int(degrees.rounded()))\u{00B0}" }

struct TiltReadout: View {
    let degrees: Double
    let onRecenter: (() -> Void)?
    var onTap: (() -> Void)? = nil
    var tapHint: String? = nil
    @Environment(\.theme) private var theme

    var body: some View {
        let text = tiltReadoutText(degrees)
        let action = onRecenter ?? onTap
        let label = HStack(spacing: 4) {
            if onRecenter != nil { Image(.planMyLocation).font(.system(size: RECENTER_GLYPH, weight: .semibold)) }
            Text(text).font(.labelMedium).osdShadow(onRecenter == nil)
        }
        .foregroundStyle(theme.aircast.outdoorForeground)
        .frame(width: TILT_READOUT_WIDTH, height: TILT_READOUT_HEIGHT)
        .background(onRecenter != nil ? Color.black.opacity(RECENTER_SCRIM_ALPHA) : Color.clear, in: Capsule())
        .minimumTouchTarget()
        Group {
            if let action {
                Button(action: action) { label }
                    .buttonStyle(.plain)
                    .accessibilityHint(onRecenter != nil ? "Recenter" : tapHint ?? "")
            } else {
                label.accessibilityElement(children: .ignore)
            }
        }
        .accessibilityLabel(onRecenter != nil ? "Tilt \(text), off centre" : "Tilt \(text)")
    }
}
