import SwiftUI

let OFFLINE_MAPS_GROUP = "offlineMapsSettings"
let OFFLINE_MAPS_VIEW = "view.offlineMaps"
let OFFLINE_START = "offlineMaps.startDownload"
let OFFLINE_RESUME = "offlineMaps.resume"
let OFFLINE_CANCEL = "offlineMaps.cancel"
let OFFLINE_DELETE = "offlineMaps.delete"
let OFFLINE_RENAME = "offlineMaps.rename"

func renameWanted(_ current: String, _ typed: String) -> String? {
    let trimmed = typed.trimmed
    return !trimmed.isEmpty && trimmed != current ? trimmed : nil
}

private let OFFLINE_POLL_MS = 1000
private let NO_TILE_CACHE = "There is no map tile cache on this device."
private let MIN_ZOOM_PATH = "settings.offlineMapsSettings.minZoomLevelDownload"
private let MAX_ZOOM_PATH = "settings.offlineMapsSettings.maxZoomLevelDownload"
private let DEFAULT_MIN_ZOOM = 13
private let DEFAULT_MAX_ZOOM = 19
private let SLIDER_MIN_ZOOM = 1.0
private let SLIDER_MAX_ZOOM = 20.0

struct OfflineSet: Equatable, Identifiable {
    let id: Int64
    let name: String
    let mapType: String
    let defaultSet: Bool
    let zoomText: String
    let totalText: String
    let uniqueText: String
    var uniqueCount: Int64 = 0
    let downloadedText: String
    let sizeText: String
    let tileCountText: String
    let errorCount: Int
    let errorCountText: String
    let downloadStatus: String
    let downloading: Bool
    let complete: Bool
    var subtitle: String = ""
    var rowText: String = ""
    var canDelete: Bool = true
}

struct OfflineEstimate: Equatable {
    let tileCountText: String
    let tileSizeText: String
    let tooMany: Bool
}

struct OfflineMaps: Equatable {
    let available: Bool
    let reason: String
    let sets: [OfflineSet]
    let mapList: [String]
    let uniqueName: String
    let takenNames: [String]
    let estimate: OfflineEstimate?
}

struct OfflineRegion: Equatable, Hashable {
    let west: Double
    let north: Double
    let east: Double
    let south: Double
}

func offlineMaps(_ view: JSON?) -> OfflineMaps? {
    guard let view, view["class"].string == "OfflineMaps" else { return nil }
    let estimate = view["estimate"]
    return OfflineMaps(
        available: view["available"].bool,
        reason: view["reason"].string,
        sets: view["sets"].objects.map { set in
            OfflineSet(
                id: set["id"].int64 ?? 0,
                name: set["name"].string,
                mapType: set["mapTypeStr"].string,
                defaultSet: set["defaultSet"].bool,
                zoomText: set["zoomText"].string,
                totalText: set["totalText"].string,
                uniqueText: set["uniqueText"].string,
                uniqueCount: set["uniqueCount"].int64 ?? 0,
                downloadedText: set["downloadedText"].string,
                sizeText: set["sizeText"].string,
                tileCountText: set["tileCountText"].string,
                errorCount: set["errorCount"].int(0),
                errorCountText: set["errorCountText"].string,
                downloadStatus: set["downloadStatus"].string,
                downloading: set["downloading"].bool,
                complete: set["complete"].bool,
                subtitle: set["subtitle"].string,
                rowText: set["rowText"].string,
                canDelete: set["canDelete"].bool
            )
        },
        mapList: view["mapList"].strings,
        uniqueName: view["uniqueName"].string,
        takenNames: view["takenNames"].strings,
        estimate: estimate.object == nil ? nil : OfflineEstimate(
            tileCountText: estimate["tileCountText"].string,
            tileSizeText: estimate["tileSizeText"].string,
            tooMany: estimate["tooMany"].bool
        )
    )
}

func offlineRegion(_ corners: [TrackPoint]) -> OfflineRegion? {
    guard !corners.isEmpty else { return nil }
    let longitudes = corners.map(\.longitude)
    let latitudes = corners.map(\.latitude)
    return OfflineRegion(
        west: longitudes.min() ?? 0,
        north: latitudes.max() ?? 0,
        east: longitudes.max() ?? 0,
        south: latitudes.min() ?? 0
    )
}

private func coordinate(_ value: Double) -> String { String(format: "%.7f", value) }

func regionCentre(_ region: OfflineRegion) -> TrackPoint {
    TrackPoint(latitude: (region.north + region.south) / 2.0, longitude: (region.west + region.east) / 2.0)
}

func offlineMapsPath(_ mapType: String?, _ region: OfflineRegion?, _ minZoom: Int, _ maxZoom: Int, _ fetchElevation: Bool = true) -> String {
    guard let mapType, let region else { return OFFLINE_MAPS_VIEW }
    return "\(OFFLINE_MAPS_VIEW)(\(mapType),\(coordinate(region.west)),\(coordinate(region.north)),\(coordinate(region.east)),\(coordinate(region.south)),\(minZoom),\(maxZoom),\(fetchElevation))"
}

private func zoomSetting(_ path: String, _ fallback: Int) -> Int {
    Qgc.get(path)["value"].int ?? fallback
}

private func act(_ path: String, _ args: Any?...) async -> String? {
    await offMain { Qgc.refusalOf(path, arguments: args) }
}

struct OfflineMapsSection: View {
    var heading: String = ""
    @State private var maps: OfflineMaps?
    @State private var polls = 0
    @State private var shown: OfflineSet?
    @State private var adding = false
    @State private var refusal: String?
    @State private var scope = ViewScope()
    @Environment(\.theme) private var theme
    @Environment(\.LocalNativeList) private var nativeList

    var body: some View {
        if nativeList {
            native
        } else {
            dialogs(
                VStack(alignment: .leading, spacing: 0) {
                    if let read = maps {
                        if !read.available {
                            FootNote(text: read.reason.ifBlank(NO_TILE_CACHE))
                        } else {
                            sets(read)
                        }
                    }
                }
            )
        }
    }

    // Inside a NativeForm: a section with a row per tile set (opening its dialog) and Add new set — the row that is
    // always there carries the poll and the dialogs — then import and export in a section of their own.
    @ViewBuilder private var native: some View {
        let available = maps?.available == true
        Section {
            if let read = maps, available {
                ForEach(read.sets) { set in
                    Button { shown = set } label: {
                        HStack(spacing: Space.s2) {
                            Text(set.name).foregroundStyle(Color.primary).frame(maxWidth: .infinity, alignment: .leading)
                            if set.downloading { ProgressView().controlSize(.small) }
                            if !set.rowText.isEmpty {
                                Text(set.rowText).foregroundStyle(Color.secondary)
                                Circle()
                                    .fill(set.complete ? theme.aircast.success : theme.aircast.alert)
                                    .frame(width: 8, height: 8)
                                    .accessibilityHidden(true)
                            }
                            Image(systemName: "chevron.right")
                                .font(.footnote.weight(.semibold))
                                .foregroundStyle(Color.secondary.opacity(0.6))
                                .accessibilityHidden(true)
                        }
                        .contentShape(Rectangle())
                    }
                    .listRowButton()
                }
            }
            dialogs(
                Group {
                    if available {
                        Button("Add new set…") { adding = true }
                    } else {
                        Text(maps.map { $0.reason.ifBlank(NO_TILE_CACHE) } ?? "Reading…").foregroundStyle(Color.secondary)
                    }
                }
            )
            if let refusal {
                Text(refusal).foregroundStyle(theme.aircast.alert)
            }
        } header: {
            if !heading.isBlank { SectionHeader(text: heading) }
        }
        if let read = maps, available {
            Section {
                TileSetTransfer(sets: read.sets) {
                    refusal = $0
                    polls += 1
                }
            }
        }
    }

    // The poll and every dialog, on the one view that is always there.
    @MainActor private func dialogs<Content: View>(_ content: Content) -> some View {
        content
        .task(id: polls) {
            while !Task.isCancelled {
                maps = await offMain { offlineMaps(Qgc.get(OFFLINE_MAPS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(OFFLINE_POLL_MS))
            }
        }
        .onDisappear { scope.cancel() }
        .queuedSheet(item: $shown) { picked in
            let current = maps?.sets.first { $0.id == picked.id } ?? picked
            OfflineSetDialog(
                set: current,
                onDismiss: { shown = nil },
                onRename: { name in
                    scope.launch {
                        refusal = await act(OFFLINE_RENAME, current.id, name)
                        polls += 1
                    }
                },
                onAction: { path in
                    scope.launch {
                        refusal = await act(path, current.id)
                        if path == OFFLINE_DELETE { shown = nil }
                        polls += 1
                    }
                }
            )
        }
        .queuedSheet(isPresented: $adding) {
            OfflineSetEditor(
                onDismiss: { adding = false },
                onDownload: { name, mapType, region, minZoom, maxZoom, elevation in
                    scope.launch {
                        refusal = await act(OFFLINE_START, name, mapType, region.west, region.north, region.east, region.south, minZoom, maxZoom, elevation)
                        if refusal == nil { adding = false }
                        polls += 1
                    }
                }
            )
            .presentationDetents([.large])
            .interactiveDismissDisabled()
        }
    }

    private func sets(_ read: OfflineMaps) -> some View {
        VStack(alignment: .leading, spacing: Space.s1) {
            ForEach(read.sets) { set in
                HStack(spacing: Space.s2) {
                    Text(set.name).frame(maxWidth: .infinity, alignment: .leading)
                    if set.downloading { ProgressView().controlSize(.small) }
                    Text(set.rowText).foregroundStyle(theme.colors.onSurfaceVariant)
                    if !set.rowText.isEmpty {
                        Circle()
                            .fill(set.complete ? theme.aircast.success : theme.aircast.alert)
                            .frame(width: 8, height: 8)
                    }
                }
                .padding(.vertical, Space.s2)
                .contentShape(Rectangle())
                .onTapGesture { shown = set }
                Divider()
            }
            Button("Add new set") { adding = true }.buttonStyle(.filled)
            TileSetTransfer(sets: read.sets) {
                refusal = $0
                polls += 1
            }
            if let refusal {
                Text(refusal).foregroundStyle(theme.aircast.alert)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
        .padding(.horizontal, Space.s4)
        .padding(.vertical, Space.s2)
    }
}

private struct InfoLine: View {
    let label: String
    let value: String

    var body: some View {
        LabeledContent(label, value: value)
    }
}

private struct OfflineSetDialog: View {
    let set: OfflineSet
    let onDismiss: () -> Void
    let onRename: (String) -> Void
    let onAction: (String) -> Void
    @State private var confirming = false
    @State private var typedName: String
    @State private var scope = ViewScope()
    @State private var probe = PresenterProbe()

    init(set: OfflineSet, onDismiss: @escaping () -> Void, onRename: @escaping (String) -> Void, onAction: @escaping (String) -> Void) {
        self.set = set
        self.onDismiss = onDismiss
        self.onRename = onRename
        self.onAction = onAction
        _typedName = State(initialValue: set.name)
    }

    var body: some View {
        NavigationStack {
            NativeForm {
                Section {
                    if set.defaultSet {
                        Text(set.subtitle).foregroundStyle(Color.secondary)
                        InfoLine(label: "Size", value: set.sizeText)
                        InfoLine(label: "Tile Count", value: set.tileCountText)
                    } else {
                        FieldRow(label: "Name", value: typedName, autocapitalize: true, onChange: { typedName = $0 })
                        InfoLine(label: "Map Type", value: set.mapType)
                        InfoLine(label: "Zoom Levels", value: set.zoomText)
                        InfoLine(label: "Total", value: set.totalText)
                        if set.uniqueCount > 0 { InfoLine(label: "Unique", value: set.uniqueText) }
                        if !set.complete { InfoLine(label: "Downloaded", value: set.downloadedText) }
                        if !set.complete && set.errorCount > 0 { InfoLine(label: "Error Count", value: set.errorCountText) }
                    }
                }
                if !set.defaultSet && (set.downloading || !set.complete) {
                    Section {
                        if !set.complete && !set.downloading {
                            Button("Resume download") { onAction(OFFLINE_RESUME) }
                        }
                        if set.downloading {
                            Button("Cancel download") { onAction(OFFLINE_CANCEL) }
                        }
                    }
                }
                Section {
                    Button("Delete", role: .destructive) { confirming = true }.disabled(!set.canDelete)
                }
            }
            .navigationTitle(set.name)
            .navigationBarTitleDisplayMode(.inline)
            .toolbar {
                ToolbarItem(placement: .cancellationAction) {
                    Button(set.defaultSet ? "Close" : "Cancel", action: onDismiss)
                }
                if !set.defaultSet {
                    ToolbarItem(placement: .confirmationAction) {
                        Button("Done") {
                            renameWanted(set.name, typedName).map(onRename)
                            onDismiss()
                        }
                        .disabled(typedName.isBlank)
                    }
                }
            }
        }
        .presentationDetents([.medium, .large])
        .nativeSheetFrame()
        .background(PresenterProbeView(probe: probe).allowsHitTesting(false))
        .onDisappear { scope.cancel() }
        .alert("Confirm delete", isPresented: $confirming) {
            Button("Delete", role: .destructive) {
                scope.launch { if await presenterFreed(probe) { onAction(OFFLINE_DELETE) } }
            }
            Button("Cancel", role: .cancel) {}
        } message: {
            Text(set.defaultSet
                ? "This will delete all tiles INCLUDING the tile sets you have created yourself.\n\nIs this really what you want?"
                : "Delete \(set.name) and all its tiles.\n\nIs this really what you want?")
        }
    }
}

private struct OperatorFix {
    let point: TrackPoint
    let heading: Double
}

private struct EditorQuery: Equatable {
    let mapType: String?
    let region: OfflineRegion?
    let minZoom: Int
    let maxZoom: Int
    let fetchElevation: Bool
}

private struct OfflineSetEditor: View {
    let onDismiss: () -> Void
    let onDownload: (String, String, OfflineRegion, Int, Int, Bool) -> Void
    @State private var mapType: String?
    @State private var zooms = Double(DEFAULT_MIN_ZOOM)...Double(DEFAULT_MAX_ZOOM)
    @State private var region: OfflineRegion?
    @State private var name: String?
    @State private var read: OfflineMaps?
    @State private var flightMap = readCamera()
    @State private var operatorFix: OperatorFix?
    @State private var operatorCentre: TrackPoint?
    @State private var showPreview = false
    @State private var fetchElevation = true
    @State private var picked: TrackPoint?
    @State private var picks = 0
    @MapPath(VEHICLES_VIEW) private var fleetJson
    @Environment(\.theme) private var theme

    private var minZoom: Int { Int(zooms.lowerBound) }
    private var maxZoom: Int { Int(zooms.upperBound) }

    var body: some View {
        let chosenName = name ?? ""
        let nameTaken = read?.takenNames.contains(chosenName.trimmed) == true
        let estimate = read?.estimate
        let downloadable = region != nil && mapType != nil && estimate != nil && estimate?.tooMany != true && !nameTaken && !chosenName.isBlank
        NavigationStack {
            VStack(spacing: 0) {
                ZStack(alignment: .topLeading) {
                    if let mapType {
                        VehicleMap(
                            mapStyle: qgcRasterStyle(mapType),
                            follow: false,
                            operator: operatorFix?.point,
                            operatorHeading: operatorFix?.heading ?? .nan,
                            onViewChanged: { corners in region = offlineRegion(corners) },
                            centreRequest: picks > 0 ? 2 + picks : operatorCentre != nil ? 2 : flightMap != nil ? 1 : 0,
                            centreOn: picked ?? operatorCentre ?? flightMap?.centre,
                            centreZoom: picked == nil && operatorCentre == nil ? flightMap?.zoom : nil
                        )
                    } else {
                        theme.colors.surfaceContainer
                    }
                    CenterMenu(
                        launch: vehicleChoices(fleetJson).active?.home,
                        myLocation: operatorFix?.point,
                        onCentre: { point in
                            picked = point
                            picks += 1
                        },
                        launchLabel: "Home"
                    )
                    .padding(Space.s2)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
                NativeForm {
                    form(chosenName: chosenName, nameTaken: nameTaken, estimate: estimate)
                }
                .frame(maxWidth: .infinity, maxHeight: .infinity)
            }
            .navigationTitle("Add new set")
            .navigationBarTitleDisplayMode(.inline)
            .pageCancelAction(action: onDismiss)
            .pagePrimaryAction("Download", enabled: downloadable) {
                if let region, let mapType { onDownload(chosenName.trimmed, mapType, region, minZoom, maxZoom, fetchElevation) }
            }
        }
        .nativeSheetFrame()
        .task {
            let (type, low, high) = await offMain {
                (currentMapType(), zoomSetting(MIN_ZOOM_PATH, DEFAULT_MIN_ZOOM), zoomSetting(MAX_ZOOM_PATH, DEFAULT_MAX_ZOOM))
            }
            mapType = mapType ?? type
            zooms = Double(low)...Double(max(low, high))
        }
        .task(id: region != nil) {
            guard region != nil else { return }
            while !Task.isCancelled {
                let view = await offMain { OperatorBridge.read() }
                let fix = operatorPoint(view).map { OperatorFix(point: $0, heading: operatorHeading(view)) }
                operatorFix = fix
                if operatorCentre == nil { operatorCentre = fix?.point }
                try? await Task.sleep(for: .milliseconds(OFFLINE_POLL_MS))
            }
        }
        .task(id: EditorQuery(mapType: mapType, region: region, minZoom: minZoom, maxZoom: maxZoom, fetchElevation: fetchElevation)) {
            let path = offlineMapsPath(mapType, region, minZoom, maxZoom, fetchElevation)
            let next = await offMain { offlineMaps(Qgc.get(path)) }
            guard !Task.isCancelled else { return }
            read = next
            if name == nil { name = next?.uniqueName }
        }
    }

    @ViewBuilder
    private func form(chosenName: String, nameTaken: Bool, estimate: OfflineEstimate?) -> some View {
        Section {
            FieldRow(
                label: "Name",
                value: chosenName,
                error: nameTaken ? "Tile set with this name already exists" : nil,
                autocapitalize: true,
                onChange: { name = $0 }
            )
            Picker("Map type", selection: $mapType) {
                ForEach(read?.mapList ?? [], id: \.self) { option in
                    Text(option).tag(Optional(option))
                }
            }
        }
        Section {
            LabeledContent("Min zoom", value: "\(minZoom)")
            Slider(
                value: Binding(get: { zooms.lowerBound }, set: { low in zooms = low...max(low, zooms.upperBound) }),
                in: SLIDER_MIN_ZOOM...SLIDER_MAX_ZOOM,
                step: 1,
                onEditingChanged: { editing in if !editing { storeZooms() } }
            )
            .accessibilityLabel("Min zoom")
            LabeledContent("Max zoom", value: "\(maxZoom)")
            Slider(
                value: Binding(get: { zooms.upperBound }, set: { high in zooms = min(zooms.lowerBound, high)...high }),
                in: SLIDER_MIN_ZOOM...SLIDER_MAX_ZOOM,
                step: 1,
                onEditingChanged: { editing in if !editing { storeZooms() } }
            )
            .accessibilityLabel("Max zoom")
            previews
        } header: {
            Text("Zoom levels")
        }
        Section {
            if let estimate {
                InfoLine(label: "Tile Count", value: estimate.tileCountText)
                InfoLine(label: "Est Size", value: estimate.tileSizeText)
                if estimate.tooMany { Text("Too many tiles").foregroundStyle(theme.aircast.alert) }
            }
            Toggle("Fetch elevation data", isOn: $fetchElevation)
        }
    }

    @ViewBuilder
    private var previews: some View {
        let previewAt = region.map(regionCentre)
        if let previewAt, showPreview, let mapType {
            HStack(spacing: Space.s2) {
                ForEach([("Min Zoom: \(minZoom)", minZoom), ("Max Zoom: \(maxZoom)", maxZoom)], id: \.0) { label, zoom in
                    ZStack(alignment: .bottom) {
                        VehicleMap(
                            mapStyle: qgcRasterStyle(mapType),
                            follow: false,
                            centreRequest: zoom + 1,
                            centreOn: previewAt,
                            centreZoom: Double(zoom),
                            gestures: false
                        )
                        Text(label).font(.labelMedium).padding(Space.s1)
                    }
                    .frame(width: 150, height: 150)
                    .contentShape(Rectangle())
                    .onTapGesture { showPreview = false }
                }
            }
        } else {
            Button("Show zoom previews") { showPreview = true }
                .disabled(previewAt == nil)
        }
    }

    private func storeZooms() {
        let low = Int(zooms.lowerBound.rounded())
        let high = Int(zooms.upperBound.rounded())
        offMain {
            _ = Qgc.set(MIN_ZOOM_PATH, low)
            _ = Qgc.set(MAX_ZOOM_PATH, high)
        }
    }
}
