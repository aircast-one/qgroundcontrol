package one.aircast.mapspike

enum class VehicleSync { Offline, Busy, Ready }

// The bridge reports ok when a method was found and invoked, not when it did
// anything. sendToVehicle with no vehicle, or while a sync is already running,
// logs a warning and returns, so the map would say it sent and be believed.
fun vehicleSyncState(offline: Boolean, syncing: Boolean): VehicleSync = when {
    offline -> VehicleSync.Offline
    syncing -> VehicleSync.Busy
    else -> VehicleSync.Ready
}

fun syncRefusal(state: VehicleSync, action: String): String? = when (state) {
    VehicleSync.Offline -> "No vehicle to $action"
    VehicleSync.Busy -> "Already syncing, wait for it to finish"
    VehicleSync.Ready -> null
}
