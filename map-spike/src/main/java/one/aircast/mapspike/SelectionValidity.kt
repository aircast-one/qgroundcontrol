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

data class SelectedFence(
    val keepsIn: Boolean,
    val kindText: String,
    val detailText: String,
    val flip: () -> Boolean,
)

private fun fenceOf(polygon: FencePolygon) = SelectedFence(
    keepsIn = polygon.inclusion,
    kindText = polygon.kindText,
    detailText = polygon.detailText,
    flip = { FenceBridge.setPolygonInclusion(polygon.index, !polygon.inclusion) },
)

private fun fenceOf(circle: FenceCircle) = SelectedFence(
    keepsIn = circle.inclusion,
    kindText = circle.kindText,
    detailText = circle.detailText,
    flip = { FenceBridge.setCircleInclusion(circle.index, !circle.inclusion) },
)

// The wording and the keep-in flip both need one answer - which fence does this
// hit name - and a circle is named by its edge or its centre, which the map
// reports as two different hits.
internal fun selectedFence(
    selected: MapHit?,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
): SelectedFence? {
    fun circleAt(index: Int) = circles.firstOrNull { it.index == index }?.let(::fenceOf)
    return when (selected) {
        is MapHit.FenceVertex -> polygons.firstOrNull { it.index == selected.polygon }?.let(::fenceOf)
        is MapHit.Circle -> circleAt(selected.index)
        is MapHit.CircleCentre -> circleAt(selected.index)
        else -> null
    }
}

internal fun fenceDetail(
    selected: MapHit?,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
): String? = selectedFence(selected, polygons, circles)
    ?.let { listOf(it.kindText, it.detailText) }
    ?.filter { it.isNotBlank() }
    ?.takeIf { it.isNotEmpty() }
    ?.joinToString(" · ")

internal fun selectedLanding(selected: MapHit?, landings: List<LandingPattern>): LandingPattern? =
    selectedItem(selected)?.let { index -> landings.firstOrNull { it.index == index } }

fun selectedSurvey(selected: MapHit?, surveys: List<Survey>): Survey? =
    selectedItem(selected)?.let { index -> surveys.firstOrNull { it.index == index } }
