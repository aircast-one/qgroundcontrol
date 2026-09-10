package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SetupViewTest {
    private val corePages = JSONObject(
        """{"groups":[
            {"title":"Vehicle","pages":[{"name":"Summary","native":true}]},
            {"title":"Setup","pages":[
                {"name":"Sensors","native":true},{"name":"Radio","native":true},
                {"name":"Frame","native":true},{"name":"Flight Modes","native":true},
                {"name":"Safety","native":true},{"name":"Power","native":true},
                {"name":"Motors","native":true},{"name":"Tuning","native":true},
                {"name":"Camera","native":true},{"name":"Lights","native":true},
                {"name":"Flight Behavior","native":false}]},
            {"title":"Advanced","pages":[
                {"name":"Remote Support","native":true},{"name":"Parameters","native":true}]}]}""",
    )

    @Test
    fun `every page the head names is a page the core offers`() {
        val offered = setupGroups(corePages).flatMap { it.pages }.map { it.name }.toSet()
        val claimed = setOf(RADIO, SENSORS, "Remote Support")

        assertEquals(emptySet<String>(), claimed - offered)
    }

    @Test
    fun `the head admits the one page it can open but not finish`() {
        assertFalse(headFinishes(RADIO))
        assertTrue(headFinishes(SENSORS))
        assertTrue(headFinishes("Safety"))
    }

    @Test
    fun `a page that watches but cannot finish says so before the tap`() {
        assertEquals("Finish on desktop", setupBadge(RADIO, openable = true))
    }

    @Test
    fun `a page the head can finish keeps the plain badge`() {
        assertEquals("Needs setup", setupBadge(SENSORS, openable = true))
    }

    @Test
    fun `a page this head cannot open is not promised a desktop finish`() {
        assertEquals("Needs setup", setupBadge(RADIO, openable = false))
    }

    @Test
    fun `a page opens on what this head has, not on a field the core may stop serving`() {
        val served = JSONObject(
            """{"groups":[{"title":"S","pages":[{"name":"Radio","parameterSections":false}]}]}""",
        )
        val page = setupPage(served, RADIO)

        assertTrue("Radio has a screen here, so no flag from the core decides it", headCanOpen(page, RADIO))
        assertEquals("Finish on desktop", setupBadge(RADIO, headCanOpen(page, RADIO)))
    }

    @Test
    fun `a page the head has no screen for does not open`() {
        val motors = SetupPage("Motors", parameterSections = false)
        val summary = SetupPage("Summary", parameterSections = false)

        assertFalse(
            "Motors opened the Remote Support screen, which forwards live position to a third party",
            headCanOpen(motors, "Motors"),
        )
        assertFalse(headCanOpen(summary, "Summary"))
    }

    @Test
    fun `the screens the head does have still open`() {
        assertTrue(headCanOpen(SetupPage(SENSORS, parameterSections = false), SENSORS))
        assertTrue(headCanOpen(SetupPage(RADIO, parameterSections = false), RADIO))
        assertTrue(
            headCanOpen(SetupPage(REMOTE_SUPPORT, parameterSections = false), REMOTE_SUPPORT),
        )
        assertTrue(headCanOpen(SetupPage("Safety", parameterSections = true), "Safety"))
    }

    @Test
    fun `a page the core describes as parameters opens, and no page opens without one`() {
        assertTrue(headCanOpen(SetupPage("Tuning", parameterSections = true), "Tuning"))
        assertFalse(headCanOpen(null, SENSORS))
    }
}

class SetupReadinessTest {
    @Test
    fun `no view means no readiness`() {
        assertNull(setupReadiness(null))
    }

    @Test
    fun `readiness comes from the core verbatim`() {
        val view = JSONObject(
            """{"ready":false,"headline":"2 components need setup","detail":"A sensor is unhealthy."}""",
        )
        val readiness = setupReadiness(view)
        assertEquals(false, readiness?.ready)
        assertEquals("2 components need setup", readiness?.headline)
        assertEquals("A sensor is unhealthy.", readiness?.detail)
    }

    @Test
    fun `a ready vehicle carries no headline`() {
        val readiness = setupReadiness(JSONObject("""{"ready":true,"headline":"","detail":""}"""))
        assertEquals(true, readiness?.ready)
        assertEquals("", readiness?.headline)
    }
}
