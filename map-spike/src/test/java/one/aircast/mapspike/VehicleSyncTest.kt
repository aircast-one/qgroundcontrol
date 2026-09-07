package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class VehicleSyncTest {
    @Test
    fun `a connected idle vehicle is ready`() {
        assertEquals(VehicleSync.Ready, vehicleSyncState(offline = false, syncing = false))
        assertNull(syncRefusal(VehicleSync.Ready, "send to"))
    }

    @Test
    fun `no vehicle is refused rather than reported as sent`() {
        assertEquals(VehicleSync.Offline, vehicleSyncState(offline = true, syncing = false))
        assertEquals("No vehicle to send to", syncRefusal(VehicleSync.Offline, "send to"))
    }

    @Test
    fun `a sync already running is refused`() {
        assertEquals(VehicleSync.Busy, vehicleSyncState(offline = false, syncing = true))
        assertEquals(
            "Already syncing, wait for it to finish",
            syncRefusal(VehicleSync.Busy, "send to"),
        )
    }

    @Test
    fun `being offline outranks being busy`() {
        assertEquals(VehicleSync.Offline, vehicleSyncState(offline = true, syncing = true))
    }

    @Test
    fun `the refusal names the action it refused`() {
        assertEquals("No vehicle to load from", syncRefusal(VehicleSync.Offline, "load from"))
    }
}
