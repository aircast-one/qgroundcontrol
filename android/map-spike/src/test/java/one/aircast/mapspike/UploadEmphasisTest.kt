package one.aircast.mapspike

import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class UploadEmphasisTest {

    private fun emphasised(offline: Boolean, syncing: Boolean) =
        syncRefusal(vehicleSyncState(offline, syncing), "upload to") == null

    @Test
    fun `with a vehicle ready to take it, Upload is the primary action`() {
        assertTrue(
            "this is the direction that matters - the Plan tab exists to get a plan onto an " +
                "aircraft, and de-emphasising the button that does it would be worse than the " +
                "problem being fixed",
            emphasised(offline = false, syncing = false),
        )
        assertNull(syncRefusal(VehicleSync.Ready, "upload to"))
    }

    @Test
    fun `with no vehicle it is not advertised as the thing to do`() {
        assertFalse(
            "the core says canSend false with 'No vehicle is connected, so there is nowhere to " +
                "send this plan' - a filled primary button is the head telling the operator to " +
                "do the one thing that cannot work",
            emphasised(offline = true, syncing = false),
        )
    }

    @Test
    fun `mid-sync it is not advertised either`() {
        assertFalse(emphasised(offline = false, syncing = true))
    }

    @Test
    fun `it stays tappable in every state, because the refusal is the explanation`() {
        listOf(VehicleSync.Offline, VehicleSync.Busy, VehicleSync.Ready).forEach { state ->
            val refusal = syncRefusal(state, "upload to")
            assertTrue(
                "$state: a greyed button on a phone cannot carry the sentence that explains it, " +
                    "so the button is always pressable and answers when pressed",
                refusal == null || refusal.isNotBlank(),
            )
        }
    }
}
