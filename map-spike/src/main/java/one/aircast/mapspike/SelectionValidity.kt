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

internal fun selectedItem(selected: MapHit?): Int? = when (selected) {
    is MapHit.Waypoint -> selected.index
    is MapHit.SurveyVertex -> selected.item
    is MapHit.LandingPlace -> selected.index
    else -> null
}

private fun fenceWording(kindText: String, detailText: String): String? =
    listOf(kindText, detailText).filter { it.isNotBlank() }
        .takeIf { it.isNotEmpty() }?.joinToString(" \u00b7 ")

// The selected fence, whichever handle names it, with the flip that turns a
// boundary into a no-fly zone.
internal fun selectedFence(
    selected: MapHit?,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
): Pair<Boolean, () -> Boolean>? = when (selected) {
    is MapHit.FenceVertex -> polygons.firstOrNull { it.index == selected.polygon }
        ?.let { it.inclusion to { FenceBridge.setPolygonInclusion(it.index, !it.inclusion) } }
    is MapHit.Circle -> circles.firstOrNull { it.index == selected.index }
        ?.let { it.inclusion to { FenceBridge.setCircleInclusion(it.index, !it.inclusion) } }
    is MapHit.CircleCentre -> circles.firstOrNull { it.index == selected.index }
        ?.let { it.inclusion to { FenceBridge.setCircleInclusion(it.index, !it.inclusion) } }
    else -> null
}

internal fun fenceDetail(
    selected: MapHit?,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
): String? = when (selected) {
    is MapHit.FenceVertex -> polygons.firstOrNull { it.index == selected.polygon }
        ?.let { fenceWording(it.kindText, it.detailText) }
    is MapHit.Circle -> circles.firstOrNull { it.index == selected.index }
        ?.let { fenceWording(it.kindText, it.detailText) }
    is MapHit.CircleCentre -> circles.firstOrNull { it.index == selected.index }
        ?.let { fenceWording(it.kindText, it.detailText) }
    else -> null
}

internal fun selectedLanding(selected: MapHit?, landings: List<LandingPattern>): LandingPattern? =
    selectedItem(selected)?.let { index -> landings.firstOrNull { it.index == index } }

fun selectedSurvey(selected: MapHit?, surveys: List<Survey>): Survey? =
    selectedItem(selected)?.let { index -> surveys.firstOrNull { it.index == index } }
