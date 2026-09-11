package one.aircast.android

import one.aircast.android.ui.HostNotice
import one.aircast.android.ui.bannersToShow
import org.junit.Assert.assertEquals
import org.junit.Test

class BannersToShowTest {
    private var next = 0L

    private fun notice(kind: String, title: String, text: String) =
        HostNotice(id = next++, kind = kind, title = title, text = text)

    private fun message(text: String) = notice("message", "Aircast", text)

    @Test
    fun `a message repeated by the vehicle is shown once`() {
        val queued = listOf(message("Parameters are missing"), message("Parameters are missing"))

        assertEquals(listOf("Aircast · Parameters are missing"), bannersToShow(queued, null))
    }

    @Test
    fun `a message identical to the one already on screen is not shown again`() {
        val queued = listOf(message("Parameters are missing"))

        assertEquals(emptyList<String>(), bannersToShow(queued, "Aircast · Parameters are missing"))
    }

    @Test
    fun `a different message still gets through`() {
        val queued = listOf(message("EKF variance"))

        assertEquals(listOf("Aircast · EKF variance"), bannersToShow(queued, "Aircast · Parameters are missing"))
    }

    @Test
    fun `the same message returning after another one is shown again`() {
        val queued = listOf(message("EKF variance"), message("Low battery"), message("EKF variance"))

        assertEquals(
            listOf("Aircast · EKF variance", "Aircast · Low battery", "Aircast · EKF variance"),
            bannersToShow(queued, null),
        )
    }

    @Test
    fun `navigation notices are never banners`() {
        val queued = listOf(notice("navigation", "setup", ""), message("EKF variance"))

        assertEquals(listOf("Aircast · EKF variance"), bannersToShow(queued, null))
    }
}
