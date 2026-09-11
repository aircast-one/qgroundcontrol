package one.aircast.mapspike

import kotlin.math.roundToInt

const val NO_POSITION = "no position"

data class ItemRow(
    val index: Int,
    val number: String,
    val name: String,
    val detail: String,
    val colour: String,
    val placed: Boolean,
)

fun itemRows(items: List<MissionItem>): List<ItemRow> = items.map { item ->
    ItemRow(
        index = item.index,
        number = item.sequence.toString(),
        name = item.command.ifBlank { "Item ${item.sequence}" },
        detail = when {
            !item.placed -> NO_POSITION
            else -> item.altitude.takeIf { !it.isNaN() }?.let { "${it.roundToInt()} m" }.orEmpty()
        },
        colour = waypointColour(item.kind, item.commandId),
        placed = item.placed,
    )
}

fun rowAt(rows: List<ItemRow>, index: Int): ItemRow? = rows.firstOrNull { it.index == index }
