package one.aircast.mapspike

import kotlin.math.roundToInt

const val NO_POSITION = "no position"
const val AFTER_THE_ROUTE_ENDS = "after the route ends"
const val PLAN_ITEMS_HEADING = "Plan items"
const val TAKEOFF_NEEDS_A_PLACE = "Long press the map to set the takeoff location"

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
    when {
        !item.placed -> NO_POSITION
        else -> item.altitude.takeIf { !it.isNaN() }?.let { "${it.roundToInt()} m" }
    },
    AFTER_THE_ROUTE_ENDS.takeIf { item.afterRouteEnds },
).joinToString(" \u00b7 ")

fun worthListing(items: List<MissionItem>): Boolean = items.any { it.index != HOME_ITEM }

fun rowAt(rows: List<ItemRow>, index: Int): ItemRow? = rows.firstOrNull { it.index == index }

sealed interface LongPress {
    data class SetLaunch(val index: Int) : LongPress
    data object AddWaypoint : LongPress
}

fun longPressAction(selected: MapHit?, items: List<MissionItem>): LongPress =
    (selected as? MapHit.Waypoint)
        ?.let { hit -> items.firstOrNull { it.index == hit.index } }
        ?.takeIf { !it.placed && it.kind == KIND_TAKEOFF }
        ?.let { LongPress.SetLaunch(it.index) }
        ?: LongPress.AddWaypoint
