package one.aircast.android.ui

internal const val ALTITUDE_DEADBAND_METERS = 0.01

internal data class AltitudeRange(val min: Double, val max: Double)

internal fun altitudeRange(settingMin: Double, settingMax: Double, current: Double): AltitudeRange {
    val low = minOf(settingMin, settingMax)
    val high = maxOf(settingMin, settingMax)
    return AltitudeRange(
        min = minOf(low, current),
        max = maxOf(high, current),
    )
}

internal fun altitudeDelta(target: Double, current: Double): Double = target - current

internal fun altitudeChangeIsUseful(target: Double, current: Double): Boolean =
    kotlin.math.abs(altitudeDelta(target, current)) >= ALTITUDE_DEADBAND_METERS

internal fun altitudeChangeSummary(target: Double, current: Double): String {
    val delta = altitudeDelta(target, current)
    val target1 = String.format("%.1f", target)
    val move = String.format("%.1f", kotlin.math.abs(delta))
    return when {
        !altitudeChangeIsUseful(target, current) ->
            "The aircraft is already at $target1 m and will not move."
        delta > 0 -> "The aircraft will climb $move m to $target1 m."
        else -> "The aircraft will descend $move m to $target1 m."
    }
}
