package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Test

class PowerLiveCardTest {
    @Test
    fun theTilesReadTheFirstPackInOrder() {
        val view = JSONObject("""{"available":true,"packs":[{"facts":[{"name":"mahConsumed","units":"mAh","valueString":"1800"},{"name":"voltage","units":"v","valueString":"11.10"},{"name":"current","units":"A","valueString":"32.00"}]}]}""")
        assertEquals(
            listOf(PowerTile("VOLTAGE", "11.10", "V"), PowerTile("CURRENT", "32.00", "A"), PowerTile("USED", "1800", "mAh")),
            powerTiles(view),
        )
    }

    @Test
    fun noBatteryMeansNoCard() {
        assertEquals(emptyList<PowerTile>(), powerTiles(JSONObject("""{"available":false}""")))
        assertEquals(emptyList<PowerTile>(), powerTiles(null))
    }
}
