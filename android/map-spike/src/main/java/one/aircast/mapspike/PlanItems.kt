package one.aircast.mapspike

import kotlin.concurrent.withLock

const val NO_POSITION = "no position"
const val AFTER_THE_ROUTE_ENDS = "after the route ends"

fun itemCountText(count: Int): String = if (count == 1) "1 item" else "$count items"

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

internal fun rallyAltitudeLabel(point: RallyPoint): String =
    "Alt ${point.altitudeUnits.ifBlank { "m" }}"

internal fun rallyAltitudeIsEditable(point: RallyPoint): Boolean =
    !point.altitude.isNaN() && point.altitudePath.isNotBlank()

internal fun sequenceLabel(item: MissionItem): String = when {
    item.foldedCommands > 0 -> "${item.sequence}\u2013${item.sequence + item.foldedCommands}"
    else -> item.sequence.toString()
}

internal fun itemPlace(item: MissionItem, items: List<MissionItem>): String? {
    val listed = items.filter { it.index != HOME_ITEM }
    val at = listed.indexOfFirst { it.index == item.index }.takeIf { it >= 0 } ?: return null
    val previous = items.lastOrNull { it.index < item.index }
    val leg = item.distanceText.takeIf { previous != null && !item.distance.isNaN() && item.distance > 0.0 && it.isNotBlank() }
        ?.let { "$it from item ${sequenceLabel(previous!!)}" }
    return listOfNotNull("Item ${at + 1} of ${listed.size}", leg).joinToString(" \u00b7 ")
}

internal fun surveyTiles(item: MissionItem, stats: SurveyStats?): List<Pair<String, String>> =
    if (stats == null) emptyList() else listOf(
        "AREA" to stats.areaText,
        "DISTANCE" to stats.distanceText.takeIf { item.kind != KIND_STRUCTURE }.orEmpty(),
        "PHOTOS" to item.cameraShots.takeIf { it > 0 }?.toString().orEmpty(),
        "INTERVAL" to stats.intervalText,
    ).filter { it.second.isNotBlank() }

internal fun sheetDetail(item: MissionItem, stats: SurveyStats?): String =
    if (surveyTiles(item, stats).isEmpty()) itemDetail(item, stats)
    else itemDetail(item.copy(cameraShots = 0), stats?.copy(areaText = ""))

internal fun deleteLabel(item: MissionItem): String = "Delete ${item.command.ifBlank { "item" }.lowercase()}"

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

fun selectionAfterRemove(removed: Int, countBefore: Int): MapHit? =
    (countBefore - 2).takeIf { it > HOME_ITEM }?.let { last -> MapHit.Waypoint(minOf(removed, last)) }

fun missionItemIndex(selected: MapHit?): Int? = when (selected) {
    is MapHit.Waypoint -> selected.index
    is MapHit.SurveyVertex -> selected.item
    is MapHit.LandingPlace -> selected.index
    is MapHit.LoiterRadius -> selected.index
    is MapHit.LoiterRotation -> selected.index
    is MapHit.ShapeCentre -> selected.owner.takeIf { !selected.fence }
    is MapHit.ShapeRadius -> selected.owner.takeIf { !selected.fence }
    else -> null
}

fun insertAfter(selected: MapHit?, items: List<MissionItem>): Int {
    val index = missionItemIndex(selected) ?: return AT_END
    if (items.none { it.index == index }) return AT_END
    return if (index + 1 >= items.size) AT_END else index + 1
}

fun selectionSequence(selected: MapHit?, items: List<MissionItem>): Int? {
    val index = missionItemIndex(selected) ?: return null
    return items.firstOrNull { it.index == index }?.sequence
}

fun addingAfterText(selected: MapHit?, items: List<MissionItem>): String? {
    val index = missionItemIndex(selected) ?: return null
    val item = items.firstOrNull { it.index == index } ?: return null
    return "Adding after #${item.sequence}"
}

fun legText(item: MissionItem): String? {
    val travelled = !item.distance.isNaN() && item.distance > 0.0
    val climbed = !item.altitudeChange.isNaN() && item.altitudeChange != 0.0
    return listOf(
        "Alt diff" to item.altitudeChangeText.takeIf { climbed },
        "Azimuth" to item.azimuthText.takeIf { travelled },
        "Heading" to item.headingText,
        "Gradient" to item.gradientText.takeIf { travelled && climbed },
        "Prev WP" to item.distanceText.takeIf { travelled },
    )
        .filter { (_, value) -> !value.isNullOrBlank() }
        .takeIf { it.isNotEmpty() }
        ?.joinToString(" \u00b7 ") { (label, value) -> "$label $value" }
}

fun movedText(hit: MapHit, items: List<MissionItem>): String = when (hit) {
    is MapHit.Waypoint ->
        items.firstOrNull { it.index == hit.index }?.let { "Moved #${it.sequence}" } ?: "Moved an item"
    is MapHit.FenceVertex -> "Moved a fence corner"
    is MapHit.SurveyVertex -> "Moved a survey corner"
    is MapHit.Rally -> "Moved a rally point"
    MapHit.BreachReturn -> "Moved the breach return point"
    is MapHit.CircleCentre -> "Moved a fence circle"
    is MapHit.CircleRadius -> "Changed a fence radius"
    is MapHit.ShapeCentre -> if (hit.fence) "Moved a fence" else "Moved a survey area"
    is MapHit.ShapeRadius -> "Changed the circle radius"
    is MapHit.LoiterRadius -> "Changed the loiter radius"
    is MapHit.LoiterRotation -> "Changed the loiter direction"
    is MapHit.Circle -> "Changed a fence radius"
    is MapHit.Midpoint -> "Added a corner"
    is MapHit.LandingPlace -> when (hit.place) {
        LANDING_PLACE_APPROACH -> "Moved the final approach"
        else -> "Moved the touchdown"
    }
}

private val moveWrites = java.util.concurrent.locks.ReentrantLock()
private val committedMoves = java.util.concurrent.atomic.AtomicLong()

fun moveGeneration(): Long = committedMoves.get()

fun writeDragStep(
    generation: Long,
    hit: MapHit,
    latitude: Double,
    longitude: Double,
    surveys: List<Survey>,
    rally: List<RallyPoint>,
    fences: List<FencePolygon>,
    items: List<MissionItem>,
    circles: List<FenceCircle> = emptyList(),
): Boolean = !moveWrites.tryLock() || try {
    generation != committedMoves.get() || applyMove(hit, latitude, longitude, surveys, rally, fences, items, circles)
} finally {
    moveWrites.unlock()
}

fun writeMove(
    hit: MapHit,
    latitude: Double,
    longitude: Double,
    surveys: List<Survey>,
    rally: List<RallyPoint>,
    fences: List<FencePolygon> = emptyList(),
    items: List<MissionItem> = emptyList(),
    circles: List<FenceCircle> = emptyList(),
): Boolean = moveWrites.withLock {
    committedMoves.incrementAndGet()
    applyMove(hit, latitude, longitude, surveys, rally, fences, items, circles)
}

private fun applyMove(
    hit: MapHit,
    latitude: Double,
    longitude: Double,
    surveys: List<Survey>,
    rally: List<RallyPoint>,
    fences: List<FencePolygon>,
    items: List<MissionItem>,
    circles: List<FenceCircle>,
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
        is MapHit.CircleRadius -> circles.firstOrNull { it.index == hit.index }
            ?.let { FenceBridge.setCircleRadius(it.index, draggedCircleRadius(it, TrackPoint(latitude, longitude))) } == true
        is MapHit.ShapeCentre -> {
            val target = if (hit.fence) shapeTarget(hit.owner, null) else shapeTarget(null, surveys.firstOrNull { it.index == hit.owner })
            val vertices = if (hit.fence) fences.firstOrNull { it.index == hit.owner }?.vertices else surveys.firstOrNull { it.index == hit.owner }?.area
            val moved = vertices?.let { shapeMovedTo(it, TrackPoint(latitude, longitude)) }
            target != null && moved != null && replaceShape(target, moved)
        }
        is MapHit.ShapeRadius -> {
            val target = if (hit.fence) shapeTarget(hit.owner, null) else shapeTarget(null, surveys.firstOrNull { it.index == hit.owner })
            val circle = target?.let { shapeVertices(it, fences, surveys) }
                ?.let { current -> polygonCentre(current)?.let { centre -> circleAround(current, metresBetween(centre, TrackPoint(latitude, longitude))) } }
            target != null && circle != null && replaceShape(target, circle)
        }
        is MapHit.LoiterRadius -> items.firstOrNull { it.index == hit.index }?.let { item ->
            PlanBridge.setLoiterRadius(item.index, draggedLoiterRadius(item, TrackPoint(latitude, longitude)))
        } == true
        is MapHit.LoiterRotation -> false
        MapHit.BreachReturn -> FenceBridge.setBreachReturn(TrackPoint(latitude, longitude))
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
