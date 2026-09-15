package one.aircast.mapspike

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
    fun `one radio carrying the whole fleet prints its name under nobody`() {
        val one = VehicleChoice(id = 1, name = "Quadrotor 1", state = "Stabilize · Disarmed", link = "SiK", active = true, contactLost = false, latitude = 0.0, longitude = 0.0)
        val two = one.copy(id = 2, name = "Quadrotor 2", active = false)
        assertEquals(
            "the link is drawn to tell two vehicles apart; the same string under every name is the " +
                "noise it was added to cut, and it pushes the state text off a narrow row",
            false,
            linkDistinguishes(listOf(one, two)),
        )
        assertEquals("Stabilize · Disarmed", vehicleChoiceLine(one, linkDistinguishes(listOf(one, two))))
    }

    @Test
    fun `two radios keep their names, because that is the question being asked`() {
        val a = VehicleChoice(id = 1, name = "A", state = "Disarmed", link = "TCP 5771", active = true, contactLost = false, latitude = 0.0, longitude = 0.0)
        val b = a.copy(id = 2, name = "B", link = "TCP 5772", active = false, contactLost = true)
        assertEquals(true, linkDistinguishes(listOf(a, b)))
        assertEquals("No contact · TCP 5772", vehicleChoiceLine(b, true))
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

    private fun gate(heading: String) = UploadGate(
        canSend = true, canProceed = true, pausesFirst = false,
        heading = heading, refusal = "", proceedTitle = "Upload",
    )

    @Test
    fun `the upload names the aircraft it is about to send to`() {
        assertEquals(
            "a mission is being written to one of two aircraft and the dialog covers the header that says which",
            "Upload this plan to Quadrotor 1?",
            uploadHeading(gate(""), vehicleChoices(two)),
        )
        assertEquals(
            "Send the plan to the vehicle to Quadrotor 1",
            uploadHeading(gate("Send the plan to the vehicle"), vehicleChoices(two)),
        )
    }

    @Test
    fun `one vehicle leaves the core's question exactly as it was`() {
        assertEquals("Upload this plan?", uploadHeading(gate(""), vehicleChoices(one)))
        assertEquals(
            "the core writes this heading when a plan is being replaced, and a name on the only aircraft connected adds nothing",
            "Replace the plan on the vehicle?",
            uploadHeading(gate("Replace the plan on the vehicle?"), vehicleChoices(one)),
        )
    }

    private fun listOfVehicles(vararg ids: Int, activeId: Int) = vehicleChoices(
        JSONObject(
            """{"ambiguous":${ids.size > 1},"vehicles":[""" +
                ids.joinToString(",") {
                    """{"id":$it,"name":"Quadrotor $it","active":${it == activeId}}"""
                } + "]}",
        ),
    )

    @Test
    fun `the aircraft is handed over before the one that left is removed`() {
        assertEquals(
            "QGC promotes the survivor FIRST and deletes the departing vehicle a moment later, " +
                "so a rule keyed on the old one being gone never fires - measured on the emulator",
            "Quadrotor 1 stopped answering. Now flying Quadrotor 2.",
            handoverNotice(listOfVehicles(1, 2, activeId = 1), listOfVehicles(1, 2, activeId = 2), asked = null),
        )
        assertEquals(
            "Quadrotor 1 is gone. Now flying Quadrotor 2.",
            handoverNotice(listOfVehicles(1, 2, activeId = 1), listOfVehicles(2, activeId = 2), asked = null),
        )
    }

    @Test
    fun `an operator switching vehicles is told nothing, having just done it`() {
        assertNull(
            "the same list change as a handover, and only the head knows it asked for this one",
            handoverNotice(listOfVehicles(1, 2, activeId = 1), listOfVehicles(1, 2, activeId = 2), asked = 2),
        )
    }

    @Test
    fun `a request lasts exactly one change of vehicle`() {
        assertTrue(
            "one transition is all a request can explain; keeping it past that silences the next " +
                "handover, and a request the core accepted but the vehicle never honoured would " +
                "silence every handover after it",
            activeChanged(listOfVehicles(1, 2, activeId = 1), listOfVehicles(1, 2, activeId = 2)),
        )
        assertFalse(activeChanged(listOfVehicles(1, 2, activeId = 1), listOfVehicles(1, 2, activeId = 1)))
        assertFalse(
            "the gap with nothing active is not a change, or the request would be spent before it landed",
            activeChanged(
                listOfVehicles(1, 2, activeId = 1),
                vehicleChoices(JSONObject("""{"vehicles":[{"id":2,"name":"Quadrotor 2","active":false}]}""")),
            ),
        )
        assertFalse(activeChanged(null, listOfVehicles(1, activeId = 1)))
    }

    @Test
    fun `a handover back to a vehicle the operator once chose is still announced`() {
        assertEquals(
            "asked for 2 an hour ago, flew 1, then 1 died - measured on the emulator, where a kept " +
                "request swallowed the notice entirely",
            "Quadrotor 1 stopped answering. Now flying Quadrotor 2.",
            handoverNotice(listOfVehicles(1, 2, activeId = 1), listOfVehicles(1, 2, activeId = 2), asked = null),
        )
    }

    @Test
    fun `an old request does not silence the next handover`() {
        assertEquals(
            "Quadrotor 2 stopped answering. Now flying Quadrotor 1.",
            handoverNotice(listOfVehicles(1, 2, activeId = 2), listOfVehicles(1, 2, activeId = 1), asked = 2),
        )
    }

    @Test
    fun `the other vehicle leaving is not a handover`() {
        assertNull(
            "the one being flown did not change, and its silence is already on the header",
            handoverNotice(listOfVehicles(1, 2, activeId = 1), listOfVehicles(1, activeId = 1), asked = null),
        )
    }

    @Test
    fun `the last vehicle leaving says nothing, because No vehicle already does`() {
        assertNull(handoverNotice(listOfVehicles(1, activeId = 1), vehicleChoices(null), asked = null))
    }

    @Test
    fun `the first reading announces nothing`() {
        assertNull(handoverNotice(null, listOfVehicles(1, 2, activeId = 1), asked = null))
    }

    @Test
    fun `the gap between one vehicle leaving and the next being promoted is not a reading`() {
        val flying = listOfVehicles(1, 2, activeId = 1)
        val gap = vehicleChoices(
            JSONObject("""{"ambiguous":false,"vehicles":[{"id":2,"name":"Quadrotor 2","active":false}]}"""),
        )
        val promoted = listOfVehicles(2, activeId = 2)

        assertEquals(
            "measured on the emulator: killing the active vehicle's link emits THREE times - two " +
                "vehicles with 1 active, one vehicle with NO active, then 2 promoted. Keeping the " +
                "middle one loses who was being flown and the handover goes unannounced",
            flying,
            rememberedChoices(flying, gap),
        )
        assertEquals(promoted, rememberedChoices(flying, promoted))
        assertEquals(
            "by the time the survivor is promoted the old vehicle has already left the list",
            "Quadrotor 1 is gone. Now flying Quadrotor 2.",
            handoverNotice(rememberedChoices(flying, gap), promoted, asked = null),
        )
    }

    @Test
    fun `losing every vehicle forgets the one that was being flown`() {
        val flying = listOfVehicles(1, activeId = 1)
        assertNull(
            "No vehicle is already on the header, and the next aircraft to connect is a new flight " +
                "rather than a handover from the one that went away",
            rememberedChoices(flying, vehicleChoices(null)),
        )
    }

    @Test
    fun `the chooser is titled as the question it answers, not as a state`() {
        assertEquals(
            "it read Flying, which is a word for what the aircraft is doing - and the sheet opens " +
                "just as readily on a disarmed one sitting on the ground",
            "Fly which aircraft?",
            CHOOSER_TITLE,
        )
    }
}
