package one.aircast.android.ui

import androidx.compose.material3.SnackbarHostState
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.yield
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

private suspend fun settle(left: Int = 100) {
    if (left == 0) return
    yield()
    settle(left - 1)
}

private val SnackbarHostState.message: String? get() = currentSnackbarData?.visuals?.message

class HostNoticesTest {
    @Test
    fun `the newest payload result comes apart from the queued banners, and a blank one only clears`() {
        val batch = { payload: String -> noticeBatch(JSONObject("""{"unseen":[{"id":3,"kind":"message"}],"banners":[],"payload":$payload}""")) }
        assertEquals("Bay 2 dropped", batch("\"Bay 2 dropped\"")?.payload)
        assertEquals("", batch("\"\"")?.payload)
        assertNull(batch("null")?.payload)
    }

    @Test
    fun `a newer payload result takes the snackbar at once, and a blank one only clears it`() = runBlocking<Unit> {
        val host = SnackbarHostState()
        val show: suspend (String) -> Unit = { host.showSnackbar(it) }
        val first = showLatest(this, null, "Bay 1 dropped", show)
        settle()
        assertEquals("Bay 1 dropped", host.message)
        val second = showLatest(this, first, "Bay 2 dropped", show)
        settle()
        assertEquals("Bay 2 dropped", host.message)
        assertNull(showLatest(this, second, "", show))
        settle()
        assertNull(host.message)
    }

    @Test
    fun `a payload result still waiting behind another notice is dropped for a newer one, and the other keeps its turn`() = runBlocking<Unit> {
        val host = SnackbarHostState()
        val show: suspend (String) -> Unit = { host.showSnackbar(it) }
        launch { host.showSnackbar("Sticks took over: Loiter") }
        settle()
        val second = showLatest(this, showLatest(this, null, "Bay 1 dropped", show), "Bay 2 dropped", show)
        settle()
        assertEquals("Sticks took over: Loiter", host.message)
        host.currentSnackbarData?.dismiss()
        settle()
        assertEquals("Bay 2 dropped", host.message)
        second?.cancel()
    }

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

    @Test
    fun `app messages arrive as OK dialogs, as showAppMessage opens one`() {
        val batch = noticeBatch(JSONObject("""{"unseen":[{"id":4,"kind":"message"}],"banners":[],"dialogs":[{"title":"Aircast QGC","text":"Parameters missing","action":null},{"title":"Aircast QGC","text":"Reboot","action":"rebootVehicle"}]}"""))
        assertEquals(listOf(AppMessage("Aircast QGC", "Parameters missing"), AppMessage("Aircast QGC", "Reboot", REBOOT_VEHICLE_ACTION)), batch?.dialogs)
    }

    @Test
    fun `a radio found on USB is offered with Use and Not now`() {
        val batch = noticeBatch(JSONObject("""{"unseen":[{"id":5,"kind":"message"}],"banners":[],"dialogs":[{"title":"Aircast QGC","text":"Fly with it?","action":"useRadio"}]}"""))
        assertEquals(USE_RADIO_ACTION, batch?.dialogs?.single()?.action)
        assertEquals("Use", confirmLabel(USE_RADIO_ACTION))
        assertEquals("Open Setup", confirmLabel(OPEN_SETUP_ACTION))
        assertEquals("OK", confirmLabel(null))
    }

    @Test
    fun `a second error in one batch is still critical and says more arrived`() {
        assertEquals(null, criticalBanner(emptyList()))
        assertEquals("EKF failure", criticalBanner(listOf("EKF failure")))
        assertEquals("EKF failure \u00b7 $ADDITIONAL_ERRORS", criticalBanner(listOf("EKF failure", "GPS glitch")))
    }
}
