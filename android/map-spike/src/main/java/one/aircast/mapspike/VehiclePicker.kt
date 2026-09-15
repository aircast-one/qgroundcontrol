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

data class VehicleChoice(
    val id: Int,
    val name: String,
    val state: String,
    val link: String,
    val contactLost: Boolean,
    val active: Boolean,
)

data class VehicleChoices(
    val ambiguous: Boolean,
    val choices: List<VehicleChoice>,
) {
    val active: VehicleChoice? = choices.firstOrNull { it.active }
}

fun vehicleChoices(view: JSONObject?): VehicleChoices {
    val listed = view?.optJSONArray("vehicles")
    return VehicleChoices(
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

fun vehicleChoiceLine(choice: VehicleChoice): String = when {
    choice.contactLost -> "No contact · ${choice.link}"
    else -> listOfNotNull(choice.state.ifBlank { null }, choice.link.ifBlank { null }).joinToString(" · ")
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

