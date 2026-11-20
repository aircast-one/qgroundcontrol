package one.aircast.android.ui

import one.aircast.android.bridge.VehicleCommands
import one.aircast.android.bridge.offMainDetached
import java.util.concurrent.atomic.AtomicReference

internal const val EMERGENCY_STOP = "emergencyStop"

internal const val KILL_SETTLE_MS = 5000L

internal val motorsKilledAt = AtomicReference<Long?>(null)

internal fun monotonicMillis(): Long = System.nanoTime() / 1_000_000

internal fun killedRecently(killedAt: Long?, now: Long): Boolean = killedAt != null && now >= killedAt && now - killedAt <= KILL_SETTLE_MS

internal fun emergencyStopOffer(offers: Map<String, GuidedOffer>): GuidedOffer? =
    offers[EMERGENCY_STOP]?.takeIf { it.shown }

internal fun armedStopOffer(offers: Map<String, GuidedOffer>, armed: Boolean): GuidedOffer? =
    emergencyStopOffer(offers)?.takeIf { armed && it.ready }

internal fun emergencyStopAction(offer: GuidedOffer): GuidedAction = GuidedAction(
    name = offer.title,
    confirm = offer.prompt,
    destructive = true,
    run = {
        motorsKilledAt.set(monotonicMillis())
        offMainDetached { VehicleCommands.emergencyStop() }
    },
)
