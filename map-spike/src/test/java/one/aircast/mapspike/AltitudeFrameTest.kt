package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class AltitudeFrameTest {

    private fun item(frame: String, text: String = "50.0 m", band: String = "") = MissionItem(
        2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint",
        altitudeText = text, altitudeBandText = band, altitudeFrame = frame,
    )

    @Test
    fun `a height above launch is the one an operator already assumes, so it is left bare`() {
        assertEquals("50.0 m", altitudeWithFrame(item("launch")))
    }

    @Test
    fun `the two frames that are not the default say so`() {
        assertEquals("541 m AMSL", altitudeWithFrame(item("amsl", "541 m")))
        assertEquals("50.0 m above ground", altitudeWithFrame(item("terrain")))
    }

    @Test
    fun `a survey and a takeoff both reading fifty metres are no longer the same row`() {
        val takeoff = item("launch")
        val survey = item("terrain")

        assertEquals("50.0 m", itemDetail(takeoff))
        assertEquals("50.0 m above ground", itemDetail(survey))
    }

    @Test
    fun `a band takes the frame too, rather than only single heights`() {
        assertEquals(
            "0.0 m to 40.0 m AMSL",
            altitudeWithFrame(item("amsl", text = "", band = "0.0 m to 40.0 m")),
        )
    }

    @Test
    fun `a frame the core withheld leaves the height unlabelled rather than guessing one`() {
        assertEquals("50.0 m", altitudeWithFrame(item("")))
        assertNull(altitudeWithFrame(item("amsl", text = "")))
    }

    @Test
    fun `the summary chip carries the frame too, being the other place a height is spelled`() {
        val amsl = item("amsl", "541 m")

        assertEquals("#2 at 541 m AMSL", selectionText(MapHit.Waypoint(2), listOf(amsl), emptyList(), emptyList()))
        assertEquals("#2 at 50.0 m", selectionText(MapHit.Waypoint(2), listOf(item("launch")), emptyList(), emptyList()))
    }

    @Test
    fun `the field being typed into names its frame, being the furthest thing from the chip`() {
        assertEquals("Alt m", altitudeFieldLabel(item("launch")))
        assertEquals("Alt m AMSL", altitudeFieldLabel(item("amsl")))
        assertEquals("Alt m above ground", altitudeFieldLabel(item("terrain")))
        assertEquals("Alt m", altitudeFieldLabel(item("")))
    }

    @Test
    fun `a frame neither head has heard of is shown, not swallowed into the default`() {
        assertEquals("50.0 m SEABED", altitudeWithFrame(item("seabed")))
    }
}
