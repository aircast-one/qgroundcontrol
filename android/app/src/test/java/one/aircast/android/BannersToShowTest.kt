package one.aircast.android

import one.aircast.android.ui.REPEAT_QUIET_MS
import one.aircast.android.ui.quietBanners
import org.junit.Assert.assertEquals
import org.junit.Test

// Which notices become banner lines is view.hostNotices' rule now (hostnoticeview.rs covers
// navigation never being one); what this head still owns is the repeat window.
class BannersToShowTest {
    private val now = 1_000_000L
    private val missing = "Aircast · Parameters are missing"
    private val ekf = "Aircast · EKF variance"

    @Test
    fun `a message arriving many times in one batch is shown once`() {
        assertEquals(listOf(missing), quietBanners(listOf(missing, missing), emptyMap(), now))
    }

    @Test
    fun `a message that repeats faster than the quiet window is held back`() {
        assertEquals(emptyList<String>(), quietBanners(listOf(missing), mapOf(missing to now - 1_000L), now))
    }

    @Test
    fun `a condition still recurring after the quiet window is announced again`() {
        assertEquals(listOf(ekf), quietBanners(listOf(ekf), mapOf(ekf to now - REPEAT_QUIET_MS - 1L), now))
    }

    @Test
    fun `one message being held back does not hold back a different one`() {
        assertEquals(listOf(ekf), quietBanners(listOf(missing, ekf), mapOf(missing to now - 1_000L), now))
    }
}
