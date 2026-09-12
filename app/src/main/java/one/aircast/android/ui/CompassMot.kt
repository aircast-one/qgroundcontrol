package one.aircast.android.ui

internal const val COMPASS_MOT_INVOCATION = "sensorsCal.calibrateMotorInterference"

internal val COMPASS_MOT_STEPS = listOf(
    "Take the propellers off, turn them over and move each one round the frame by " +
        "one position. Held that way they push the aircraft into the ground when the " +
        "throttle comes up.",
    "Tape or strap the aircraft down so it cannot move.",
    "Turn the transmitter on and leave the throttle at zero.",
)

internal const val COMPASS_MOT_PURPOSE =
    "Measures how much the motors disturb the compass. Worth doing on an aircraft with " +
        "only an internal compass, or where the motors and power wiring sit close to it."

internal const val COMPASS_MOT_CURRENT_WARNING =
    "This needs a battery current monitor to work well — the interference grows with " +
        "current drawn. Without one it can be run off throttle instead, and the result " +
        "is worse."

internal fun compassMotBlocked(connected: Boolean, busy: Boolean, flying: Boolean): String? = when {
    !connected -> "Connect a vehicle first."
    flying -> "Not while the aircraft is flying."
    busy -> "Another calibration is running."
    else -> null
}
