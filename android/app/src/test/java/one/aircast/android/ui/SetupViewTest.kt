package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class SetupViewTest {
    @Test
    fun `a core that declines to judge is not judged as not ready`() {
        assertNull(
            "readiness() answers Option<bool> and returns None when it has no basis for a verdict - " +
                "its own test says the false that means \"checked and not ready\" once drew an error " +
                "heading beside an instruction to connect a vehicle. optBoolean flattens JSON null " +
                "to false, so the head could not hold that third state at all",
            setupReadiness(JSONObject("""{"ready":null,"headline":"No vehicle connected"}"""))?.ready,
        )
    }

    @Test
    fun `a verdict that was reached is still carried`() {
        assertEquals(true, setupReadiness(JSONObject("""{"ready":true}"""))?.ready)
        assertEquals(false, setupReadiness(JSONObject("""{"ready":false}"""))?.ready)
    }

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
    fun `radio is unfinished only on px4, whose aux mappings this head has no control for`() {
        assertFalse(headFinishes(RADIO, px4 = true))
        assertTrue(headFinishes(SENSORS, px4 = true))
        assertTrue(headFinishes("Safety", px4 = true))
        assertTrue("calibration, trims and both binds are all here for apm", headFinishes(RADIO, px4 = false))
    }

    @Test
    fun `an apm radio page carries no desktop promise`() {
        assertEquals("Needs setup", setupBadge(RADIO, openable = true, px4 = false))
    }

    @Test
    fun `a page that watches but cannot finish says so before the tap`() {
        assertEquals("Finish on desktop", setupBadge(RADIO, openable = true, px4 = true))
    }

    @Test
    fun `a page the head can finish keeps the plain badge`() {
        assertEquals("Needs setup", setupBadge(SENSORS, openable = true, px4 = true))
    }

    @Test
    fun `a page this head cannot open is not promised a desktop finish`() {
        assertEquals("Needs setup", setupBadge(RADIO, openable = false, px4 = true))
    }

    @Test
    fun `a page opens on what this head has, not on a field the core may stop serving`() {
        val served = JSONObject(
            """{"groups":[{"title":"S","pages":[{"name":"Radio","parameterSections":false}]}]}""",
        )
        val page = setupPage(served, RADIO)

        assertTrue("Radio has a screen here, so no flag from the core decides it", headCanOpen(page, RADIO))
        assertEquals("Finish on desktop", setupBadge(RADIO, headCanOpen(page, RADIO), px4 = true))
    }

    @Test
    fun `a page the head has no screen for does not open`() {
        val summary = SetupPage("Summary", parameterSections = false)
        val frame = SetupPage("Frame", parameterSections = false)

        assertFalse(
            "an unnamed page once opened the Remote Support screen, which forwards live position to a third party",
            headCanOpen(summary, "Summary"),
        )
        assertFalse(headCanOpen(frame, "Frame"))
    }

    @Test
    fun `Motors opens here now, because the operator asked for it rather than the desktop`() {
        val motors = SetupPage("Motors", parameterSections = false)

        assertTrue(headCanOpen(motors, MOTORS))
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

class SetupFirmwareTest {
    @Test
    fun `the served firmware token answers px4, apm and no vehicle apart`() {
        val px4 = setupReadiness(JSONObject("""{"connected":true,"firmware":"px4"}"""))
        val apm = setupReadiness(JSONObject("""{"connected":true,"firmware":"apm"}"""))
        val none = setupReadiness(JSONObject("""{"connected":false,"firmware":"none"}"""))

        assertTrue(isPx4(px4))
        assertFalse("apm is not px4", isPx4(apm))
        assertFalse("and no vehicle is not px4 either, which a boolean could not distinguish", isPx4(none))
        assertFalse(isPx4(null))
    }

    @Test
    fun `connected comes from the same view rather than a second read`() {
        assertTrue(setupReadiness(JSONObject("""{"connected":true}"""))!!.connected)
        assertFalse(setupReadiness(JSONObject("""{"connected":false}"""))!!.connected)
    }
}
