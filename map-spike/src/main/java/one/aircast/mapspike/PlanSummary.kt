package one.aircast.mapspike

fun planSummary(
    itemCount: Int,
    shape: List<String>,
    items: List<MissionItem>,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
    rally: List<RallyPoint>,
    surveys: List<Survey>,
    summaryText: String,
    selected: MapHit?,
): String {
    val counts = listOfNotNull(
        itemCount.takeIf { it > 0 }?.let {
            "$it item${if (it == 1) "" else "s"}" +
                shape.takeIf { named -> named.isNotEmpty() }
                    ?.joinToString(", ", " (", ")").orEmpty()
        },
        (polygons.size + circles.size).takeIf { it > 0 }?.let { "$it fence${if (it == 1) "" else "s"}" },
        rally.size.takeIf { it > 0 }?.let { "$it rally" },
        surveys.sumOf { it.transects.size }.takeIf { it > 0 }?.let { "$it scan pts" },
    )
    if (counts.isEmpty()) {
        return "Empty plan · long press to add"
    }

    return (counts + listOfNotNull(
        summaryText.takeIf { it.isNotEmpty() },
        selectionText(selected, items, circles, polygons),
    )).joinToString(" · ")
}

internal fun circleText(circle: FenceCircle): String =
    circle.detailText

private fun selectionText(
    selected: MapHit?,
    items: List<MissionItem>,
    circles: List<FenceCircle>,
    polygons: List<FencePolygon>,
): String? = when (selected) {
    is MapHit.Waypoint ->
        items.firstOrNull { it.index == selected.index }
            ?.let { item -> item.altitudeText.ifBlank { null }?.let { "#${item.sequence} at $it" } }
    is MapHit.Circle -> circles.firstOrNull { it.index == selected.index }?.let(::circleText)
    is MapHit.CircleCentre -> circles.firstOrNull { it.index == selected.index }?.let(::circleText)
    is MapHit.FenceVertex -> polygons.firstOrNull { it.index == selected.polygon }
        ?.let { "corner ${selected.vertex + 1} of ${it.vertices.size}" }
    is MapHit.SurveyVertex -> null
    is MapHit.Midpoint -> null
    is MapHit.LandingPlace -> when (selected.place) {
        LANDING_PLACE_APPROACH -> "final approach"
        else -> "touchdown"
    }
    is MapHit.Rally -> null
    null -> null
}
