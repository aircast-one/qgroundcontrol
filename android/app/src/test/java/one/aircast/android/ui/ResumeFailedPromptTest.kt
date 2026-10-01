package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ResumeFailedPromptTest {
    @Test
    fun `a failed resume upload names the index to retry`() {
        assertEquals(4, resumeFailedIndex(JSONObject("""{"resumeFailedIndex":4}""")))
        assertNull(resumeFailedIndex(JSONObject("""{"resumeFailedIndex":null}""")))
        assertNull(resumeFailedIndex(null))
    }
}
