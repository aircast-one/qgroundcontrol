package one.aircast.mapspike

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class ItemCameraTest {

    private fun view(
        available: Boolean = true,
        commandsGimbal: Boolean = true,
        action: String = """{"value":1,"text":"Take photo","units":""}""",
        pitch: String = """{"value":-45.0,"text":"-45.0","units":"deg"}""",
        yaw: String = """{"value":0.0,"text":"0.0","units":"deg"}""",
    ) = JSONObject(
        """{"kind":"object","class":"ItemCamera","index":1,"available":$available,
           "commandsGimbal":$commandsGimbal,"cameraAction":$action,
           "gimbalPitch":$pitch,"gimbalYaw":$yaw}""",
    )

    @Test
    fun `an item with no camera section says nothing`() {
        assertNull(itemCameraText(null))
        assertNull(itemCameraText(view(available = false)))
    }

    @Test
    fun `an item that commands the gimbal states the action and both angles`() {
        assertEquals("Take photo · gimbal -45.0 deg / 0.0 deg", itemCameraText(view()))
    }

    @Test
    fun `angles are withheld for an item that does not command the gimbal`() {
        val untouched = view(commandsGimbal = false, pitch = "null", yaw = "null")

        assertEquals("Take photo", itemCameraText(untouched))
    }

    @Test
    fun `an item that touches neither is a blank line rather than an empty bullet`() {
        val nothing = view(commandsGimbal = false, action = "null", pitch = "null", yaw = "null")

        assertNull(itemCameraText(nothing))
    }

    @Test
    fun `a bare number is not a camera action and is not drawn as one`() {
        val unlabelled = view(
            commandsGimbal = false,
            action = """{"value":0,"text":"0.000","units":""}""",
            pitch = "null",
            yaw = "null",
        )

        assertNull(itemCameraText(unlabelled))
        assertEquals("", namedAction("0.000"))
        assertEquals("", namedAction("-1"))
        assertEquals("Take photo", namedAction("Take photo"))
    }

    @Test
    fun `a number beside a gimbal angle still leaves the angle`() {
        val mixed = view(action = """{"value":0,"text":"0.000","units":""}""")

        assertEquals("gimbal -45.0 deg / 0.0 deg", itemCameraText(mixed))
    }

    @Test
    fun `a measure with no units is not given a trailing space`() {
        val unitless = view(
            commandsGimbal = false,
            action = """{"value":1,"text":"Take photo","units":null}""",
            pitch = "null",
            yaw = "null",
        )

        assertEquals("Take photo", itemCameraText(unitless))
    }
}

class CameraChoicesTest {

    private val enums = """["No change","Take photo","Take photos (time)","Stop taking photos"]"""

    private fun view(action: String) = JSONObject(
        """{"kind":"object","class":"ItemCamera","index":2,"available":true,
           "commandsGimbal":false,"cameraAction":$action,"gimbalPitch":null,"gimbalYaw":null}""",
    )

    @Test
    fun `a fact with choices offers them and says which is chosen`() {
        val choices = cameraChoices(
            view("""{"value":1,"text":"Take photo","units":"","choices":$enums,"choice":1}"""),
        )!!

        assertEquals(4, choices.labels.size)
        assertEquals(1, choices.chosen)
    }

    @Test
    fun `a blank label keeps its place, because choice indexes the list the core sent`() {
        val gap = cameraChoices(
            view("""{"value":2,"text":"","units":"","choices":["No change","","Take photo"],"choice":2}"""),
        )!!

        assertEquals(
            "dropping the blank shifted every later label down one, so choice 2 named the entry " +
                "before the one the core chose - on a plan item that is a different camera command",
            "Take photo",
            gap.labels[gap.chosen],
        )
    }

    @Test
    fun `the Fact metadata spelling offers nothing, because the core does not serve it`() {
        assertNull(
            "itemcamera.rs serves choices and choice - the cooked list and the chosen index - and " +
                "never enumStrings or enumIndex. Reading the Qt spelling meant the picker could " +
                "never populate, and the fixture that said otherwise was written by hand",
            cameraChoices(
                view("""{"value":1,"text":"Take photo","units":"","enumStrings":$enums,"enumIndex":1}"""),
            ),
        )
    }

    @Test
    fun `a plain numeric fact offers nothing, because it is not a choice`() {
        assertNull(cameraChoices(view("""{"value":-45.0,"text":"-45.0","units":"deg"}""")))
        assertNull(cameraChoices(view("""{"value":0,"text":"0","units":"","choices":[]}""")))
        assertNull(cameraChoices(null))
    }

    @Test
    fun `the label comes from the choice list when there is one`() {
        val named = view("""{"value":1,"text":"1.000","units":"","choices":$enums,"choice":1}""")

        assertEquals("Take photo", actionLabel(named))
        assertEquals("Take photo", itemCameraText(named))
    }

    @Test
    fun `a fact with no metadata keeps whatever its text could render`() {
        assertEquals("Something", actionLabel(view("""{"value":3,"text":"Something","units":""}""")))
    }

    @Test
    fun `an index outside the list falls back rather than crashing`() {
        val odd = view("""{"value":9,"text":"9.000","units":"","choices":$enums,"choice":9}""")

        assertEquals("", actionLabel(odd))
        assertNull(itemCameraText(odd))
    }
}
