package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertNotEquals
import org.junit.Assert.assertTrue
import org.junit.Test

class RallyAltitudeEditTest {

    private val imperial = JSONObject(
        """{"kind":"object","class":"Fences","rallyPoints":[
             {"index":0,"latitude":47.397,"longitude":8.545,
              "altitude":164.0,"altitudeUnits":"ft","altitudeText":"164 ft",
              "altitudeMetres":50.0,
              "altitudePath":"plan.rallyPointController.points.0.textFieldFacts.2"}]}""",
    )

    @Test
    fun `the edited number is the operator's unit, never the metres used for a drag`() {
        val point = rallyPoints(imperial).single()

        assertEquals(164.0, point.altitude, 1e-9)
        assertEquals(
            "a drag writes a whole coordinate the bridge reads as metres, so the two numbers " +
                "describe the same height and only one of them belongs in the field",
            50.0,
            point.altitudeMetres,
            1e-9,
        )
        assertNotEquals(point.altitude, point.altitudeMetres, 1e-9)
        assertEquals("164", altitudeFieldText(point.altitude))
    }

    @Test
    fun `the label carries the served unit, not a hardcoded metre`() {
        assertEquals("Alt ft", rallyAltitudeLabel(rallyPoints(imperial).single()))
    }

    @Test
    fun `the field writes to the fact path, which converts, not to the coordinate`() {
        val point = rallyPoints(imperial).single()

        assertEquals("plan.rallyPointController.points.0.textFieldFacts.2", point.altitudePath)
        assertTrue(
            "the fact takes the cooked number and QGC converts it; writing 164 into the " +
                "coordinate instead would set the point to 164 metres",
            point.altitudePath.endsWith("textFieldFacts.2"),
        )
    }

    @Test
    fun `a point whose altitude is not a fact offers no field`() {
        val notAFact = JSONObject(
            """{"kind":"object","class":"Fences","rallyPoints":[
                 {"index":0,"latitude":47.397,"longitude":8.545,
                  "altitude":null,"altitudeUnits":"","altitudeMetres":50.0,"altitudePath":""}]}""",
        )

        assertFalse(rallyAltitudeIsEditable(rallyPoints(notAFact).single()))
    }

    @Test
    fun `a core too old to serve the path offers no field rather than writing nowhere`() {
        val old = JSONObject(
            """{"kind":"object","class":"Fences","rallyPoints":[
                 {"index":0,"latitude":47.397,"longitude":8.545,"altitudeMetres":50.0}]}""",
        )

        assertFalse(rallyAltitudeIsEditable(rallyPoints(old).single()))
    }

    @Test
    fun `an altitude with nowhere to write it offers no field`() {
        val noPath = JSONObject(
            """{"kind":"object","class":"Fences","rallyPoints":[
                 {"index":0,"latitude":47.397,"longitude":8.545,
                  "altitude":164.0,"altitudeUnits":"ft","altitudeMetres":50.0}]}""",
        )

        assertFalse(
            "the path is the write target; a field that renders without one edits nothing " +
                "and reports success",
            rallyAltitudeIsEditable(rallyPoints(noPath).single()),
        )
    }

    @Test
    fun `a served point is editable`() {
        assertTrue(rallyAltitudeIsEditable(rallyPoints(imperial).single()))
    }

    @Test
    fun `a metric point labels itself in metres without a fallback`() {
        val metric = JSONObject(
            """{"kind":"object","class":"Fences","rallyPoints":[
                 {"index":0,"latitude":47.397,"longitude":8.545,
                  "altitude":50.0,"altitudeUnits":"m","altitudeMetres":50.0,
                  "altitudePath":"plan.rallyPointController.points.0.textFieldFacts.2"}]}""",
        )
        val point = rallyPoints(metric).single()

        assertEquals("Alt m", rallyAltitudeLabel(point))
        assertEquals(point.altitude, point.altitudeMetres, 1e-9)
    }
}
