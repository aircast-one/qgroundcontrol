package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class HostNoticesTest {
    private fun view(vararg rows: String) =
        JSONObject("""{"notices":[${rows.joinToString(",")}]}""")

    private fun row(id: Long, kind: Int, title: String, text: String = "") =
        """{"id":$id,"kind":$kind,"title":"$title","text":"$text"}"""

    @Test
    fun `no view means nothing queued`() {
        assertTrue(hostNotices(null).isEmpty())
        assertTrue(hostNotices(JSONObject("{}")).isEmpty())
    }

    @Test
    fun `a notice without an id is not drawn rather than acknowledged as minus one`() {
        val view = JSONObject("""{"notices":[{"kind":0,"title":"t"}]}""")
        assertTrue(hostNotices(view).isEmpty())
    }

    @Test
    fun `the queue keeps every notice, not the latest`() {
        val notices = hostNotices(view(row(1, 0, "a"), row(2, 0, "b")))
        assertEquals(listOf(1L, 2L), notices.map { it.id })
    }

    @Test
    fun `every message is shown, not just the first`() {
        val notices = hostNotices(
            view(row(1, NOTICE_MESSAGE, "one"), row(2, NOTICE_VEHICLE_ERROR, "two"), row(3, NOTICE_MESSAGE, "three")),
        )
        assertEquals(listOf("one", "two", "three"), noticesToShow(notices).map { it.title })
    }

    @Test
    fun `navigation carries its target in the title`() {
        val notices = hostNotices(view(row(1, NOTICE_NAVIGATION, "setup")))
        assertEquals("setup", noticeDestination(notices))
    }

    @Test
    fun `the last navigation wins when two are queued`() {
        val notices = hostNotices(view(row(1, NOTICE_NAVIGATION, "setup"), row(2, NOTICE_NAVIGATION, "plan")))
        assertEquals("plan", noticeDestination(notices))
    }

    @Test
    fun `a navigation notice is never shown as a message`() {
        val notices = hostNotices(view(row(1, NOTICE_NAVIGATION, "setup")))
        assertTrue(noticesToShow(notices).isEmpty())
    }

    @Test
    fun `a banner joins the title and text it actually has`() {
        assertEquals("Aircast · went wrong", noticeBanner(HostNotice(1, 1, "Aircast", "went wrong")))
        assertEquals("Aircast", noticeBanner(HostNotice(1, 1, "Aircast", "")))
        assertEquals("went wrong", noticeBanner(HostNotice(1, 1, "", "went wrong")))
    }
}
