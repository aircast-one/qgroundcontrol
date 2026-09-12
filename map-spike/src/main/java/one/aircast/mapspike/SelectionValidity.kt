package one.aircast.mapspike

fun selectionSurvives(
    selected: MapHit?,
    items: List<MissionItem>,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
    rally: List<RallyPoint>,
    surveys: List<Survey>,
    landings: List<LandingPattern> = emptyList(),
): Boolean = when (selected) {
    null -> true
    is MapHit.Waypoint -> items.any { it.index == selected.index }
    is MapHit.FenceVertex -> polygons.any {
        it.index == selected.polygon && selected.vertex in it.vertices.indices
    }
    is MapHit.SurveyVertex -> surveys.any {
        it.index == selected.item && selected.vertex in it.area.indices
    }
    is MapHit.Rally -> rally.any { it.index == selected.index }
    is MapHit.Circle -> circles.any { it.index == selected.index }
    is MapHit.CircleCentre -> circles.any { it.index == selected.index }
    is MapHit.Midpoint -> false
    is MapHit.LandingPlace -> landings.any { it.index == selected.index }
}

fun selectedSurvey(selected: MapHit?, surveys: List<Survey>): Survey? = when (selected) {
    is MapHit.SurveyVertex -> surveys.firstOrNull { it.index == selected.item }
    is MapHit.Waypoint -> surveys.firstOrNull { it.index == selected.index }
    else -> null
}
