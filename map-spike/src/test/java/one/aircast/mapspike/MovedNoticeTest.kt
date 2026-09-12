package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Test

class MovedNoticeTest {

    private val items = listOf(
        MissionItem(4, 72, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint"),
    )

    @Test
    fun `a moved waypoint is named by the number on its marker`() {
        assertEquals("Moved #72", movedText(MapHit.Waypoint(4), items))
    }

    @Test
    fun `an item the list no longer holds still says something happened`() {
        assertEquals("Moved an item", movedText(MapHit.Waypoint(99), items))
    }

    @Test
    fun `every other kind of handle names what it moved`() {
        assertEquals("Moved a fence corner", movedText(MapHit.FenceVertex(0, 2), items))
        assertEquals("Moved a survey corner", movedText(MapHit.SurveyVertex(1, 0), items))
        assertEquals("Moved a rally point", movedText(MapHit.Rally(0), items))
        assertEquals("Moved a fence circle", movedText(MapHit.CircleCentre(0), items))
        assertEquals("Changed a fence radius", movedText(MapHit.Circle(0), items))
    }
}
