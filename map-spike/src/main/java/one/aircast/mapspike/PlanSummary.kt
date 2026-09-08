package one.aircast.mapspike

fun planSummary(
    itemCount: Int,
    shape: List<String>,
    items: List<MissionItem>,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
    rally: List<RallyPoint>,
    surveys: List<Survey>,
    distanceMetres: Double,
    seconds: Double,
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
        surveys.sumOf { it.transects.size }.takeIf { it > 0 }?.let { "$it survey pts" },
    )
    if (counts.isEmpty()) {
        return "Empty plan · long press to add"
    }

    return (counts + listOfNotNull(
        missionSummary(distanceMetres, seconds).takeIf { it.isNotEmpty() },
        selectionText(selected, items, circles),
    )).joinToString(" · ")
}

private fun selectionText(
    selected: MapHit?,
    items: List<MissionItem>,
    circles: List<FenceCircle>,
): String? = when (selected) {
    is MapHit.Waypoint ->
        items.firstOrNull { it.index == selected.index }
            ?.altitude?.takeIf { !it.isNaN() }
            ?.let { "#${selected.index} at ${it.toInt()} m" }
    is MapHit.Circle -> circles.firstOrNull { it.index == selected.index }
        ?.let { "circle ${it.radius.toInt()} m" }
    is MapHit.CircleCentre -> circles.firstOrNull { it.index == selected.index }
        ?.let { "circle ${it.radius.toInt()} m" }
    else -> null
}
