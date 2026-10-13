package one.aircast.map

private const val SHORTEST_GLIDE_NS = 100_000_000L
private const val LONGEST_GLIDE_NS = 1_000_000_000L

data class VehiclePose(val latitude: Double, val longitude: Double, val heading: Double)

val UNPLACED = VehiclePose(Double.NaN, Double.NaN, Double.NaN)

data class Glide(val from: VehiclePose, val to: VehiclePose, val startNs: Long, val endNs: Long, val fixedAtNs: Long) {
    fun at(nowNs: Long): VehiclePose {
        val fraction = if (endNs <= startNs) 1.0 else ((nowNs - startNs).toDouble() / (endNs - startNs)).coerceIn(0.0, 1.0)
        val turn = ((to.heading - from.heading) % 360 + 540) % 360 - 180
        return VehiclePose(
            latitude = from.latitude + (to.latitude - from.latitude) * fraction,
            longitude = from.longitude + (to.longitude - from.longitude) * fraction,
            heading = if (from.heading.isNaN()) to.heading else from.heading + turn * fraction,
        )
    }

    fun toward(target: VehiclePose, nowNs: Long): Glide {
        val shown = at(nowNs)
        val moved = target.latitude != to.latitude || target.longitude != to.longitude
        return when {
            target == to -> this
            !isPlottable(shown.latitude, shown.longitude) || !isPlottable(target.latitude, target.longitude) -> settled(target, nowNs)
            moved -> Glide(shown, target, nowNs, nowNs + (nowNs - fixedAtNs).coerceIn(SHORTEST_GLIDE_NS, LONGEST_GLIDE_NS), nowNs)
            else -> Glide(shown, target, nowNs, maxOf(endNs, nowNs + SHORTEST_GLIDE_NS), fixedAtNs)
        }
    }

    companion object {
        fun settled(pose: VehiclePose, nowNs: Long): Glide = Glide(pose, pose, nowNs, nowNs, nowNs)
    }
}
