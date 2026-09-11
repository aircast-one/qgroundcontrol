package one.aircast.mapspike

import kotlin.math.roundToInt

const val NO_POSITION = "no position"
const val AFTER_THE_ROUTE_ENDS = "after the route ends"
const val PLAN_ITEMS_HEADING = "Plan items"

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
        detail = itemDetail(item),
        colour = waypointColour(item.kind, item.commandId),
        placed = item.placed,
    )
}

internal fun itemDetail(item: MissionItem): String = listOfNotNull(
    item.altitude.takeIf { !it.isNaN() }?.let { "${it.roundToInt()} m" }
        ?: NO_POSITION.takeIf { !item.placed },
    AFTER_THE_ROUTE_ENDS.takeIf { item.afterRouteEnds },
).joinToString(" \u00b7 ")

fun worthListing(items: List<MissionItem>): Boolean = items.any { it.index != HOME_ITEM }

fun rowAt(rows: List<ItemRow>, index: Int): ItemRow? = rows.firstOrNull { it.index == index }

fun insertAfter(selected: MapHit?, items: List<MissionItem>): Int {
    val index = (selected as? MapHit.Waypoint)?.index ?: return AT_END
    if (items.none { it.index == index }) return AT_END
    return if (index + 1 >= items.size) AT_END else index + 1
}

