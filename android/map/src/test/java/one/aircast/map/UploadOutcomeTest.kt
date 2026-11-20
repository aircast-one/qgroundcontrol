package one.aircast.map

import org.junit.Assert.assertEquals
import org.junit.Test

class UploadOutcomeTest {
    @Test
    fun `an invoked upload is reported as sent, never as accepted`() {
        assertEquals(UploadOutcome.Sent, uploadOutcome(invoked = true))
        assertEquals("Upload sent to vehicle", uploadMessage(UploadOutcome.Sent))
    }

    @Test
    fun `a download is reported as requested, never as received`() {
        assertEquals("Download requested from vehicle", downloadMessage(UploadOutcome.Sent))
        assertEquals("Download did not start", downloadMessage(UploadOutcome.NotStarted))
    }

    @Test
    fun `a call that never ran says so`() {
        assertEquals(UploadOutcome.NotStarted, uploadOutcome(invoked = false))
        assertEquals("Upload did not start", uploadMessage(UploadOutcome.NotStarted))
    }

    @Test
    fun `a pause the aircraft refuses stops the upload and says why, so a mission it is still flying is not replaced`() {
        assertEquals("Pause is not available right now.", refusalIn(org.json.JSONObject("""{"ok":false,"reason":"Pause is not available right now."}"""), "The aircraft did not pause."))
        assertEquals("The aircraft did not pause.", refusalIn(null, "The aircraft did not pause."))
        assertEquals(null, refusalIn(org.json.JSONObject("""{"ok":true}"""), "The aircraft did not pause."))
        assertEquals("The aircraft did not pause, so the plan was not uploaded. Pause is not available right now.", pauseRefusedText("Pause is not available right now."))
    }
}
