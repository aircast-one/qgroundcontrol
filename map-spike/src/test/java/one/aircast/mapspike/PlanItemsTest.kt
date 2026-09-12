package one.aircast.mapspike

import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Assert.assertNull
import org.junit.Test

class PlanItemsTest {
    private fun item(
        index: Int,
        sequence: Int,
        name: String,
        altitude: Double = 50.0,
        kind: String = "waypoint",
        commandId: Int = 16,
    ) = MissionItem(
        index, sequence, 41.0, 44.0, name, false, altitude,
        kind = kind, commandId = commandId,
        altitudeText = if (altitude.isNaN()) "" else "${altitude.toInt()} m",
    )

    @Test
    fun `a row carries the number, the name, the altitude and the marker colour`() {
        val rows = itemRows(listOf(item(1, 1, "Waypoint", altitude = 49.6)))

        assertEquals(listOf("1"), rows.map { it.number })
        assertEquals(listOf("Waypoint"), rows.map { it.name })
        assertEquals(listOf("49 m"), rows.map { it.detail })
        assertEquals(listOf(WAYPOINT_COLOUR), rows.map { it.colour })
    }

    @Test
    fun `the list is coloured the same way the map is, so a row and its marker match`() {
        val rows = itemRows(
            listOf(
                item(0, 0, "Mission Start", kind = "settings"),
                item(1, 1, "Takeoff", kind = "takeoff", commandId = 22),
                item(2, 2, "Return To Launch", kind = "command", commandId = MAV_CMD_NAV_RETURN_TO_LAUNCH),
            ),
        )

        assertEquals(listOf(START_COLOUR, TAKEOFF_COLOUR, RETURN_COLOUR), rows.map { it.colour })
    }

    @Test
    fun `a pattern spanning heights shows the band, having no single altitude`() {
        val survey = MissionItem(
            4, 4, 41.0, 44.0, "Survey", false, Double.NaN, kind = "survey",
            altitudeBandText = "585 m to 660 m",
        )

        assertEquals("585 m to 660 m", itemDetail(survey))
    }

    @Test
    fun `an item with its own altitude ignores any band`() {
        val waypoint = MissionItem(
            2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint",
            altitudeText = "50.0 m", altitudeBandText = "nonsense",
        )

        assertEquals("50.0 m", itemDetail(waypoint))
    }

    @Test
    fun `an item with no altitude says nothing rather than NaN`() {
        assertEquals("", itemRows(listOf(item(1, 1, "Waypoint", altitude = Double.NaN))).single().detail)
    }

    @Test
    fun `an item the vehicle did not name is still reachable by its number`() {
        assertEquals("Item 3", itemRows(listOf(item(3, 3, ""))).single().name)
    }

    @Test
    fun `a command with no place by design is listed and says nothing about position`() {
        val rows = itemRows(
            listOf(
                MissionItem(
                    1, 1, Double.NaN, Double.NaN, "Change Speed", false, Double.NaN,
                    kind = "command", commandId = 178, placed = false,
                    specifiesCoordinate = false,
                ),
            ),
        )

        assertEquals("", rows.single().detail)
        assertEquals(false, rows.single().placed)
    }

    @Test
    fun `an item that is supposed to have a place and has none still says so`() {
        val rows = itemRows(
            listOf(
                MissionItem(
                    1, 1, Double.NaN, Double.NaN, "Waypoint", false, Double.NaN,
                    kind = "waypoint", commandId = 16, placed = false,
                    specifiesCoordinate = true,
                ),
            ),
        )

        assertEquals(NO_POSITION, rows.single().detail)
    }

    @Test
    fun `an ArduPilot takeoff specifies an altitude and no place, so the altitude is what it says`() {
        val takeoff = MissionItem(
            1, 1, Double.NaN, Double.NaN, "Takeoff", false, 50.0,
            kind = KIND_TAKEOFF, commandId = 22, placed = false, altitudeText = "50 m",
        )

        assertEquals("50 m", itemRows(listOf(takeoff)).single().detail)
    }

    @Test
    fun `the altitude is whatever the core spelled, because the operator may not be in metres`() {
        val feet = MissionItem(
            1, 1, 41.0, 44.0, "Waypoint", false, 75.0,
            kind = "waypoint", commandId = 16, altitudeText = "246 ft",
        )

        assertEquals("246 ft", itemRows(listOf(feet)).single().detail)
    }

    @Test
    fun `an item sitting after the land says so, because the map just draws no line to it`() {
        val stranded = MissionItem(
            3, 3, 41.0, 44.0, "Waypoint", false, 50.0,
            routed = false, kind = "waypoint", commandId = 16, afterRouteEnds = true,
            altitudeText = "50 m",
        )

        assertEquals("50 m \u00b7 after the route ends", itemRows(listOf(stranded)).single().detail)
    }

    @Test
    fun `an item the route does not pass through is not accused of being stranded`() {
        val roi = MissionItem(
            2, 2, 41.0, 44.0, "Region Of Interest", false, Double.NaN,
            routed = false, kind = "roi", commandId = 201,
        )

        assertEquals("", itemRows(listOf(roi)).single().detail)
    }

    @Test
    fun `a row is found by the index the map selects with, not by position`() {
        val rows = itemRows(listOf(item(4, 1, "Waypoint"), item(9, 2, "Land", kind = "land", commandId = 21)))

        assertEquals("Land", rowAt(rows, 9)?.name)
        assertNull(rowAt(rows, 1))
    }
}

class UnplacedSelectionTest {
    private val unplaced = MissionItem(
        1, 1, Double.NaN, Double.NaN, "Takeoff", false, Double.NaN,
        kind = "takeoff", commandId = 22, placed = false,
    )

    @Test
    fun `selecting an item the map cannot draw is not thrown away on the next read`() {
        assertTrue(
            selectionSurvives(
                MapHit.Waypoint(1), listOf(unplaced),
                emptyList(), emptyList(), emptyList(), emptyList(),
            ),
        )
    }

    @Test
    fun `a selection of an item that left the plan is still thrown away`() {
        assertFalse(
            selectionSurvives(
                MapHit.Waypoint(1), emptyList(),
                emptyList(), emptyList(), emptyList(), emptyList(),
            ),
        )
    }
}

class WorthListingTest {
    private fun item(index: Int) =
        MissionItem(index, index, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint")

    @Test
    fun `a plan holding only its settings item has nothing to list`() {
        assertFalse(worthListing(listOf(item(HOME_ITEM))))
    }

    @Test
    fun `one real item is enough to be worth listing`() {
        assertTrue(worthListing(listOf(item(HOME_ITEM), item(1))))
    }

    @Test
    fun `no plan at all is nothing to list`() {
        assertFalse(worthListing(emptyList()))
    }
}

class InsertAfterTest {
    private fun item(index: Int) =
        MissionItem(index, index, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint")

    private val plan = listOf(item(0), item(1), item(2))

    @Test
    fun `with nothing selected an item goes on the end`() {
        assertEquals(AT_END, insertAfter(null, plan))
    }

    @Test
    fun `a new item follows the one the operator selected, as it does in QGC`() {
        assertEquals(2, insertAfter(MapHit.Waypoint(1), plan))
    }

    @Test
    fun `selecting the last item still means the end, not a place past it`() {
        assertEquals(AT_END, insertAfter(MapHit.Waypoint(2), plan))
    }

    @Test
    fun `a selection the plan no longer holds does not name a position`() {
        assertEquals(AT_END, insertAfter(MapHit.Waypoint(9), plan))
    }

    @Test
    fun `a selection that is not an item does not move the insertion point`() {
        assertEquals(AT_END, insertAfter(MapHit.SurveyVertex(1, 0), plan))
    }
}

class SelectionSequenceTest {
    private val plan = listOf(
        MissionItem(0, 0, 41.0, 44.0, "Mission Start", false, 0.0, kind = "settings"),
        MissionItem(1, 1, 41.0, 44.0, "Takeoff", false, 50.0, kind = KIND_TAKEOFF),
        MissionItem(2, 4, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint"),
    )

    @Test
    fun `the sequence is sent, not the index, because they are not the same number`() {
        assertEquals(4, selectionSequence(MapHit.Waypoint(2), plan))
    }

    @Test
    fun `nothing selected selects nothing`() {
        assertNull(selectionSequence(null, plan))
    }

    @Test
    fun `a selection the plan no longer holds sends nothing`() {
        assertNull(selectionSequence(MapHit.Waypoint(9), plan))
    }

    @Test
    fun `a fence vertex is not a mission item and does not move the plan view`() {
        assertNull(selectionSequence(MapHit.FenceVertex(0, 1), plan))
    }
}

class AddingAfterTextTest {
    private val plan = listOf(
        MissionItem(0, 0, 41.0, 44.0, "Mission Start", false, 0.0, kind = "settings"),
        MissionItem(2, 5, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint"),
    )

    @Test
    fun `the line names the sequence the operator sees on the marker`() {
        assertEquals("Adding after #5", addingAfterText(MapHit.Waypoint(2), plan))
    }

    @Test
    fun `with nothing selected the insertion point is the end and needs no line`() {
        assertNull(addingAfterText(null, plan))
    }

    @Test
    fun `a fence vertex does not move the insertion point, so it says nothing`() {
        assertNull(addingAfterText(MapHit.FenceVertex(0, 1), plan))
    }

    @Test
    fun `a selection the plan no longer holds says nothing rather than a stale number`() {
        assertNull(addingAfterText(MapHit.Waypoint(9), plan))
    }
}

class LegTextTest {
    private fun item(
        distance: Double,
        distanceText: String = "449 m",
        azimuthText: String = "47°",
        altitudeChange: Double = 0.0,
        altitudeChangeText: String = "",
    ) = MissionItem(
        2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint", commandId = 16,
        distance = distance, distanceText = distanceText, azimuthText = azimuthText,
        altitudeChange = altitudeChange, altitudeChangeText = altitudeChangeText,
    )

    @Test
    fun `a leg reads as its length and its bearing, both spelled by the core`() {
        assertEquals("449 m · 47°", legText(item(449.0)))
    }

    @Test
    fun `the first item has no leg into it, and QGC says so with a zero rather than a null`() {
        assertNull(legText(item(0.0)))
    }

    @Test
    fun `an item the controller has not measured says nothing`() {
        assertNull(legText(item(Double.NaN)))
    }

    @Test
    fun `a bearing without a distance is not a leg`() {
        assertNull(legText(item(0.0, distanceText = "", azimuthText = "47°")))
    }

    @Test
    fun `a climb straight up still says the climb, having no distance to report`() {
        assertEquals(
            "+10.0 m",
            legText(item(0.0, altitudeChange = 10.0, altitudeChangeText = "+10.0 m")),
        )
    }

    @Test
    fun `a leg with neither distance nor climb says nothing`() {
        assertNull(legText(item(0.0)))
    }

    @Test
    fun `a climbing leg says how much it climbs, signed and in the vertical unit`() {
        assertEquals(
            "449 m · 47° · +12.0 m",
            legText(item(449.0, altitudeChange = 12.0, altitudeChangeText = "+12.0 m")),
        )
    }

    @Test
    fun `a level leg says nothing about climbing rather than plus zero`() {
        assertEquals(
            "449 m · 47°",
            legText(item(449.0, altitudeChange = 0.0, altitudeChangeText = "+0.0 m")),
        )
    }

    @Test
    fun `a measured leg with only one of the two still says what it has`() {
        assertEquals("449 m", legText(item(449.0, azimuthText = "")))
    }
}

class BlockedReasonTest {

    @Test
    fun `an item that blocks the save says why, where the operator is looking`() {
        val item = MissionItem(
            4, 4, 41.0, 44.0, "Landing pattern", false, Double.NaN, kind = "land",
            blockedReason = "Landing point not set",
        )

        assertEquals("Landing point not set", itemDetail(item))
    }

    @Test
    fun `a reason joins whatever else the row already says`() {
        val item = MissionItem(
            2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint",
            altitudeText = "50.0 m", blockedReason = "Needs a value",
        )

        assertEquals("50.0 m · Needs a value", itemDetail(item))
    }

    @Test
    fun `an item with nothing wrong says nothing extra`() {
        val item = MissionItem(2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint", altitudeText = "50.0 m")

        assertEquals("50.0 m", itemDetail(item))
    }
}

class PhotosTextTest {

    @Test
    fun `a pattern that takes photos says how many`() {
        val survey = MissionItem(
            5, 5, 41.0, 44.0, "Survey", false, Double.NaN, kind = "survey",
            altitudeBandText = "0.0 m to 40.0 m", cameraShots = 340,
        )

        assertEquals("0.0 m to 40.0 m · 340 photos", itemDetail(survey))
    }

    @Test
    fun `one photo is not one photos`() {
        assertEquals("1 photo", photosText(1))
        assertEquals("2 photos", photosText(2))
    }

    @Test
    fun `an item that takes none says nothing about photos`() {
        assertEquals(null, photosText(0))
        assertEquals(null, photosText(-1))

        val waypoint = MissionItem(2, 2, 41.0, 44.0, "Waypoint", false, 50.0, kind = "waypoint", altitudeText = "50.0 m")
        assertEquals("50.0 m", itemDetail(waypoint))
    }
}
