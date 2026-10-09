package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class CameraBeamTest {
    private val view = JSONObject(
        """{"available":true,"beam":[{"latitude":-35.36,"longitude":149.16},{"latitude":-35.3597,"longitude":149.1604},{"latitude":-35.358,"longitude":149.162},{"latitude":-35.3585,"longitude":149.1628},{"latitude":-35.36,"longitude":149.1606},{"latitude":-35.36,"longitude":149.16}]}""",
    )

    @Test
    fun `the beam is the polygon the core places in front of the drone`() {
        val beam = cameraBeam(view)
        assertEquals(6, beam.size)
        assertEquals(TrackPoint(-35.36, 149.16), beam.first())
        assertEquals(1, beamFeatures(beam).features()?.size)
    }

    @Test
    fun `nothing is drawn while the core cannot place the camera`() {
        assertTrue(cameraBeam(JSONObject("""{"available":false,"beam":[]}""")).isEmpty())
        assertTrue(cameraBeam(null).isEmpty())
        assertEquals(0, beamFeatures(emptyList()).features()?.size)
    }
}
