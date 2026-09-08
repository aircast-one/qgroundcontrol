package one.aircast.mapspike

enum class VehicleSync { Offline, Busy, Ready }

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
