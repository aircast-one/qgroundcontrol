package one.aircast.android.ui

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Test

class ActiveVehicleTest {

    private val two = JSONObject(
        """
        {"count":2,"activeId":1,"ambiguous":true,"vehicles":[
          {"id":1,"name":"Quadrotor 1","link":"TCP 127.0.0.1:5771","active":true,
           "armed":false,"flying":false,"flightMode":"Stabilize","contactLost":false},
          {"id":2,"name":"Quadrotor 2","link":"TCP 127.0.0.1:5772","active":false,
           "armed":true,"flying":true,"flightMode":"Guided","contactLost":true}
        ]}
        """,
    )

    private val one = JSONObject(
        """
        {"count":1,"activeId":1,"ambiguous":false,"vehicles":[
          {"id":1,"name":"Quadrotor 1","link":"TCP 127.0.0.1:5771","active":true,
           "armed":false,"flying":false,"flightMode":"Stabilize","contactLost":false}
        ]}
        """,
    )

    @Test
    fun `the header names the aircraft only when more than one is connected`() {
        assertEquals(
            "one vehicle needs no name: the header is about the only thing it can be about",
            "Stabilize · Disarmed",
            activeVehicleTitle(vehicleChoices(one), "Stabilize · Disarmed"),
        )
        assertEquals(
            "with two connected, Arm and Takeoff act on one of them and the header is the only thing that says which",
            "Quadrotor 1 · Stabilize · Disarmed",
            activeVehicleTitle(vehicleChoices(two), "Stabilize · Disarmed"),
        )
    }

    @Test
    fun `a vehicle out of contact says so instead of what it was last doing`() {
        val lost = vehicleChoices(two).choices.single { it.id == 2 }
        assertEquals("No contact · TCP 127.0.0.1:5772", vehicleChoiceLine(lost))
        assertTrue(lost.contactLost)
    }

    @Test
    fun `a vehicle in contact says what it is doing and which link carries it`() {
        val live = vehicleChoices(two).choices.single { it.id == 1 }
        assertEquals("Stabilize · Disarmed · TCP 127.0.0.1:5771", vehicleChoiceLine(live))
        assertTrue(live.active)
    }

    @Test
    fun `flying outranks armed, because an armed vehicle on the ground is a different thing`() {
        val flying = vehicleChoices(two).choices.single { it.id == 2 }
        assertEquals("Guided · Flying", flying.state)
    }

    @Test
    fun `a contactLost the core could not answer is not read as a loss`() {
        val unknown = JSONObject(
            """{"ambiguous":true,"vehicles":[{"id":3,"name":"Rover 3","active":false,"contactLost":null}]}""",
        )
        assertFalse(
            "null means the vehicle does not track contact loss at all, which is not the same as having lost it",
            vehicleChoices(unknown).choices.single().contactLost,
        )
    }

    @Test
    fun `an empty view offers nothing rather than an empty name`() {
        assertTrue(vehicleChoices(null).choices.isEmpty())
        assertFalse(vehicleChoices(null).ambiguous)
        assertEquals("No vehicle", activeVehicleTitle(vehicleChoices(null), "No vehicle"))
    }

    @Test
    fun `a vehicle the core did not name still has something to tap`() {
        val nameless = JSONObject("""{"ambiguous":true,"vehicles":[{"id":7,"active":false}]}""")
        assertEquals("Vehicle 7", vehicleChoices(nameless).choices.single().name)
    }

    @Test
    fun `a vehicle that stopped answering is named even while another is being flown`() {
        val lost = lostVehicles(vehicleChoices(two))
        assertEquals(listOf(2), lost.map { it.id })
        assertEquals(
            "QGC announces this by voice and nowhere else, so the screen is the only place an operator can see it",
            "Quadrotor 2 is not answering",
            lostVehiclesText(lost),
        )
    }

    @Test
    fun `the vehicle being flown is not counted among the silent ones`() {
        val activeLost = JSONObject(
            """{"ambiguous":true,"vehicles":[
                 {"id":1,"name":"Quadrotor 1","active":true,"contactLost":true},
                 {"id":2,"name":"Quadrotor 2","active":false,"contactLost":false}]}""",
        )
        assertTrue(
            "the header already says Communication lost for the one in command, and saying it twice reads as two failures",
            lostVehicles(vehicleChoices(activeLost)).isEmpty(),
        )
    }

    @Test
    fun `more than one silent vehicle is counted rather than listed`() {
        val many = JSONObject(
            """{"ambiguous":true,"vehicles":[
                 {"id":1,"name":"Quadrotor 1","active":true,"contactLost":false},
                 {"id":2,"name":"Quadrotor 2","active":false,"contactLost":true},
                 {"id":3,"name":"Quadrotor 3","active":false,"contactLost":true}]}""",
        )
        assertEquals("2 other vehicles are not answering", lostVehiclesText(lostVehicles(vehicleChoices(many))))
    }

    @Test
    fun `nothing is said when every vehicle is answering`() {
        assertNull(lostVehiclesText(lostVehicles(vehicleChoices(one))))
    }
}
