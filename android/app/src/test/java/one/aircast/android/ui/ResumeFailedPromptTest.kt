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

    @Test
    fun `a dismissal is forgotten only once the core says nothing failed, not while the view is still loading`() {
        org.junit.Assert.assertFalse(resumeCleared(null))
        org.junit.Assert.assertFalse(resumeCleared(JSONObject("""{"resumeFailedIndex":4}""")))
        org.junit.Assert.assertTrue(resumeCleared(JSONObject("""{"resumeFailedIndex":null}""")))
    }
}
