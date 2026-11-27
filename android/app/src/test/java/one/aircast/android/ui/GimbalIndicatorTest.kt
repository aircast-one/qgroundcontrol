package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertTrue
import org.junit.Test

class GimbalIndicatorTest {
    @Test
    fun `the cell reads status, pitch and yaw as the toolbar does`() {
        val state = gimbalIndicator(
            JSONObject(
                """{"shown":true,"statusText":"Yaw follow","pitchText":"P: -12.3","yawText":"Y: 5.0","yawLockLabel":"Yaw Lock",""" +
                    """"retractOffered":true,"controlOffered":false,"controlLabel":"Acquire Control","gimbals":[{"name":"Gimbal 1","managerCompid":1,"deviceId":154,"active":true}]}""",
            ),
        )!!
        assertEquals("Yaw follow · P: -12.3 · Y: 5.0", gimbalCellText(state))
        val two = state.copy(gimbals = state.gimbals + GimbalChoice("1-155", 1, 155, false))
        assertEquals("the slot a reading sits in is as wide as the widest value it can show", "P: -90.0", widestReading("P: 4.0", WIDEST_PITCH))
        assertEquals("Az: -180.0", widestReading("Az: 157.5", WIDEST_YAW))
        assertEquals("with several gimbals the toolbar names the active one", "Gimbal 1 · Yaw follow · P: -12.3 · Y: 5.0", gimbalCellText(two))
        assertTrue(state.retractOffered)
        assertEquals(154, state.gimbals[0].deviceId)
        assertNull(gimbalIndicator(JSONObject("""{"shown":false}""")))
    }

    @Test
    fun `the yaw slider spans the range the core gives, and a gimbal with no yaw axis gets none`() {
        assertEquals(-180.0..180.0, gimbalIndicator(JSONObject("""{"shown":true,"yawRange":[-180,180]}"""))!!.yawRange)
        assertNull(turnRange(org.json.JSONArray("[0,0]")))
        assertNull(turnRange(null))
    }

    @Test
    fun `a tap maps to QGC's cooked screen fractions`() {
        val centre = screenFraction(50f, 50f, 100, 100)
        assertEquals(0f, centre.first, 0f)
        assertEquals(0f, centre.second, 0f)
        val corner = screenFraction(100f, 0f, 100, 100)
        assertEquals(1f, corner.first, 0f)
        assertEquals(1f, corner.second, 0f)
        assertEquals(-1f, screenFraction(0f, 100f, 100, 100).second, 0f)
        assertEquals(OnScreenGimbal(true, true), onScreenGimbal(JSONObject("""{"shown":true,"onScreen":{"enabled":true,"clickAndDrag":true}}""")))
        assertEquals("with on-screen control off the video still aims by drag, as CameraAimArea", OnScreenGimbal(false, true), onScreenGimbal(JSONObject("""{"shown":true,"onScreen":{"enabled":false,"clickAndDrag":true}}""")))
    }

    @Test
    fun `a hold sends its deflection, stops once when it ends, and a tap that never moved sends nothing`() {
        val sent = java.util.concurrent.CopyOnWriteArrayList<Pair<Float, Float>>()
        val hold = StickHold { x, y -> sent.add(x to y) }
        hold.steer(0f, 0f)
        hold.stop()
        assertEquals("a tap must not take the gimbal or the camera channels", emptyList<Pair<Float, Float>>(), sent.toList())
        hold.steer(0.5f, 0f)
        hold.steer(0.5f, 0f)
        hold.steer(0f, 0f)
        hold.steer(0f, 0f)
        hold.steer(0f, -1f)
        hold.stop()
        hold.stop()
        assertEquals(
            "a held deflection is resent so the core knows the head is there; back at the start it stops once; letting go stops once",
            listOf(0.5f to 0f, 0.5f to 0f, 0f to 0f, 0f to -1f, 0f to 0f),
            sent.toList(),
        )
    }

    @Test
    fun `stick calls reach the core in the order they were made, so the stop is never overtaken by the last tick`() {
        val landed = java.util.concurrent.CopyOnWriteArrayList<Int>()
        val done = java.util.concurrent.CountDownLatch(500)
        (0 until 500).forEach { n ->
            one.aircast.android.bridge.offMainInOrder {
                landed.add(n)
                done.countDown()
            }
        }
        assertTrue(done.await(10, java.util.concurrent.TimeUnit.SECONDS))
        assertEquals((0 until 500).toList(), landed.toList())
    }
}
