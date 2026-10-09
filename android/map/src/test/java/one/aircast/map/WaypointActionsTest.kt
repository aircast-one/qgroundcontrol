package one.aircast.map

import org.json.JSONObject
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Test

class WaypointActionsTest {
    private val labels = listOf("No change", "Take photo", "Take photos (time)", "Take photos (distance)", "Stop taking photos", "Start recording video", "Stop recording video")
    private val idleCamera = CameraExtras(null, null, "m", modeSupported = true, commandsMode = false, mode = 0, commandsGimbal = false, pitch = 0.0, yaw = 0.0)
    private val noHold = WaypointHold(0.0, "s", "h")
    private val noTurn = WaypointYaw(null, "deg", "y")

    @Test
    fun `a plain waypoint has no actions and offers every one it can carry`() {
        assertEquals(emptyList<WaypointAction>(), activeActions(noHold, noTurn, CameraChoices(labels, 0), idleCamera))
        assertEquals(
            listOf("Hover", "Photo", "Video", "Tilt camera", "Camera mode", "Turn aircraft"),
            offeredActions(noHold, noTurn, CameraChoices(labels, 0), idleCamera).map { it.title },
        )
    }

    @Test
    fun `each action set on the waypoint shows once with its value and leaves the offers`() {
        val active = activeActions(
            WaypointHold(5.0, "s", "h"),
            WaypointYaw(90.0, "deg", "y"),
            CameraChoices(labels, 2),
            idleCamera.copy(intervalTime = 10.0, commandsGimbal = true, pitch = -45.0, commandsMode = true, mode = 1),
        )
        assertEquals(
            listOf("Hover 5 s", "Take photos (time) every 10 s", "Gimbal -45° / 0°", "Camera mode Video", "Turn aircraft 90°"),
            active.map { "${it.title} ${it.value}" },
        )
        assertEquals(emptyList<ActionOffer>(), offeredActions(WaypointHold(5.0, "s", "h"), WaypointYaw(90.0, "deg", "y"), CameraChoices(labels, 2), idleCamera.copy(commandsGimbal = true, commandsMode = true)))
    }

    @Test
    fun `photo and video start from their first action, and the editor offers the rest`() {
        assertEquals(listOf(1, 5), listOf(startingChoice(labels, "photo"), startingChoice(labels, "video")))
        assertNull(startingChoice(listOf("No change"), "photo"))
    }

    @Test
    fun `video actions read as video and stopping photos reads as stop`() {
        assertEquals(
            listOf(R.drawable.plan_photo, R.drawable.plan_video, R.drawable.plan_video_off, R.drawable.plan_stop),
            listOf("Take photo", "Start recording video", "Stop recording video", "Stop taking photos").map(::cameraIcon),
        )
    }

    @Test
    fun `an unset heading reads as no turn and an item without one offers none`() {
        assertEquals(WaypointYaw(null, "deg", "p"), waypointYaw(JSONObject("""{"yaw":{"value":null,"units":"deg","path":"p"}}""")))
        assertEquals(WaypointYaw(45.0, "deg", "p"), waypointYaw(JSONObject("""{"yaw":{"value":45.0,"units":"deg","path":"p"}}""")))
        assertNull(waypointYaw(JSONObject("""{"yaw":null}""")))
    }

    @Test
    fun `each leg is labelled at its middle with the distance the core measured`() {
        val home = MissionItem(0, 0, 41.0, 44.0, "Home", true, Double.NaN)
        val first = MissionItem(1, 1, 41.0, 44.002, "Waypoint", true, 50.0, distance = 168.0, distanceText = "168 m")
        val onTop = MissionItem(2, 2, 41.0, 44.002, "Waypoint", true, 50.0, distance = 0.0, distanceText = "0 m")
        val legs = legLabels(listOf(home, first, onTop))
        assertEquals(listOf("168 m"), legs.map { it.second })
        assertEquals(44.001, legs.single().first.longitude, 1e-9)
        assertEquals(41.0, legs.single().first.latitude, 1e-9)
    }
}
