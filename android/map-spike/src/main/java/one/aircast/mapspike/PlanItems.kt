package one.aircast.mapspike


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


internal fun altitudeWithFrame(item: MissionItem): String? {
    val height = item.altitudeText.ifBlank { null }
        ?: item.altitudeBandText.ifBlank { null }
        ?: return null
    val frame = item.altitudeFrameText
    return if (frame.isBlank()) height else "$height $frame"
}

internal fun altitudeFieldLabel(item: MissionItem): String {
    val unit = item.altitudeEditUnits.ifBlank { "m" }
    val frame = item.altitudeFrameText
    return if (frame.isBlank()) "Alt $unit" else "Alt $unit $frame"
}

internal fun sequenceLabel(item: MissionItem): String = when {
    item.foldedCommands > 0 -> "${item.sequence}\u2013${item.sequence + item.foldedCommands}"
    else -> item.sequence.toString()
}

fun itemRows(
    items: List<MissionItem>,
    stats: Map<Int, SurveyStats> = emptyMap(),
): List<ItemRow> = items.map { item ->
    ItemRow(
        index = item.index,
        number = sequenceLabel(item),
        name = item.command.ifBlank { "Item ${item.sequence}" },
        detail = itemDetail(item, stats[item.index]),
        colour = waypointColour(item.kind, item.commandId),
        placed = item.placed,
    )
}

internal fun itemDetail(item: MissionItem, stats: SurveyStats? = null): String = listOfNotNull(
    altitudeWithFrame(item)
        ?: NO_POSITION.takeIf { !item.placed && item.specifiesCoordinate },
    AFTER_THE_ROUTE_ENDS.takeIf { item.afterRouteEnds },
    item.speedChangeText.ifBlank { null },
    stats?.areaText?.ifBlank { null },
    photosText(item.cameraShots),
    holdText(item.extraSeconds),
    item.blockedReason.ifBlank { null },
    stats?.warning?.ifBlank { null },
).joinToString(" \u00b7 ")

fun worthListing(items: List<MissionItem>): Boolean = items.any { it.index != HOME_ITEM }

fun rowAt(rows: List<ItemRow>, index: Int): ItemRow? = rows.firstOrNull { it.index == index }

fun insertAfter(selected: MapHit?, items: List<MissionItem>): Int {
    val index = (selected as? MapHit.Waypoint)?.index ?: return AT_END
    if (items.none { it.index == index }) return AT_END
    return if (index + 1 >= items.size) AT_END else index + 1
}

fun selectionSequence(selected: MapHit?, items: List<MissionItem>): Int? {
    val index = (selected as? MapHit.Waypoint)?.index ?: return null
    return items.firstOrNull { it.index == index }?.sequence
}

fun addingAfterText(selected: MapHit?, items: List<MissionItem>): String? {
    val index = (selected as? MapHit.Waypoint)?.index ?: return null
    val item = items.firstOrNull { it.index == index } ?: return null
    return "Adding after #${item.sequence}"
}

fun legText(item: MissionItem): String? {
    val travelled = !item.distance.isNaN() && item.distance > 0.0
    val climbed = !item.altitudeChange.isNaN() && item.altitudeChange != 0.0
    return listOf(
        item.distanceText.takeIf { travelled }.orEmpty(),
        item.azimuthText.takeIf { travelled }.orEmpty(),
        item.altitudeChangeText.takeIf { climbed }.orEmpty(),
    )
        .filter { it.isNotBlank() }
        .takeIf { it.isNotEmpty() }
        ?.joinToString(" \u00b7 ")
}

fun movedText(hit: MapHit, items: List<MissionItem>): String = when (hit) {
    is MapHit.Waypoint ->
        items.firstOrNull { it.index == hit.index }?.let { "Moved #${it.sequence}" } ?: "Moved an item"
    is MapHit.FenceVertex -> "Moved a fence corner"
    is MapHit.SurveyVertex -> "Moved a survey corner"
    is MapHit.Rally -> "Moved a rally point"
    is MapHit.CircleCentre -> "Moved a fence circle"
    is MapHit.Circle -> "Changed a fence radius"
    is MapHit.Midpoint -> "Added a corner"
    is MapHit.LandingPlace -> when (hit.place) {
        LANDING_PLACE_APPROACH -> "Moved the final approach"
        else -> "Moved the touchdown"
    }
}

fun writeMove(
    hit: MapHit,
    latitude: Double,
    longitude: Double,
    surveys: List<Survey>,
    rally: List<RallyPoint> = emptyList(),
): Boolean =
    when (hit) {
        is MapHit.Waypoint -> PlanBridge.moveItem(hit.index, latitude, longitude)
        is MapHit.FenceVertex -> FenceBridge.adjustVertex(hit.polygon, hit.vertex, latitude, longitude)
        is MapHit.SurveyVertex -> surveys.firstOrNull { it.index == hit.item }
            ?.let { SurveyBridge.adjustVertex(it, hit.vertex, latitude, longitude) } == true
        is MapHit.Rally -> FenceBridge.moveRallyPoint(
            hit.index,
            latitude,
            longitude,
            rallyAltitudeFor(rally, hit.index),
        )
        is MapHit.CircleCentre -> FenceBridge.moveCircle(hit.index, latitude, longitude)
        is MapHit.Circle -> true
        is MapHit.Midpoint -> false
        is MapHit.LandingPlace ->
            moveLandingPlace(hit.index, hit.place, latitude, longitude)
    }

internal fun patternName(index: Int, items: List<MissionItem>): String =
    items.firstOrNull { it.index == index }?.command?.ifBlank { null } ?: "pattern"

internal fun layersText(survey: Survey?): String? {
    val count = survey?.layers?.takeIf { it > 1 } ?: return null
    val span = survey.layerSpanText
    return if (span.isBlank()) "$count layers, one drawn" else "$count layers, $span, one drawn"
}

internal fun cameraText(stats: SurveyStats?): String? = listOfNotNull(
    stats?.surfaceDistanceText?.ifBlank { null }?.let { "$it above the surface" },
    stats?.footprintText?.ifBlank { null }?.let { "each shot covers $it" },
    stats?.intervalText?.ifBlank { null }?.let { "a shot every $it" },
).takeIf { it.isNotEmpty() }?.joinToString(" \u00b7 ")

internal fun photosText(shots: Int): String? = when {
    shots <= 0 -> null
    shots == 1 -> "1 photo"
    else -> "$shots photos"
}

internal fun holdText(seconds: Double): String? = when {
    seconds.isNaN() || seconds <= 0.0 -> null
    else -> "holds ${seconds.toInt()} s"
}
