package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class UploadReadinessTest {

    private fun view(ready: Boolean, reason: String, canSend: Boolean = true) = JSONObject(
        """{"readiness":{"ready":$ready,"reason":"$reason"},
            "upload":{"canSend":$canSend,"canProceed":false,"pausesFirst":false,
                      "heading":"","refusal":"","proceedTitle":""}}""",
    )

    @Test
    fun `a plan with an item still being drawn is not sent`() {
        val v = view(false, "An item is still being drawn, so the plan cannot be saved or sent.")

        assertEquals(
            "An item is still being drawn, so the plan cannot be saved or sent.",
            (uploadStep(uploadGate(v), notReadyToSend(v)) as UploadStep.Refuse).reason,
        )
    }

    @Test
    fun `readiness is checked before the vehicle precheck, because it is about the plan`() {
        val v = view(false, "Waiting for terrain heights before the plan can be saved or sent.", canSend = true)

        assertTrue(uploadStep(uploadGate(v), notReadyToSend(v)) is UploadStep.Refuse)
    }

    @Test
    fun `a ready plan is sent as before`() {
        val v = view(true, "")

        assertNull(notReadyToSend(v))
        assertTrue(uploadStep(uploadGate(v), notReadyToSend(v)) is UploadStep.Send)
    }

    @Test
    fun `a view with no readiness at all does not block the upload`() {
        assertNull(notReadyToSend(JSONObject("""{"upload":{"canSend":true}}""")))
        assertNull(notReadyToSend(null))
    }

    @Test
    fun `an unready plan with no reason still says something`() {
        val v = view(false, "")

        assertEquals("The plan is not ready to send.", notReadyToSend(v))
    }
}
