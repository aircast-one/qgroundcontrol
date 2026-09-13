package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class HostNoticesTest {
    private fun view(vararg rows: String) =
        JSONObject("""{"notices":[${rows.joinToString(",")}]}""")

    private fun row(id: Long, kind: String, title: String, text: String = "") =
        """{"id":$id,"kind":"$kind","title":"$title","text":"$text"}"""

    @Test
    fun `no view means nothing queued`() {
        assertTrue(hostNotices(null).isEmpty())
        assertTrue(hostNotices(JSONObject("{}")).isEmpty())
    }

    @Test
    fun `a notice without an id is not drawn rather than acknowledged as minus one`() {
        val view = JSONObject("""{"notices":[{"kind":"message","title":"t"}]}""")
        assertTrue(hostNotices(view).isEmpty())
    }

    @Test
    fun `the queue keeps every notice, not the latest`() {
        val notices = hostNotices(view(row(1, NOTICE_MESSAGE, "a"), row(2, NOTICE_MESSAGE, "b")))
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
        assertEquals("Aircast · went wrong", noticeBanner(HostNotice(1, NOTICE_VEHICLE_ERROR, "Aircast", "went wrong")))
        assertEquals("Aircast", noticeBanner(HostNotice(1, NOTICE_VEHICLE_ERROR, "Aircast", "")))
        assertEquals("went wrong", noticeBanner(HostNotice(1, NOTICE_VEHICLE_ERROR, "", "went wrong")))
    }

    @Test
    fun `an unrecognised kind is shown rather than silently treated as a message`() {
        val notices = hostNotices(view(row(1, "somethingNew", "surprise")))
        assertEquals(listOf("surprise"), noticesToShow(notices).map { it.title })
    }

    @Test
    fun `an ordinal kind no longer decodes as a message`() {
        val notices = hostNotices(JSONObject("""{"notices":[{"id":1,"kind":2,"title":"setup"}]}"""))
        assertNull(noticeDestination(notices))
    }

    @Test
    fun `a notice already acknowledged is not delivered again`() {
        val notices = listOf(
            HostNotice(1L, NOTICE_NAVIGATION, "setup", ""),
            HostNotice(2L, NOTICE_MESSAGE, "Aircast", "Components need setup."),
        )

        assertEquals(listOf(1L, 2L), noticesAfter(notices, -1L).map { it.id })
        assertEquals(listOf(2L), noticesAfter(notices, 1L).map { it.id })
        assertEquals(emptyList<Long>(), noticesAfter(notices, 2L).map { it.id })
    }

    @Test
    fun `a navigation that keeps arriving does not keep moving the operator`() {
        val first = listOf(HostNotice(7L, NOTICE_NAVIGATION, "setup", ""))
        val redelivered = first + HostNotice(8L, NOTICE_MESSAGE, "Aircast", "Still not set up.")

        val through = noticesAfter(first, -1L).last().id
        assertEquals("setup", noticeDestination(noticesAfter(first, -1L)))
        assertNull(noticeDestination(noticesAfter(redelivered, through)))
    }
}
