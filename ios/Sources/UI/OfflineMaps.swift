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
        sets: view["sets"].array.filter { $0.object != nil }.map { set in
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
    Qgc.get(path)["value"].double.flatMap { $0.isFinite ? Int($0) : nil } ?? fallback
}

private func act(_ path: String, _ args: Any?...) async -> String? {
    await offMain { refusal(Qgc.call(path, arguments: args)) }
}

struct OfflineMapsSection: View {
    @State private var maps: OfflineMaps?
    @State private var polls = 0
    @State private var shown: OfflineSet?
    @State private var adding = false
    @State private var refusal: String?
    @Environment(\.theme) private var theme

    var body: some View {
        VStack(alignment: .leading, spacing: 0) {
            if let read = maps {
                if !read.available {
                    FootNote(text: read.reason.ifBlank("There is no map tile cache on this device."))
                } else {
                    sets(read)
                }
            }
        }
        .task(id: polls) {
            while !Task.isCancelled {
                maps = await offMain { offlineMaps(Qgc.get(OFFLINE_MAPS_VIEW)) }
                try? await Task.sleep(for: .milliseconds(OFFLINE_POLL_MS))
            }
        }
        .sheet(item: $shown) { picked in
            let current = maps?.sets.first { $0.id == picked.id } ?? picked
            OfflineSetDialog(
                set: current,
                onDismiss: { shown = nil },
                onRename: { name in
                    Task {
                        refusal = await act(OFFLINE_RENAME, current.id, name)
                        polls += 1
                    }
                },
                onAction: { path in
                    Task {
                        refusal = await act(path, current.id)
                        if path == OFFLINE_DELETE { shown = nil }
                        polls += 1
                    }
                }
            )
        }
        .fullScreenCover(isPresented: $adding) {
            OfflineSetEditor(
                onDismiss: { adding = false },
                onDownload: { name, mapType, region, minZoom, maxZoom, elevation in
                    Task {
                        refusal = await act(OFFLINE_START, name, mapType, region.west, region.north, region.east, region.south, minZoom, maxZoom, elevation)
                        if refusal == nil { adding = false }
                        polls += 1
                    }
                }
            )
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
    @Environment(\.theme) private var theme

    var body: some View {
        HStack(alignment: .top, spacing: Space.s2) {
            Text(label).foregroundStyle(theme.colors.onSurfaceVariant).frame(maxWidth: .infinity, alignment: .leading)
            Text(value).frame(maxWidth: .infinity, alignment: .leading)
        }
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
    @Environment(\.theme) private var theme

    init(set: OfflineSet, onDismiss: @escaping () -> Void, onRename: @escaping (String) -> Void, onAction: @escaping (String) -> Void) {
        self.set = set
        self.onDismiss = onDismiss
        self.onRename = onRename
        self.onAction = onAction
        _typedName = State(initialValue: set.name)
    }

    var body: some View {
        NavigationStack {
            Form {
                Section {
                    if set.defaultSet {
                        Text(set.subtitle).foregroundStyle(theme.colors.onSurfaceVariant)
                        InfoLine(label: "Size:", value: set.sizeText)
                        InfoLine(label: "Tile Count:", value: set.tileCountText)
                    } else {
                        TextField("Name", text: $typedName)
                        Text(set.mapType).foregroundStyle(theme.colors.onSurfaceVariant)
                        InfoLine(label: "Zoom Levels:", value: set.zoomText)
                        InfoLine(label: "Total:", value: set.totalText)
                        if set.uniqueCount > 0 { InfoLine(label: "Unique:", value: set.uniqueText) }
                        if !set.complete { InfoLine(label: "Downloaded:", value: set.downloadedText) }
                        if !set.complete && set.errorCount > 0 { InfoLine(label: "Error Count:", value: set.errorCountText) }
                    }
                }
                Section {
                    if !set.defaultSet && !set.complete && !set.downloading {
                        Button("Resume download") { onAction(OFFLINE_RESUME) }
                    }
                    if !set.defaultSet && set.downloading {
                        Button("Cancel download") { onAction(OFFLINE_CANCEL) }
                    }
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
                        Button("Ok") {
                            renameWanted(set.name, typedName).map(onRename)
                            onDismiss()
                        }
                        .disabled(typedName.isBlank)
                    }
                }
            }
        }
        .presentationDetents([.medium, .large])
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
            ScrollView {
                form(chosenName: chosenName, nameTaken: nameTaken, estimate: estimate)
                    .padding(Space.s4)
            }
            .frame(maxWidth: .infinity, maxHeight: .infinity)
        }
        .background(theme.colors.surface)
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

    private func form(chosenName: String, nameTaken: Bool, estimate: OfflineEstimate?) -> some View {
        VStack(alignment: .leading, spacing: Space.s2) {
            Text("Add new set").font(.titleMedium)
            TextField("Name:", text: Binding(get: { chosenName }, set: { name = $0 }))
                .textFieldStyle(.roundedBorder)
                .overlay {
                    if nameTaken { RoundedRectangle(cornerRadius: 6).stroke(theme.aircast.alert, lineWidth: 1) }
                }
            Menu {
                ForEach(read?.mapList ?? [], id: \.self) { option in
                    Button(option) { mapType = option }
                }
            } label: {
                Text("Map type: \(mapType ?? "")")
            }
            .buttonStyle(.bordered)
            Text("Min/Max Zoom Levels")
            Slider(
                value: Binding(get: { zooms.lowerBound }, set: { low in zooms = low...max(low, zooms.upperBound) }),
                in: SLIDER_MIN_ZOOM...SLIDER_MAX_ZOOM,
                step: 1,
                onEditingChanged: { editing in if !editing { storeZooms() } }
            )
            Slider(
                value: Binding(get: { zooms.upperBound }, set: { high in zooms = min(zooms.lowerBound, high)...high }),
                in: SLIDER_MIN_ZOOM...SLIDER_MAX_ZOOM,
                step: 1,
                onEditingChanged: { editing in if !editing { storeZooms() } }
            )
            HStack(spacing: Space.s4) {
                Text("Min zoom: \(minZoom)")
                Text("Max zoom: \(maxZoom)")
            }
            previews
            if let estimate {
                InfoLine(label: "Tile Count:", value: estimate.tileCountText)
                InfoLine(label: "Est Size:", value: estimate.tileSizeText)
                if estimate.tooMany { Text("Too many tiles").foregroundStyle(theme.aircast.alert) }
            }
            Toggle("Fetch elevation data", isOn: $fetchElevation)
            if nameTaken { Text("Tile set with this name already exists").foregroundStyle(theme.aircast.alert) }
            HStack(spacing: Space.s2) {
                Button("Download") {
                    if let region, let mapType { onDownload(chosenName.trimmed, mapType, region, minZoom, maxZoom, fetchElevation) }
                }
                .buttonStyle(.filled)
                .disabled(region == nil || mapType == nil || estimate == nil || estimate?.tooMany == true || nameTaken || chosenName.isBlank)
                Button("Cancel", action: onDismiss).buttonStyle(.bordered)
            }
        }
        .frame(maxWidth: .infinity, alignment: .leading)
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
                .buttonStyle(.bordered)
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
