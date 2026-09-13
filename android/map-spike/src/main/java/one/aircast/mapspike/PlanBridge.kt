package one.aircast.mapspike

import org.json.JSONArray
import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

const val PLAN_ROOT = "plan"
const val PLAN_ITEMS = "$PLAN_ROOT.missionController.visualItems"
const val PLAN_VIEW = "view.missionItems(geometry)"

const val KIND_LAND = "land"
const val KIND_TAKEOFF = "takeoff"
const val KIND_SURVEY = "survey"
const val KIND_CORRIDOR = "corridor"
const val KIND_STRUCTURE = "structure"
const val KIND_ROI = "roi"

data class MissionKind(
    val id: String,
    val label: String,
    val enabled: Boolean,
    val disabledReason: String,
)

// view.missionKinds names every kind, says which are patterns, and says whether
// the plan will take one right now and why not. Spelling any of that here would
// be a second opinion that goes stale the day the core adds a fourth pattern.
fun missionKinds(view: JSONObject?): List<MissionKind> {
    val kinds = view?.optJSONArray("kinds") ?: return emptyList()
    return (0 until kinds.length()).mapNotNull { kinds.optJSONObject(it) }.map { kind ->
        MissionKind(
            id = kind.optText("id"),
            label = kind.optText("complexName"),
            enabled = kind.optBoolean("enabled", true),
            disabledReason = kind.optText("disabledReason"),
        )
    }
}

// A pattern is a kind the core gave a complexName to; nothing else distinguishes
// one, and inventing a second test here is how the hardcoded list started.
fun scanPatterns(kinds: List<MissionKind>): List<MissionKind> =
    kinds.filter { it.label.isNotBlank() }

// A kind the core has not spoken about is offered, because withholding a
// control on silence removes something that works - see the fence gate.
fun kindAllows(kinds: List<MissionKind>, id: String): Boolean =
    kinds.firstOrNull { it.id == id }?.enabled ?: true

fun blockedReason(kinds: List<MissionKind>): String? =
    kinds.firstOrNull { !it.enabled && it.disabledReason.isNotBlank() }?.disabledReason


const val MAV_CMD_NAV_RETURN_TO_LAUNCH = 20

internal fun planItems(json: JSONObject?): JSONArray? = json?.optJSONArray("items")

fun planItemCount(json: JSONObject?): Int =
    ((planItems(json)?.length() ?: 0) - 1).coerceAtLeast(0)

fun linksStartToHome(json: JSONObject?): Boolean =
    planItems(json)?.optJSONObject(1)?.optText("kind") == KIND_TAKEOFF

fun planShape(json: JSONObject?): List<String> {
    val items = planItems(json) ?: return emptyList()
    val endsAfter = routeEndsAfter(items)

    val named = (1 until items.length())
        .mapNotNull { index -> items.optJSONObject(index) }
        .mapNotNull { element ->
            when {
                element.optText("kind") == KIND_TAKEOFF -> "takeoff"
                element.optInt("command") == MAV_CMD_NAV_RETURN_TO_LAUNCH -> "RTL"
                else -> null
            }
        }
        .distinct()

    val stranded = (1 until items.length()).count { it > endsAfter }

    return named + listOfNotNull(
        stranded.takeIf { it > 0 }?.let { "$it after the landing" },
    )
}

data class MissionItem(
    val index: Int,
    val sequence: Int,
    val latitude: Double,
    val longitude: Double,
    val command: String,
    val selected: Boolean,
    val altitude: Double = Double.NaN,
    val exit: TrackPoint? = null,
    val routed: Boolean = true,
    val kind: String = "",
    val commandId: Int = 0,
    val placed: Boolean = true,
    val afterRouteEnds: Boolean = false,
    val altitudeText: String = "",
    val specifiesCoordinate: Boolean = false,
    val distance: Double = Double.NaN,
    val distanceText: String = "",
    val azimuthText: String = "",
    val altitudeChange: Double = Double.NaN,
    val altitudeChangeText: String = "",
    val altitudeBandText: String = "",
    val blockedReason: String = "",
    val cameraShots: Int = 0,
    val extraSeconds: Double = 0.0,
    val speedChangeText: String = "",
    val foldedCommands: Int = 0,
    val altitudeFrame: String = "",
    val altitudeFrameText: String = "",
    val altitudeMode: Int = -1,
    val altitudeEditUnits: String = "",
    val complexPattern: Boolean = false,
)

fun routeEndsAfter(items: JSONArray?): Int =
    (0 until (items?.length() ?: 0))
        .firstOrNull { index -> items?.optJSONObject(index)?.optBoolean("endsRoute") == true }
        ?: Int.MAX_VALUE

internal fun placed(element: JSONObject, key: String): TrackPoint? {
    val at = element.optJSONObject(key) ?: return null
    val latitude = at.optDouble("latitude", Double.NaN)
    val longitude = at.optDouble("longitude", Double.NaN)
    return TrackPoint(latitude, longitude).takeIf { isPlottable(latitude, longitude) }
}

fun allMissionItems(json: JSONObject?): List<MissionItem> {
    val items = planItems(json) ?: return emptyList()
    val endsAfter = routeEndsAfter(items)
    return (0 until items.length()).mapNotNull { index ->
        val element = items.optJSONObject(index) ?: return@mapNotNull null
        val at = placed(element, "coordinate")

        MissionItem(
            index = index,
            sequence = element.optInt("sequence", index),
            latitude = at?.latitude ?: Double.NaN,
            longitude = at?.longitude ?: Double.NaN,
            command = element.optText("name"),
            kind = element.optText("kind"),
            commandId = element.optInt("command"),
            selected = element.optBoolean("selected"),
            altitude = element.optDouble("altitude", Double.NaN),
            altitudeText = element.optText("altitudeText"),
            specifiesCoordinate = element.optBoolean("specifiesCoordinate"),
            distance = element.optDouble("distance", Double.NaN),
            distanceText = element.optText("distanceText"),
            azimuthText = element.optText("azimuthText"),
            altitudeChange = element.optDouble("altitudeChange", Double.NaN),
            altitudeChangeText = element.optText("altitudeChangeText"),
            altitudeBandText = element.optText("altitudeBandText"),
            blockedReason = element.optText("blockedReason"),
            cameraShots = element.optInt("cameraShots"),
            extraSeconds = element.optDouble("extraSeconds", 0.0),
            speedChangeText = element.optText("speedChangeText"),
            foldedCommands = element.optInt("foldedCommands"),
            altitudeFrame = element.optText("altitudeFrame"),
            altitudeFrameText = element.optText("altitudeFrameText"),
            altitudeMode = element.optInt("altitudeMode", -1),
            altitudeEditUnits = element.optText("altitudeEditUnits"),
            complexPattern = !element.optBoolean("simple", true),
            routed = element.optBoolean("flownLeg") && index <= endsAfter,
            afterRouteEnds = index > endsAfter,
            placed = at != null,
            exit = placed(element, "exitCoordinate")
                ?.takeIf { at == null || it.latitude != at.latitude || it.longitude != at.longitude },
        )
    }
}

fun missionItems(json: JSONObject?): List<MissionItem> = allMissionItems(json).filter { it.placed }

object PlanBridge {
    fun rawItems(): JSONObject? =
        runCatching { JSONObject(QGCBridge.get(PLAN_VIEW)) }.getOrNull()

    fun loadFromVehicle() = invokeOk("$PLAN_ROOT.loadFromVehicle")

    fun clearPlan() = invokeOk("$PLAN_ROOT.removeAll")

    fun sendToVehicle() = invokeOk("$PLAN_ROOT.sendToVehicle")

    fun setAltitude(index: Int, metres: Double): Boolean =
        setOk("$PLAN_ITEMS.$index.altitude", settingJson("$metres"))

    fun setAltitudeMode(index: Int, raw: Int): Boolean =
        setOk(altitudeModePath(index), settingJson("$raw"))

    fun removeItem(index: Int): Boolean = removeMissionItem(index).ok

    fun selectSequence(sequence: Int): Boolean =
        invokeOk("$MISSION_CONTROLLER.setCurrentPlanViewSeqNum", "[$sequence, true]")

    fun moveItem(index: Int, latitude: Double, longitude: Double): Boolean =
        setOk("$PLAN_ITEMS.$index.coordinate", settingJson(coordinateJson(latitude, longitude)))
}

internal fun takeoffMissing(items: List<MissionItem>): Boolean =
    items.any { it.latitude != 0.0 || it.longitude != 0.0 } &&
        items.none { it.kind == "takeoff" }

fun landingPatterns(items: List<MissionItem>): List<LandingPattern> =
    items.filter { it.kind == KIND_LAND }
        .mapNotNull { item ->
            landingPattern(
                item.index,
                runCatching {
                    JSONObject(QGCBridge.get("view.landingPattern(${item.index})"))
                }.getOrNull(),
            )
        }

fun moveLandingPlace(index: Int, place: Int, latitude: Double, longitude: Double): Boolean {
    val property = when (place) {
        LANDING_PLACE_APPROACH -> "finalApproachCoordinate"
        LANDING_PLACE_TOUCHDOWN -> "landingCoordinate"
        else -> return false
    }
    return setOk("$PLAN_ITEMS.$index.$property", settingJson(coordinateJson(latitude, longitude)))
}

fun placeTakeoff(index: Int, latitude: Double, longitude: Double): Boolean {
    val placed = setOk(
        "$PLAN_ITEMS.$index.launchCoordinate",
        settingJson(coordinateJson(latitude, longitude)),
    )
    return placed && leaveWizardMode(index)
}

fun placeLandingIfUnplaced(index: Int, latitude: Double, longitude: Double): Boolean {
    val view = runCatching { JSONObject(QGCBridge.get("view.landingPattern($index)")) }.getOrNull()
    if (landingPattern(index, view)?.landing != null) {
        return false
    }
    if (!moveLandingPlace(index, LANDING_PLACE_TOUCHDOWN, latitude, longitude)) {
        return false
    }
    return leaveWizardMode(index)
}

fun leaveWizardMode(index: Int): Boolean =
    setOk("$PLAN_ITEMS.$index.wizardMode", settingJson("false"))
