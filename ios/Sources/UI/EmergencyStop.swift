import Foundation

let EMERGENCY_STOP = "emergencyStop"

let KILL_SETTLE_SECONDS = 5.0

nonisolated(unsafe) var motorsKilledAt: TimeInterval?

func killedRecently(_ killedAt: TimeInterval?, _ now: TimeInterval) -> Bool {
    killedAt.map { now >= $0 && now - $0 <= KILL_SETTLE_SECONDS } ?? false
}

func emergencyStopOffer(_ offers: [String: GuidedOffer]) -> GuidedOffer? {
    offers[EMERGENCY_STOP].flatMap { $0.shown ? $0 : nil }
}

func armedStopOffer(_ offers: [String: GuidedOffer], _ armed: Bool) -> GuidedOffer? {
    emergencyStopOffer(offers).flatMap { armed && $0.ready ? $0 : nil }
}

func emergencyStopAction(_ offer: GuidedOffer) -> GuidedAction {
    GuidedAction(
        name: offer.title,
        confirm: offer.prompt,
        destructive: true,
        run: {
            motorsKilledAt = ProcessInfo.processInfo.systemUptime
            offMain { VehicleCommands.emergencyStop() }
        }
    )
}
