package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class HostNoticesTest {
    private fun view(unseen: String, destination: String? = null, banners: String = "") = JSONObject(
        """{"unseen":[$unseen],"destination":${destination?.let { "\"$it\"" } ?: "null"},"banners":[$banners]}""",
    )

    @Test
    fun `nothing unseen is no batch`() {
        assertNull(noticeBatch(null))
        assertNull(noticeBatch(view("")))
        assertNull(noticeBatch(view("""{"kind":"message","title":"no id"}""")))
    }

    @Test
    fun `a batch acknowledges through its latest id and carries the core's rules`() {
        val batch = noticeBatch(
            view(
                """{"id":4,"kind":"navigation","known":true},{"id":6,"kind":"somethingNew","known":false}""",
                destination = "plan",
                banners = "\"Battery · Low voltage\"",
            ),
        )!!
        assertEquals(6L, batch.through)
        assertEquals("plan", batch.destination)
        assertEquals(listOf("Battery · Low voltage"), batch.banners)
        assertEquals(listOf("somethingNew"), batch.unknownKinds)
        assertEquals("view.hostNotices(6)", hostNoticesPath(batch.through))
    }

    @Test
    fun `a banner shown in the last thirty seconds is held back and the rest are kept once`() {
        val shownAt = mapOf("Battery · Low voltage" to 1_000L)
        assertEquals(listOf("GPS lost"), quietBanners(listOf("Battery · Low voltage", "GPS lost", "GPS lost"), shownAt, 20_000L))
        assertEquals(
            listOf("Battery · Low voltage"),
            quietBanners(listOf("Battery · Low voltage"), shownAt, 1_000L + REPEAT_QUIET_MS),
        )
    }
}
