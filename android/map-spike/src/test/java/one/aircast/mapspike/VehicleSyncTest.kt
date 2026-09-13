package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
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

class UploadGateTest {
    private fun view(upload: String) = org.json.JSONObject("""{"upload":$upload}""")

    @Test
    fun `a clean plan uploads without asking`() {
        val gate = uploadGate(view("""{"canSend":true,"canProceed":false}"""))
        assertEquals(UploadStep.Send, uploadStep(gate))
    }

    @Test
    fun `a vehicle flying this mission is paused first, not silently overwritten`() {
        val gate = uploadGate(
            view(
                """{"canSend":false,"canProceed":true,"pausesFirst":true,
                    "heading":"Upload this plan?","proceedTitle":"Pause and upload",
                    "refusal":"The vehicle is flying this mission."}""",
            ),
        )
        val step = uploadStep(gate)
        assertTrue(step is UploadStep.Confirm)
        assertTrue((step as UploadStep.Confirm).gate.pausesFirst)
        assertEquals("Pause and upload", step.gate.proceedTitle)
    }

    @Test
    fun `a firmware mismatch warns and offers to go ahead`() {
        val gate = uploadGate(
            view("""{"canSend":false,"canProceed":true,"pausesFirst":false,"proceedTitle":"Upload anyway"}"""),
        )
        val step = uploadStep(gate)
        assertTrue(step is UploadStep.Confirm)
        assertFalse((step as UploadStep.Confirm).gate.pausesFirst)
    }

    @Test
    fun `no vehicle is a refusal with the core's words`() {
        val gate = uploadGate(
            view("""{"canSend":false,"canProceed":false,"refusal":"No vehicle is connected."}"""),
        )
        assertEquals(UploadStep.Refuse("No vehicle is connected."), uploadStep(gate))
    }

    @Test
    fun `a view with no upload block refuses rather than sending`() {
        assertTrue(uploadStep(null) is UploadStep.Refuse)
        assertTrue(uploadStep(uploadGate(org.json.JSONObject("{}"))) is UploadStep.Refuse)
    }
}
