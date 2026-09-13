package one.aircast.android

import one.aircast.android.ui.HostNotice
import one.aircast.android.ui.REPEAT_QUIET_MS
import one.aircast.android.ui.bannersToShow
import org.junit.Assert.assertEquals
import org.junit.Test

class BannersToShowTest {
    private var next = 0L

    private fun notice(kind: String, title: String, text: String) =
        HostNotice(id = next++, kind = kind, title = title, text = text)

    private fun message(text: String) = notice("message", "Aircast", text)

    private val now = 1_000_000L

    @Test
    fun `a message arriving many times in one batch is shown once`() {
        val queued = listOf(message("Parameters are missing"), message("Parameters are missing"))

        assertEquals(
            listOf("Aircast · Parameters are missing"),
            bannersToShow(queued, emptyMap(), now),
        )
    }

    @Test
    fun `a message that repeats faster than the quiet window is held back`() {
        val shown = mapOf("Aircast · Parameters are missing" to now - 1_000L)

        assertEquals(emptyList<String>(), bannersToShow(listOf(message("Parameters are missing")), shown, now))
    }

    @Test
    fun `a condition still recurring after the quiet window is announced again`() {
        val shown = mapOf("Aircast · EKF variance" to now - REPEAT_QUIET_MS - 1L)

        assertEquals(
            listOf("Aircast · EKF variance"),
            bannersToShow(listOf(message("EKF variance")), shown, now),
        )
    }

    @Test
    fun `one message being held back does not hold back a different one`() {
        val shown = mapOf("Aircast · Parameters are missing" to now - 1_000L)
        val queued = listOf(message("Parameters are missing"), message("EKF variance"))

        assertEquals(listOf("Aircast · EKF variance"), bannersToShow(queued, shown, now))
    }

    @Test
    fun `navigation notices are never banners`() {
        val queued = listOf(notice("navigation", "setup", ""), message("EKF variance"))

        assertEquals(listOf("Aircast · EKF variance"), bannersToShow(queued, emptyMap(), now))
    }
}
