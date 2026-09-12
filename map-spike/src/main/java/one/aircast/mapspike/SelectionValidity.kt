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

internal fun fenceDetail(
    selected: MapHit?,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
): String? = when (selected) {
    is MapHit.FenceVertex -> polygons.firstOrNull { it.index == selected.polygon }
        ?.let { listOf(it.kindText, it.detailText) }
    is MapHit.Circle -> circles.firstOrNull { it.index == selected.index }
        ?.let { listOf(it.kindText, it.detailText) }
    is MapHit.CircleCentre -> circles.firstOrNull { it.index == selected.index }
        ?.let { listOf(it.kindText, it.detailText) }
    else -> null
}?.filter { it.isNotBlank() }?.takeIf { it.isNotEmpty() }?.joinToString(" \u00b7 ")

internal fun selectedLanding(selected: MapHit?, landings: List<LandingPattern>): LandingPattern? = when (selected) {
    is MapHit.LandingPlace -> landings.firstOrNull { it.index == selected.index }
    is MapHit.Waypoint -> landings.firstOrNull { it.index == selected.index }
    else -> null
}

fun selectedSurvey(selected: MapHit?, surveys: List<Survey>): Survey? = when (selected) {
    is MapHit.SurveyVertex -> surveys.firstOrNull { it.index == selected.item }
    is MapHit.Waypoint -> surveys.firstOrNull { it.index == selected.index }
    else -> null
}
