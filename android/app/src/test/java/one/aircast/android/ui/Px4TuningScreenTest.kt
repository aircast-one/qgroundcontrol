package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class Px4TuningScreenTest {
    @Test
    fun `tabs, axes and sliders read from the tuning view`() {
        val tabs = tuningTabs(
            JSONObject(
                """{"available":true,"tabs":[{"name":"Rate Controller","title":"Rate","unit":"deg/s","extras":[],"axes":[{"name":"Roll","params":[""" +
                    """{"title":"Overall Multiplier (MC_ROLLRATE_K)","description":"d","param":"MC_ROLLRATE_K","min":0.3,"max":3,"step":0.05,""" +
                    """"fact":{"class":"Control","path":"vehicle.parameterManager.getParameter(-1,MC_ROLLRATE_K)","name":"MC_ROLLRATE_K","value":1,"valueString":"1.00"}}]}]}]}""",
            ),
        )
        assertEquals("Rate controller", tabs.single().name)
        val param = tabs.single().axes.single().params.single()
        assertEquals("MC_ROLLRATE_K", param.fact.name)
        assertEquals("Overall multiplier (MC_ROLLRATE_K)", param.title)
        assertEquals(1f, factNumber(param.fact))
        assertEquals(53, sliderSteps(param.min, param.max, param.step))
    }
}
