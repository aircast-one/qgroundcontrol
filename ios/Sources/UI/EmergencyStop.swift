import Foundation

let EMERGENCY_STOP = "emergencyStop"

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
        run: { offMain { VehicleCommands.emergencyStop() } }
    )
}
