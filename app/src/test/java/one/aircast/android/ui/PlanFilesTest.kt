package one.aircast.android.ui

import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.json.JSONArray
import org.json.JSONObject
import org.junit.Test

class SaveGuardTest {
    @Test
    fun `a ready plan saves`() {
        assertNull(saveBlockedReason(READY_FOR_SAVE))
    }

    @Test
    fun `a plan still fetching terrain is refused, because its altitudes are wrong`() {
        assertEquals(
            "Waiting on terrain data. Saving now would store wrong altitudes.",
            saveBlockedReason(NOT_READY_TERRAIN),
        )
    }

    @Test
    fun `a plan with incomplete items is refused`() {
        assertEquals(
            "Some items still need a position or a value.",
            saveBlockedReason(NOT_READY_DATA),
        )
    }

    @Test
    fun `an unreadable readiness check blocks the save rather than allowing it`() {
        assertEquals("The plan could not be checked for saving.", saveBlockedReason(null))
    }

    @Test
    fun `a state this build does not know about blocks the save`() {
        assertEquals("The plan could not be checked for saving.", saveBlockedReason(7))
    }
}

class PlanStatusTest {
    @Test
    fun `an untouched plan is new, not unsaved`() {
        assertEquals("New plan", planStatusText(null, dirty = false, offline = true))
    }

    @Test
    fun `an edited plan with no file says so`() {
        assertEquals("Unsaved plan", planStatusText(null, dirty = true, offline = true))
    }

    @Test
    fun `a saved plan is named`() {
        assertEquals("mission.plan", planStatusText("mission.plan", dirty = false, offline = true))
    }

    @Test
    fun `with no vehicle, dirty does mean the file is behind`() {
        assertEquals(
            "mission.plan · unsaved changes",
            planStatusText("mission.plan", dirty = true, offline = true),
        )
    }

    @Test
    fun `with a vehicle, a saved plan is not called unsaved`() {
        assertEquals(
            "mission.plan · not uploaded",
            planStatusText("mission.plan", dirty = true, offline = false),
        )
    }
}

class PlanActionsTest {
    private fun actions(
        syncing: Boolean = false,
        containsItems: Boolean = true,
        hasMissionItems: Boolean = true,
        offline: Boolean = false,
    ) = planActions(syncing, containsItems, hasMissionItems, offline)

    @Test
    fun `an empty plan cannot be saved over a real one`() {
        val can = actions(containsItems = false, hasMissionItems = false)
        assertEquals(false, can.save)
        assertEquals(false, can.exportKml)
    }

    @Test
    fun `opening stays available on an empty plan, because that is how you get one`() {
        assertEquals(true, actions(containsItems = false, hasMissionItems = false).open)
    }

    @Test
    fun `nothing is offered while a sync is in progress`() {
        val can = actions(syncing = true)
        assertEquals(false, can.open)
        assertEquals(false, can.save)
        assertEquals(false, can.exportKml)
    }

    @Test
    fun `a fence-only plan saves but exports no KML, because saveToKml writes only the mission`() {
        val can = actions(hasMissionItems = false)
        assertEquals(true, can.save)
        assertEquals(false, can.exportKml)
    }

    @Test
    fun `a plan with mission items offers everything`() {
        val can = actions()
        assertEquals(true, can.open)
        assertEquals(true, can.save)
        assertEquals(true, can.exportKml)
    }
}

class DestructiveActionsTest {
    @Test
    fun `a mission cannot be cleared from a vehicle that is not there`() {
        assertEquals(false, planActions(false, true, true, offline = true).clearMission)
        assertEquals(true, planActions(false, true, true, offline = false).clearMission)
    }

    @Test
    fun `a sync in progress stops the mission being cleared`() {
        assertEquals(false, planActions(true, true, true, offline = false).clearMission)
    }

    @Test
    fun `starting a new plan stays available with no vehicle and an empty plan`() {
        assertEquals(true, planActions(false, containsItems = false, hasMissionItems = false, offline = true).newPlan)
    }

    @Test
    fun `clearing the vehicle says it touches the aircraft, not just the plan`() {
        val copy = confirmCopy(PlanConfirm.ClearMission)
        assertEquals(true, copy.body.contains("aircraft"))
        assertEquals("Clear mission", copy.confirm)
    }

    @Test
    fun `each confirmation names the act rather than saying OK`() {
        listOf(PlanConfirm.Open, PlanConfirm.NewPlan, PlanConfirm.ClearMission)
            .map { confirmCopy(it).confirm }
            .forEach { assertEquals("vague confirm label: $it", false, it in listOf("OK", "Yes", "Confirm")) }
    }

    @Test
    fun `the two discarding actions warn that the loss is permanent`() {
        listOf(PlanConfirm.Open, PlanConfirm.NewPlan)
            .map { confirmCopy(it).body }
            .forEach { assertEquals("no warning in: $it", true, it.contains("cannot be recovered")) }
    }
}

class ClearHonestyTest {
    @Test
    fun `only the action that touches the aircraft is styled destructive`() {
        assertEquals(true, confirmCopy(PlanConfirm.ClearMission).destructive)
        assertEquals(false, confirmCopy(PlanConfirm.Open).destructive)
        assertEquals(false, confirmCopy(PlanConfirm.NewPlan).destructive)
    }
}

class LoadFailureTest {
    @Test
    fun `a loaded plan reports nothing`() {
        assertNull(loadFailureMessage(true))
    }

    @Test
    fun `a rejected file is named as the wrong kind of file`() {
        assertEquals(
            "That is not a plan file. The current plan is unchanged.",
            loadFailureMessage(false),
        )
    }

    @Test
    fun `a bridge that never answered does not get to blame the file`() {
        assertEquals(
            "The plan could not be loaded. The current plan is unchanged.",
            loadFailureMessage(null),
        )
    }
}

class BoundaryImportTest {
    @Test
    fun `the cache file keeps the suffix the parser reads`() {
        assertEquals("boundary.kml", boundaryCacheName("site.kml"))
        assertEquals("boundary.shp", boundaryCacheName("Site Boundary.SHP"))
        assertEquals("boundary.kml", boundaryCacheName("field.plan.kml"))
    }

    @Test
    fun `a name with no suffix falls back rather than losing the extension`() {
        assertEquals("boundary.kml", boundaryCacheName("boundary"))
        assertEquals("boundary.kml", boundaryCacheName(null))
    }

    @Test
    fun `a pattern with no area counts as nothing imported`() {
        assertEquals(true, importedNothing(null))
        assertEquals(true, importedNothing(0.0))
    }

    @Test
    fun `a pattern that covers ground counts as imported`() {
        assertEquals(false, importedNothing(0.5))
        assertEquals(false, importedNothing(5100.0))
    }
}

class WriteRefusalTest {
    @Test
    fun `an accepted write says nothing`() {
        assertNull(writeRefusal(true))
    }

    @Test
    fun `a refused write is named rather than left to look like a glitch`() {
        assertEquals("That change was not accepted.", writeRefusal(false))
    }

    @Test
    fun `the refusal does not guess at a cause it cannot know`() {
        val message = writeRefusal(false)!!
        listOf("vehicle", "property", "WRITE", "bridge")
            .forEach { assertEquals("claims a cause: $message", false, message.contains(it)) }
    }
}

class LinkFailureTest {
    @Test
    fun `a link call that finished says nothing`() {
        assertNull(linkFailure("connect", done = true))
    }

    @Test
    fun `a refused call names the action that did not happen`() {
        assertEquals("Could not connect that link.", linkFailure("connect", done = false))
        assertEquals("Could not disconnect that link.", linkFailure("disconnect", done = false))
        assertEquals("Could not remove that link.", linkFailure("remove", done = false))
    }
}

class AccelBlockTest {
    private fun named(name: String) = CALIBRATIONS.first { it.name == name }

    @Test
    fun `compass and level horizon wait for the accelerometer`() {
        assertEquals(true, blockedByAccel(named("Compass"), accelNeeded = true))
        assertEquals(true, blockedByAccel(named("Level Horizon"), accelNeeded = true))
    }

    @Test
    fun `the accelerometer is never blocked, because it is the way out`() {
        assertEquals(false, blockedByAccel(named("Accelerometer"), accelNeeded = true))
    }

    @Test
    fun `gyro and pressure do not depend on the accelerometer`() {
        assertEquals(false, blockedByAccel(named("Gyro"), accelNeeded = true))
        assertEquals(false, blockedByAccel(named("Pressure"), accelNeeded = true))
    }

    @Test
    fun `nothing is blocked once the accelerometer is calibrated`() {
        CALIBRATIONS.forEach { assertEquals(false, blockedByAccel(it, accelNeeded = false)) }
    }
}

class CalibrationStartTest {
    @Test
    fun `a calibration that started says nothing`() {
        assertNull(calibrationFailure("Compass", started = true))
    }

    @Test
    fun `a calibration that never started names which one`() {
        assertEquals("Compass calibration did not start.", calibrationFailure("Compass", false))
        assertEquals(
            "Accelerometer calibration did not start.",
            calibrationFailure("Accelerometer", false),
        )
    }
}

class CalibrationBeganTest {
    @Test
    fun `a calibration still running has begun`() {
        assertEquals(true, calibrationBegan(running = true, statusBefore = "", statusNow = ""))
    }

    @Test
    fun `one that finished before the first poll left its status behind`() {
        assertEquals(
            true,
            calibrationBegan(false, "", "Requesting pressure calibration... Successfully completed"),
        )
    }

    @Test
    fun `nothing running and nothing said means it never began`() {
        assertEquals(false, calibrationBegan(false, "old text", "old text"))
    }
}

class AltitudeLabelTest {
    @Test
    fun `the operator's own unit is used when both halves arrive`() {
        assertEquals("33 ft", altitudeLabel(meters = 10.0, converted = 32.8, unit = "ft"))
    }

    @Test
    fun `a missing conversion falls back to metres, not to a converted number`() {
        assertEquals("10 m", altitudeLabel(10.0, converted = null, unit = "ft"))
    }

    @Test
    fun `a missing unit falls back to metres, not to a bare number`() {
        assertEquals("10 m", altitudeLabel(10.0, converted = 32.8, unit = null))
        assertEquals("10 m", altitudeLabel(10.0, converted = 32.8, unit = ""))
    }
}

class UndrawnItemsTest {
    private fun item(cls: String, simple: Boolean = false, pattern: String = "") = JSONObject()
        .put("class", cls)
        .put("isSimpleItem", simple)
        .put("patternName", pattern)

    private fun plan(vararg items: JSONObject) = JSONArray().also { items.forEach(it::put) }

    @Test
    fun `an ordinary plan raises no warning`() {
        val ordinary = plan(
            item("MissionSettingsItem"),
            item("TakeoffMissionItem", simple = true),
            item("SimpleMissionItem", simple = true),
            item("SurveyComplexItem", pattern = "Survey"),
        )

        assertEquals(emptyList<String>(), undrawnItemNames(ordinary))
        assertNull(undrawnItemsWarning(undrawnItemNames(ordinary)))
    }

    @Test
    fun `a corridor scan is named in the warning`() {
        val mixed = plan(
            item("MissionSettingsItem"),
            item("SimpleMissionItem", simple = true),
            item("CorridorScanComplexItem", pattern = "Corridor Scan"),
        )

        assertEquals(listOf("Corridor Scan"), undrawnItemNames(mixed))
        assertEquals(
            "The map cannot draw Corridor Scan. Those items are still in the plan and will still be flown.",
            undrawnItemsWarning(undrawnItemNames(mixed)),
        )
    }

    @Test
    fun `each undrawn kind is named once however many the plan holds`() {
        val many = plan(
            item("CorridorScanComplexItem", pattern = "Corridor Scan"),
            item("CorridorScanComplexItem", pattern = "Corridor Scan"),
            item("StructureScanComplexItem", pattern = "Structure Scan"),
        )

        assertEquals(listOf("Corridor Scan", "Structure Scan"), undrawnItemNames(many))
    }

    @Test
    fun `an item with no pattern name falls back to its class`() {
        assertEquals(
            listOf("FixedWingLandingComplexItem"),
            undrawnItemNames(plan(item("FixedWingLandingComplexItem"))),
        )
    }

    @Test
    fun `a build whose bridge does not send the class stays quiet rather than warning wrongly`() {
        val older = JSONArray().put(JSONObject().put("isSimpleItem", false))

        assertEquals(emptyList<String>(), undrawnItemNames(older))
    }
}
