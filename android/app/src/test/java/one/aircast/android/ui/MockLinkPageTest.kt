package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class MockLinkPageTest {

    @Test
    fun `the mock link page is offered although it has no settings sections`() {
        val view = JSONObject("""{"pages":[{"title":"Mock Link","showsMockLink":true,"sections":[]}]}""")
        val page = settingsPages(view).single()
        assertTrue(page.showsMockLink)
        assertEquals(SettingsGroup.App, pageLook(page.title).group)
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
        assertEquals(listOf("px4", "apmCopter", "apmPlane", "apmSub", "apmRover", "generic"), MOCK_VEHICLES.map { it.first })
        assertEquals(6, MOCK_VIDEO_STREAMS.size)
    }
}
