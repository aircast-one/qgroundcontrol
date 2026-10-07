package one.aircast.android.ui

import one.aircast.android.bridge.VehicleCommands
import one.aircast.android.bridge.offMainDetached

internal const val EMERGENCY_STOP = "emergencyStop"

internal fun emergencyStopOffer(offers: Map<String, GuidedOffer>): GuidedOffer? =
    offers[EMERGENCY_STOP]?.takeIf { it.shown }

internal fun armedStopOffer(offers: Map<String, GuidedOffer>, armed: Boolean): GuidedOffer? =
    emergencyStopOffer(offers)?.takeIf { armed && it.ready }

internal fun emergencyStopAction(offer: GuidedOffer): GuidedAction = GuidedAction(
    name = offer.title,
    confirm = offer.prompt,
    destructive = true,
    run = { offMainDetached { VehicleCommands.emergencyStop() } },
)
