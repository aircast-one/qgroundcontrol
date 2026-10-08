package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class MockLinkFieldsTest {

    @Test
    fun `mock links are started from Add link, not a settings page of their own`() {
        val view = JSONObject("""{"pages":[{"title":"Mock Link","sections":[]}]}""")
        assertTrue(settingsPages(view).isEmpty())
        assertEquals(listOf(LinkType.Udp, LinkType.Tcp, LinkType.Serial, LinkType.Mock), addableLinkTypes(JSONObject("""{"linkTypeIds":["serial","udp","tcp","mock"]}""")))
    }

    @Test
    fun `start sends the vehicle key and options in the order the core reads them`() {
        val choices = MockLinkChoices(sendStatusText = true, camera = true, proximity = true, freshParams = true, vehicle = 1, videoStream = 3)
        assertEquals(listOf("apmCopter", true, true, false, true, true, 3), mockLinkArguments(choices))
    }

    @Test
    fun `fresh firmware parameters only reach an ArduPilot vehicle`() {
        val px4 = MockLinkChoices(freshParams = true, vehicle = 0)
        assertEquals(false, mockLinkArguments(px4)[5])
        assertEquals(false, withMockVehicle(MockLinkChoices(freshParams = true, vehicle = 1), 0).freshParams)
        assertEquals(listOf("px4", "apmCopter", "apmPlane", "apmSub", "apmRover", "generic"), MOCK_VEHICLES.map { it.first })
        assertEquals(6, MOCK_VIDEO_STREAMS.size)
    }
}
