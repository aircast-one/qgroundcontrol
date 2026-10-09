package one.aircast.map

internal fun circleText(circle: FenceCircle): String =
    circle.detailText

internal fun selectionText(
    selected: MapHit?,
    items: List<MissionItem>,
    circles: List<FenceCircle>,
    polygons: List<FencePolygon>,
): String? = when (selected) {
    is MapHit.Waypoint ->
        items.firstOrNull { it.index == selected.index }
            ?.let { item -> altitudeWithFrame(item)?.let { "#${item.sequence} at $it" } }
    is MapHit.Circle -> circles.firstOrNull { it.index == selected.index }?.let(::circleText)
    is MapHit.CircleCentre -> circles.firstOrNull { it.index == selected.index }?.let(::circleText)
    is MapHit.CircleRadius -> circles.firstOrNull { it.index == selected.index }?.let(::circleText)
    is MapHit.FenceVertex -> polygons.firstOrNull { it.index == selected.polygon }
        ?.let { "corner ${selected.vertex + 1} of ${it.vertices.size}" }
    is MapHit.SurveyVertex -> null
    is MapHit.Midpoint -> null
    is MapHit.ShapeCentre -> null
    is MapHit.ShapeRadius -> null
    is MapHit.LoiterRadius, is MapHit.LoiterRotation -> null
    is MapHit.LandingPlace -> when (selected.place) {
        LANDING_PLACE_APPROACH -> "final approach"
        else -> "touchdown"
    }
    is MapHit.Rally -> null
    MapHit.BreachReturn -> null
    null -> null
}
