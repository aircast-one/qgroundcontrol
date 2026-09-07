package one.aircast.mapspike

// A selection names an index, and indices only mean anything against the plan
// they came from. Loading from the vehicle replaces the plan wholesale, so a
// selection made before it points at whatever now happens to sit at that index,
// and the buttons act on that instead. Deletes clear it themselves; this covers
// every other way the plan can change underneath a selection.
fun selectionSurvives(
    selected: MapHit?,
    items: List<MissionItem>,
    polygons: List<FencePolygon>,
    circles: List<FenceCircle>,
    rally: List<RallyPoint>,
    surveys: List<Survey>,
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
}

// Rotate used to act on the first survey in the plan whatever was selected, so
// with two surveys it turned the wrong grid. Every other control on that row
// follows the selection and this one has to as well.
fun selectedSurvey(selected: MapHit?, surveys: List<Survey>): Survey? =
    (selected as? MapHit.SurveyVertex)?.let { hit -> surveys.firstOrNull { it.index == hit.item } }
