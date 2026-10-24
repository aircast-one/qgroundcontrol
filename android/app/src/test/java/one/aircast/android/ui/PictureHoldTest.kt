package one.aircast.android.ui

import androidx.compose.ui.geometry.Offset
import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class PictureHoldTest {
    private val spot = Offset(0.25f, 0.75f)

    @Test
    fun `a spot on the ground opens the video menu with the core's actions and its field of view warning`() {
        val warning = "The camera's field of view isn't set, so this spot may be off. Set it in Settings > Gimbal."
        val press = picturePress(
            spot,
            JSONObject("""{"ok":true,"latitude":47.4,"longitude":8.5,"actions":["GoTo","Orbit","Roi","StopLooking"],"warning":"$warning"}"""),
            hasGimbal = true,
        )
        assertEquals(MapPoint(47.4, 8.5), press.point)
        assertEquals(listOf("GoTo", "Orbit", "Roi", "StopLooking"), press.actions)
        assertEquals(warning, press.warning)
        assertEquals(spot, press.spot)
        assertNull(picturePress(spot, JSONObject("""{"ok":true,"latitude":47.4,"longitude":8.5,"actions":[],"warning":null}"""), hasGimbal = true).warning)
    }

    @Test
    fun `a spot that cannot be placed says why in the words QGC uses`() {
        val sky = picturePress(spot, JSONObject("""{"ok":false,"refusal":"notGround","reason":"That is above the horizon, or farther than 2 km."}"""), hasGimbal = true)
        assertNull(sky.point)
        assertEquals(BELOW_HORIZON, sky.reason)
        assertEquals(NO_GIMBAL, picturePress(spot, null, hasGimbal = false).reason)
        assertNull(picturePress(spot, JSONObject("""{"ok":true,"latitude":47.4,"longitude":8.5}"""), hasGimbal = false).point)
        val unplaced = "Where the aircraft is and where its camera looks are not known yet."
        assertEquals(unplaced, picturePress(spot, JSONObject("""{"ok":false,"refusal":"notPlaced","reason":"$unplaced"}"""), hasGimbal = true).reason)
    }

    @Test
    fun `the reticle says orbiting over looking, and stop pauses an orbit but cancels a look-at`() {
        assertEquals(Pointing.Orbiting, pointing(looking = true, orbiting = true))
        assertEquals(PAUSE_VEHICLE_PATH, Pointing.Orbiting.stopPath)
        assertEquals(Pointing.Looking, pointing(looking = true, orbiting = false))
        assertEquals(STOP_ROI_PATH, Pointing.Looking.stopPath)
        assertNull(pointing(looking = false, orbiting = false))
        assertEquals("Looking at this point", Pointing.Looking.text)
        assertEquals("Orbiting this point", Pointing.Orbiting.text)
    }
}
