package one.aircast.mapspike

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
}
