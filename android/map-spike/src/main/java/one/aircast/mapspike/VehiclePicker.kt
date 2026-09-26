package one.aircast.mapspike

import org.json.JSONObject
import org.mavlink.qgroundcontrol.QGCBridge

object VehicleBridge {
    var lastRefusal: String? = null
        private set

    var lastAsked: Int? = null
        private set

    fun forget() {
        lastAsked = null
    }

    fun askFor(id: Int): Boolean =
        runCatching {
            val answer = JSONObject(QGCBridge.invoke("vehicles.setActive", "[$id]"))
            lastRefusal = answer.optText("reason").takeIf { it.isNotBlank() }
            answer.optBoolean("ok").also { accepted -> if (accepted) lastAsked = id }
        }.onFailure { lastRefusal = "bridge threw: ${it.message}" }.getOrDefault(false)
}

const val VEHICLES_VIEW = "view.vehicles"

const val VEHICLE_MANAGER = "vehicles"

object FleetBridge {
    fun setSelected(id: Int, selected: Boolean): Boolean =
        invokeOk(if (selected) "$VEHICLE_MANAGER.selectVehicle" else "$VEHICLE_MANAGER.deselectVehicle", "[$id]")

    fun selectAll(choices: VehicleChoices): Boolean =
        choices.choices.filter { !it.selected }.map { setSelected(it.id, true) }.all { it }

    fun deselectAll(): Boolean = invokeOk("$VEHICLE_MANAGER.deselectAllVehicles")

    fun command(action: String, confirmed: Set<Int>): Boolean {
        val targets = selectedPaths().filter { (_, id) -> id in confirmed }
        return when {
            targets.isEmpty() -> false
            action == "mvArm" -> targets.map { still(it) { setOk("$it.armed", settingJson("true")) } }.all { it }
            action == "mvDisarm" -> targets.map { still(it) { setOk("$it.armed", settingJson("false")) } }.all { it }
            action == "mvPause" -> targets.map { still(it) { invokeOk("$it.pauseVehicle") } }.all { it }
            action == "mvStartMission" -> targets.filter { (path, _) -> armedAt(path) }
                .map { still(it) { invokeOk("$it.startMission") } }
                .let { sent -> sent.isNotEmpty() && sent.all { it } }
            else -> false
        }
    }

    // selectedVehicles is addressed by position, and a vehicle dropping off renumbers it. The id
    // is read again against the path about to be written, so a command cannot land on a vehicle
    // that moved into the index between the read and the write.
    private fun still(target: Pair<String, Int>, write: (String) -> Boolean): Boolean {
        val (path, id) = target
        return idAt(path) == id && write(path)
    }

    private fun selectedPaths(): List<Pair<String, Int>> {
        val count = runCatching {
            JSONObject(QGCBridge.get("$VEHICLE_MANAGER.selectedVehicles.count")).optInt("value", 0)
        }.getOrDefault(0)
        return (0 until count).mapNotNull { index ->
            val path = "$VEHICLE_MANAGER.selectedVehicles.$index"
            idAt(path)?.let { id -> path to id }
        }
    }

    private fun idAt(path: String): Int? =
        runCatching { JSONObject(QGCBridge.getFields(path, "id")).optInt("id", -1).takeIf { it >= 0 } }.getOrNull()

    private fun armedAt(path: String): Boolean =
        runCatching { JSONObject(QGCBridge.getFields(path, "armed")).optBoolean("armed") }.getOrDefault(false)
}

const val CHOOSER_TITLE = "Fly which aircraft?"

data class VehicleChoice(
    val id: Int,
    val name: String,
    val state: String,
    val link: String,
    val contactLost: Boolean,
    val active: Boolean,
    val latitude: Double = Double.NaN,
    val longitude: Double = Double.NaN,
    val selected: Boolean = false,
    val heading: Double = Double.NaN,
    val home: TrackPoint? = null,
)

data class VehicleChoices(
    val ambiguous: Boolean,
    val choices: List<VehicleChoice>,
    val canSelectAll: Boolean = false,
    val canDeselectAll: Boolean = false,
) {
    val active: VehicleChoice? = choices.firstOrNull { it.active }
    val selectedCount: Int = choices.count { it.selected }
}

fun vehicleChoices(view: JSONObject?): VehicleChoices {
    val listed = view?.optJSONArray("vehicles")
    return VehicleChoices(
        canSelectAll = view?.optBoolean("canSelectAll") == true,
        canDeselectAll = view?.optBoolean("canDeselectAll") == true,
        ambiguous = view?.optBoolean("ambiguous") == true,
        choices = (0 until (listed?.length() ?: 0)).mapNotNull { index ->
            listed!!.optJSONObject(index)?.let { entry ->
                val id = entry.optInt("id", -1).takeIf { it >= 0 } ?: return@mapNotNull null
                VehicleChoice(
                    id = id,
                    name = entry.optText("name").ifBlank { "Vehicle $id" },
                    state = vehicleChoiceState(entry),
                    link = entry.optText("link"),
                    contactLost = !entry.isNull("contactLost") && entry.optBoolean("contactLost"),
                    active = entry.optBoolean("active"),
                    latitude = entry.optJSONObject("coordinate")?.optDouble("latitude") ?: Double.NaN,
                    longitude = entry.optJSONObject("coordinate")?.optDouble("longitude") ?: Double.NaN,
                    selected = entry.optBoolean("selected"),
                    // view.vehicles serves each aircraft's own heading, null before any attitude,
                    // and its home, null while QGC holds an invalid one.
                    heading = if (entry.isNull("heading")) Double.NaN else entry.optDouble("heading", Double.NaN),
                    home = entry.optJSONObject("home")?.let { at ->
                        TrackPoint(at.optDouble("latitude", Double.NaN), at.optDouble("longitude", Double.NaN))
                            .takeIf { isPlottable(it.latitude, it.longitude) }
                    },
                )
            }
        },
    )
}

private fun vehicleChoiceState(entry: JSONObject): String = listOfNotNull(
    entry.optText("flightMode").ifBlank { null },
    when {
        entry.optBoolean("flying") -> "Flying"
        entry.optBoolean("armed") -> "Armed"
        else -> "Disarmed"
    },
).joinToString(" · ")

fun linkDistinguishes(choices: List<VehicleChoice>): Boolean =
    choices.mapNotNull { it.link.ifBlank { null } }.distinct().size > 1

fun vehicleChoiceLine(choice: VehicleChoice, distinguishes: Boolean = true): String {
    val link = choice.link.takeIf { distinguishes && it.isNotBlank() }
    return when {
        choice.contactLost -> listOfNotNull("No contact", link).joinToString(" · ")
        else -> listOfNotNull(choice.state.ifBlank { null }, link).joinToString(" · ")
    }
}

fun lostVehicles(choices: VehicleChoices): List<VehicleChoice> =
    choices.choices.filter { it.contactLost && !it.active }

fun lostVehiclesText(lost: List<VehicleChoice>): String? = when (lost.size) {
    0 -> null
    1 -> "${lost.single().name} is not answering"
    else -> "${lost.size} other vehicles are not answering"
}

fun activeVehicleTitle(choices: VehicleChoices, subtitle: String): String = when {
    !choices.ambiguous -> subtitle
    else -> listOfNotNull(choices.active?.name, subtitle.ifBlank { null }).joinToString(" · ")
}

fun uploadHeading(gate: UploadGate, choices: VehicleChoices): String {
    val asked = gate.heading.ifBlank { "Upload this plan?" }
    val target = choices.takeIf { it.ambiguous }?.active?.name ?: return asked
    return when {
        asked.endsWith("?") -> asked.dropLast(1) + " to $target?"
        else -> "$asked to $target"
    }
}

fun handoverNotice(before: VehicleChoices?, now: VehicleChoices, asked: Int?): String? {
    val left = before?.active ?: return null
    val arrived = now.active ?: return null
    if (left.id == arrived.id || arrived.id == asked) return null
    return when (now.choices.any { it.id == left.id }) {
        true -> "${left.name} stopped answering. Now flying ${arrived.name}."
        false -> "${left.name} is gone. Now flying ${arrived.name}."
    }
}

fun activeChanged(before: VehicleChoices?, now: VehicleChoices): Boolean {
    val was = before?.active?.id ?: return false
    val isNow = now.active?.id ?: return false
    return was != isNow
}

fun rememberedChoices(previous: VehicleChoices?, now: VehicleChoices): VehicleChoices? = when {
    now.choices.isEmpty() -> null
    now.active != null -> now
    else -> previous
}

