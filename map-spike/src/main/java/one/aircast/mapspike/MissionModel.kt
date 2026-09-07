package one.aircast.mapspike

data class Waypoint(val id: Int, val latitude: Double, val longitude: Double)

class MissionModel(initial: List<Waypoint> = emptyList()) {
    private val waypoints = initial.toMutableList()
    private var nextId = (initial.maxOfOrNull { it.id } ?: 0) + 1

    fun waypoints(): List<Waypoint> = waypoints.toList()

    val size: Int get() = waypoints.size

    fun add(latitude: Double, longitude: Double): Waypoint? {
        if (!isPlottable(latitude, longitude)) {
            return null
        }
        val waypoint = Waypoint(nextId++, latitude, longitude)
        waypoints.add(waypoint)
        return waypoint
    }

    fun move(id: Int, latitude: Double, longitude: Double): Boolean {
        if (!isPlottable(latitude, longitude)) {
            return false
        }
        val index = waypoints.indexOfFirst { it.id == id }
        if (index < 0) {
            return false
        }
        waypoints[index] = waypoints[index].copy(latitude = latitude, longitude = longitude)
        return true
    }

    fun remove(id: Int): Boolean = waypoints.removeAll { it.id == id }

    fun clear() {
        waypoints.clear()
    }

    fun find(id: Int): Waypoint? = waypoints.firstOrNull { it.id == id }

    fun sequence(): List<Pair<Int, Waypoint>> = waypoints.mapIndexed { index, wp -> (index + 1) to wp }
}
