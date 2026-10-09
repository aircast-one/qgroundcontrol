import SwiftUI

private let FLY_STORE = "fly"
private let FLY_VIEW_KEY = "view"
private let FLY_SCRIM_ALPHA = 0.55
private let STATUS_ROW_HEIGHT: CGFloat = 32
private let MAP_PIP_KEY = "mapPip"
private let VIDEO_PIP_KEY = "videoPip"
private let MINIMAP_WIDTH: CGFloat = 184
private let MINIMAP_HEIGHT: CGFloat = 112
private let COMPASS_DIAL_KEY = "compassDial"
private let ACTION_RAIL_CLEARANCE: CGFloat = 72
private let STOP_CLEARANCE: CGFloat = 56
private let RAIL_CLEARANCE = STOP_CLEARANCE + Space.s3 * 2
private let TOP_SCRIM_HEIGHT: CGFloat = 96
private let TOP_SCRIM_ALPHA = 0.6
private let PIP_TOGGLE_SIZE: CGFloat = 28
private let MINI_MAP_KEY = "LandscapeMiniMap"
private let MINIMAP_THUMB: CGFloat = 56
private let CAMERA_PIP_KEY = "cameraPip"
private let CAMERA_PIP_WIDTH: CGFloat = 112
private let CAMERA_PIP_HEIGHT: CGFloat = 63
private let FULL_SCREEN_LAYER: Double = 10
private let MAP_LAYERS_BUTTON: CGFloat = 48

private var flyStore: UserDefaults { UserDefaults(suiteName: FLY_STORE) ?? .standard }

extension EnvironmentValues {
    @Entry var LocalRootSize: CGSize = .zero
    @Entry var LocalAvoidedByVideoMessage = false
    @Entry var LocalRailFloor: CGFloat = 0
}

enum MiniMap: String, CaseIterable {
    case Thumb, Map, Compass
}

func miniMapNamed(_ name: String?) -> MiniMap { MiniMap.allCases.first { $0.rawValue == name } ?? .Thumb }

func loadMiniMap() -> MiniMap { miniMapNamed(flyStore.string(forKey: MINI_MAP_KEY)) }

func saveMiniMap(_ mini: MiniMap) { flyStore.set(mini.rawValue, forKey: MINI_MAP_KEY) }

func miniWidth(_ mini: MiniMap) -> CGFloat {
    switch mini {
    case .Thumb: MINIMAP_THUMB
    case .Map: MINIMAP_WIDTH
    case .Compass: MINIMAP_HEIGHT
    }
}

private func miniHeight(_ mini: MiniMap) -> CGFloat { mini == .Thumb ? MINIMAP_THUMB : MINIMAP_HEIGHT }

func videoPipShown(_ hasVideo: Bool, _ expanded: Bool) -> Bool { hasVideo && expanded }

private struct PipToggle: View {
    let expanded: Bool
    let onToggle: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onToggle) {
            Image(expanded ? .arrowDropDown : .arrowUp)
                .font(.system(size: 12, weight: .semibold))
                .foregroundStyle(theme.colors.onSurface)
                .frame(width: PIP_TOGGLE_SIZE, height: PIP_TOGGLE_SIZE)
                .background(theme.colors.surfaceContainerHigh, in: Circle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(expanded ? "Hide picture-in-picture" : "Show picture-in-picture")
    }
}

enum FlyView: String, CaseIterable {
    case Video, Map, ThreeD

    var label: String {
        switch self {
        case .Video: "Video"
        case .Map: "Map"
        case .ThreeD: "3D"
        }
    }

    var icon: Icon {
        switch self {
        case .Video: .videocam
        case .Map: .map
        case .ThreeD: .explore
        }
    }
}

func flyViewNamed(_ name: String?) -> FlyView { FlyView.allCases.first { $0.rawValue == name } ?? .Video }

func flyViewsOffered(_ viewer3dEnabled: Bool?, _ current: FlyView) -> [FlyView] {
    FlyView.allCases.filter { $0 != .ThreeD || viewer3dEnabled == true || (viewer3dEnabled == nil && current == .ThreeD) }
}

func flyViewAllowed(_ viewer3dEnabled: Bool?, _ current: FlyView) -> FlyView {
    current == .ThreeD && viewer3dEnabled == false ? .Map : current
}

func viewer3dEnabled(_ view: JSON?) -> Bool? {
    guard let view, view.has("enabled") else { return nil }
    return view["enabled"].bool
}

func flyViewShown(_ chosen: FlyView, _ armed: Bool, _ noVideoSource: Bool) -> FlyView {
    chosen == .Video && armed && noVideoSource ? .Map : chosen
}

func flyViewSwapped(_ view: FlyView) -> FlyView { view == .Map ? .Video : .Map }

func loadFlyView() -> FlyView { flyViewNamed(flyStore.string(forKey: FLY_VIEW_KEY)) }

func saveFlyView(_ view: FlyView) { flyStore.set(view.rawValue, forKey: FLY_VIEW_KEY) }

@propertyWrapper
struct FlyIsPortrait: DynamicProperty {
    @Environment(\.LocalRootSize) private var root

    init() {}

    var wrappedValue: Bool { root.height > root.width }
}

struct FlyScreen: View {
    let view: FlyView
    let onView: (FlyView) -> Void
    let status: () -> AnyView
    let video: (Bool) -> AnyView
    let map: () -> AnyView
    let keyRow: () -> AnyView
    let rail: (Bool) -> AnyView
    let overlays: () -> AnyView
    let actions: (FlyDeckLayout) -> AnyView
    let fullScreen: Bool
    let onFullScreen: () -> Void
    let onExitFullScreen: () -> Void
    @Environment(\.theme) private var theme
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(AppNavigationState.self) private var navigation
    @QgcPath(VIDEO_VIEW) private var videoJson
    @QgcPath(FLY_STATE) private var flyJson
    @HasVehicle private var hasVehicle
    @CameraStepper private var cameras
    @GimbalDrags private var gimbalDrags
    @ShownPipCamera private var pipCamera
    @State private var mini = loadMiniMap()
    @State private var holding = false
    @State private var layers = false

    private var hasVideo: Bool { videoJson?["available"].bool == true }
    private var pipExpanded: Bool { mini != .Thumb }

    private func showMini(_ next: MiniMap) {
        mini = next
        saveMiniMap(next)
    }

    private func togglePip() { showMini(pipExpanded ? .Thumb : .Map) }

    var body: some View {
        GeometryReader { safeArea in
            GeometryReader { geometry in
                screen(geometry.size, safeArea.safeAreaInsets)
            }
            .ignoresSafeArea()
        }
        .onChange(of: flyState(flyJson)?.armed == true, initial: true) { _, armed in flyScreen.layout.lockWhileArmed(armed) }
    }

    @ViewBuilder
    private func screen(_ box: CGSize, _ safe: EdgeInsets) -> some View {
        let layout = flyScreen.layout
        let mapIsPip = view == .Video
        let videoIsPip = view == .Map
        let mapShown = view == .Map || (mapIsPip && mini != .Compass)
        let videoShown = view != .ThreeD && (view != .Map || videoPipShown(hasVideo, pipExpanded))
        let cutout = EdgeInsets(top: 0, leading: safe.leading, bottom: 0, trailing: safe.trailing)
        let videoKey = orientedKey(VIDEO_PIP_KEY, true)
        let videoMoved = layout.offsets[videoKey] ?? .zero
        let pipHome = CGPoint(x: cutout.leading + Space.s3, y: box.height - cutout.bottom - Space.s3 - MINIMAP_HEIGHT)
        let pipFrame = CGRect(x: pipHome.x + videoMoved.width, y: pipHome.y + videoMoved.height, width: MINIMAP_WIDTH, height: MINIMAP_HEIGHT)
        let pipNow = videoIsPip && !fullScreen
        let videoTarget = pipNow ? pipFrame : CGRect(origin: .zero, size: box)
        let mapPipSide = mini == .Thumb ? CGSize(width: MINIMAP_THUMB, height: MINIMAP_THUMB) : CGSize(width: MINIMAP_WIDTH, height: MINIMAP_HEIGHT)
        let mapPipShape = RoundedRectangle(cornerRadius: Corner.medium)
        ZStack(alignment: .topLeading) {
            if view == .ThreeD {
                Viewer3DPane().frame(width: box.width, height: box.height)
            }
            if mapShown {
                map()
                    .frame(width: mapIsPip ? mapPipSide.width : box.width, height: mapIsPip ? mapPipSide.height : box.height)
                    .clipShape(RoundedRectangle(cornerRadius: mapIsPip ? Corner.medium : 0))
                    .overlay(RoundedRectangle(cornerRadius: mapIsPip ? Corner.medium : 0).stroke(theme.colors.onSurface, lineWidth: mapIsPip ? 2 : 0))
                    .layoutPlacement(MAP_PIP_KEY, keepOnScreen: false, active: mapIsPip)
                    .padding(.leading, mapIsPip ? cutout.leading + Space.s3 : 0)
                    .padding(.bottom, mapIsPip ? Space.s3 : 0)
                    .frame(width: box.width, height: box.height, alignment: mapIsPip ? .bottomLeading : .topLeading)
                    .accessibilityHidden(fullScreen)
                    .zIndex(mapIsPip ? 1 : 0)
            }
            if videoShown {
                video(!pipNow)
                    .background(Color.black)
                    .clipShape(RoundedRectangle(cornerRadius: pipNow ? Corner.medium : 0))
                    .contentShape(Rectangle())
                    .videoGestures(videoHandlers(videoKey, pipNow, pipHome, videoTarget))
                    .videoFrame(videoTarget, holding: holding, nudge: cameras.nudge)
                    .transition(.opacity.combined(with: .move(edge: .bottom)))
                    .zIndex(fullScreen ? FULL_SCREEN_LAYER : videoIsPip ? 1 : 0)
            }
            if !fullScreen {
                chrome(box, safe, cutout, mapIsPip: mapIsPip, videoIsPip: videoIsPip, videoShown: videoShown, mapPipSide: mapPipSide, mapPipShape: mapPipShape, videoKey: videoKey)
            }
        }
        .frame(width: box.width, height: box.height)
        .animation(.default, value: videoShown)
        .background {
            if layers { MapLayersSheet { layers = false } }
        }
    }

    private func videoHandlers(_ videoKey: String, _ pipNow: Bool, _ pipHome: CGPoint, _ target: CGRect) -> VideoGestureHandlers {
        let layout = flyScreen.layout
        let gimbal = gimbalDrags
        let stepper = cameras
        let expanded = pipExpanded
        let full = fullScreen
        return VideoGestureHandlers(
            owned: { false },
            claimsSwipe: { moved in sideways(moved) && !gimbal },
            onTap: {},
            onDoubleTap: { full ? onExitFullScreen() : onFullScreen() },
            onSwipe: { moved in cameraStep(videoSwipe(moved, VIDEO_SWIPE_DISTANCE)).map(stepper.step) },
            onHold: { at in
                guard !pipNow else { return false }
                holding = true
                let wanted = CGPoint(x: target.minX + at.x - MINIMAP_WIDTH / 2 - pipHome.x, y: target.minY + at.y - MINIMAP_HEIGHT / 2 - pipHome.y)
                let current = layout.offsets[videoKey] ?? .zero
                layout.nudge(videoKey, wanted.x - current.width, wanted.y - current.height)
                if !expanded { showMini(.Map) }
                onExitFullScreen()
                onView(.Map)
                return true
            },
            onHoldDrag: { delta in layout.nudge(videoKey, delta.width, delta.height) },
            onHoldEnd: {
                layout.saveOffset(videoKey)
                holding = false
            }
        )
    }

    private func pipHandlers(_ videoKey: String) -> VideoGestureHandlers {
        let layout = flyScreen.layout
        let stepper = cameras
        return VideoGestureHandlers(
            owned: { !layout.editing },
            claimsSwipe: { _ in false },
            onTap: { onView(.Video) },
            onDoubleTap: { onFullScreen() },
            onSwipe: { moved in
                switch videoSwipe(moved, VIDEO_SWIPE_DISTANCE) {
                case .Down: togglePip()
                case .Up: onView(.Video)
                case let swipe: cameraStep(swipe).map(stepper.step)
                }
            },
            onHold: { _ in
                guard !layout.editing else { return false }
                holding = true
                return true
            },
            onHoldDrag: { delta in layout.nudge(videoKey, delta.width, delta.height) },
            onHoldEnd: {
                layout.saveOffset(videoKey)
                holding = false
            }
        )
    }

    @ViewBuilder
    private func chrome(_ box: CGSize, _ safe: EdgeInsets, _ cutout: EdgeInsets, mapIsPip: Bool, videoIsPip: Bool, videoShown: Bool, mapPipSide: CGSize, mapPipShape: RoundedRectangle, videoKey: String) -> some View {
        let bottomStart = { (content: AnyView) in
            content
                .padding(.leading, cutout.leading + Space.s3)
                .padding(.bottom, Space.s3)
                .frame(width: box.width, height: box.height, alignment: .bottomLeading)
        }
        LinearGradient(colors: [Color.black.opacity(TOP_SCRIM_ALPHA), .clear], startPoint: .top, endPoint: .bottom)
            .frame(width: box.width, height: TOP_SCRIM_HEIGHT)
            .allowsHitTesting(false)
        if mapIsPip {
            if mini == .Thumb {
                bottomStart(AnyView(
                    Color.clear
                        .contentShape(mapPipShape)
                        .frame(width: mapPipSide.width, height: mapPipSide.height)
                        .layoutPlacement(MAP_PIP_KEY, keepOnScreen: false)
                        .onTapGesture { showMini(.Map) }
                        .accessibilityElement()
                        .accessibilityLabel("Map")
                        .accessibilityAddTraits(.isButton)
                        .accessibilityAction(named: "Show the mini-map") { showMini(.Map) }
                ))
                .zIndex(2)
            }
            if mini == .Compass {
                bottomStart(AnyView(
                    OsdCompassDial(size: MINIMAP_HEIGHT)
                        .frame(width: MINIMAP_HEIGHT, height: MINIMAP_HEIGHT)
                        .avoidedByVideoMessage(COMPASS_DIAL_KEY)
                        .clipShape(Circle())
                        .contentShape(Circle())
                        .onTapGesture { showMini(.Map) }
                        .accessibilityElement()
                        .accessibilityLabel("Compass")
                        .accessibilityAddTraits(.isButton)
                        .accessibilityAction(named: "Show the map") { showMini(.Map) }
                ))
                .zIndex(3)
            }
            if mini == .Map {
                bottomStart(AnyView(
                    ZStack(alignment: .topLeading) {
                        Color.clear.contentShape(mapPipShape).onTapGesture { onView(.Map) }
                        PipToggle(expanded: true, onToggle: togglePip)
                        Button { showMini(.Compass) } label: {
                            Image(.explore)
                                .font(.system(size: 13))
                                .foregroundStyle(theme.colors.onSurface)
                                .frame(width: PIP_TOGGLE_SIZE, height: PIP_TOGGLE_SIZE)
                                .background(theme.colors.surfaceContainerHigh, in: Circle())
                        }
                        .buttonStyle(.plain)
                        .accessibilityLabel("Show the compass")
                        .frame(maxWidth: .infinity, alignment: .topTrailing)
                        LayoutPipEditor(key: MAP_PIP_KEY, shape: AnyShape(mapPipShape))
                    }
                    .frame(width: MINIMAP_WIDTH, height: MINIMAP_HEIGHT)
                    .clipShape(mapPipShape)
                    .avoidedByVideoMessage(MAP_PIP_KEY)
                    .holdToEditLayout()
                    .layoutPlacement(MAP_PIP_KEY, keepOnScreen: true)
                    .accessibilityElement(children: .contain)
                    .accessibilityLabel("Mini-map")
                    .accessibilityAction(named: "Show the map full screen") { onView(.Map) }
                ))
                .zIndex(3)
            }
        }
        if videoIsPip && hasVideo {
            if pipExpanded {
                bottomStart(AnyView(
                    ZStack(alignment: .topLeading) {
                        Color.clear.contentShape(Rectangle())
                        PipToggle(expanded: true, onToggle: togglePip)
                        LayoutPipEditor(key: VIDEO_PIP_KEY, shape: AnyShape(RoundedRectangle(cornerRadius: Corner.medium)))
                    }
                    .frame(width: MINIMAP_WIDTH, height: MINIMAP_HEIGHT)
                    .clipShape(RoundedRectangle(cornerRadius: Corner.medium))
                    .videoGestures(pipHandlers(videoKey))
                    .layoutPlacement(VIDEO_PIP_KEY, keepOnScreen: true)
                    .accessibilityElement(children: .contain)
                    .accessibilityLabel("Video picture-in-picture")
                    .accessibilityAction(named: "Show the video large") { onView(.Video) }
                    .accessibilityAction(named: "Show the video full screen") { onFullScreen() }
                    .accessibilityAction(named: "Hide the video") { togglePip() }
                ))
                .zIndex(3)
            } else {
                bottomStart(AnyView(
                    PipToggle(expanded: false, onToggle: togglePip)
                        .verticalSwipe(VIDEO_SWIPE_DISTANCE) { if $0 == .Up { togglePip() } }
                ))
                .zIndex(3)
            }
        }
        if mapIsPip, let camera = pipCamera {
            CameraPipThumbnail(camera: camera) {
                LayoutPipEditor(key: CAMERA_PIP_KEY, shape: AnyShape(RoundedRectangle(cornerRadius: Corner.medium)))
            }
            .frame(width: CAMERA_PIP_WIDTH, height: CAMERA_PIP_HEIGHT)
            .avoidedByVideoMessage(CAMERA_PIP_KEY)
            .holdToEditLayout()
            .layoutPlacement(CAMERA_PIP_KEY, keepOnScreen: true)
            .padding(.leading, cutout.leading + ACTION_RAIL_CLEARANCE)
            .padding(.bottom, miniHeight(mini) + Space.s3 * 2)
            .frame(width: box.width, height: box.height, alignment: .bottomLeading)
            .zIndex(3)
        }
        flyChrome(box, safe, cutout, mapIsPip: mapIsPip, videoIsPip: videoIsPip, videoShown: videoShown)
            .opacity(navigation.settingsOpen ? 0 : 1)
            .zIndex(flyScreen.flightActions.deciding ? 4 : 1)
    }

    @ViewBuilder
    private func flyChrome(_ box: CGSize, _ safe: EdgeInsets, _ cutout: EdgeInsets, mapIsPip: Bool, videoIsPip: Bool, videoShown: Bool) -> some View {
        let inner = CGSize(width: max(box.width - cutout.leading - cutout.trailing, 0), height: box.height)
        let videoMessage = videoShown && !videoIsPip && videoReading(videoJson)?.decoding != true
        let videoPipOpen = hasVideo && pipExpanded
        let layersBottom = PIP_TOGGLE_SIZE + Space.s3 * 2
        let railFloor = mapIsPip ? miniHeight(mini) + Space.s3 + Space.s2
            : !videoIsPip ? 0
            : videoPipOpen ? MINIMAP_HEIGHT + Space.s3 + Space.s2
            : layersBottom + MAP_LAYERS_BUTTON + Space.s2
        ZStack(alignment: .topLeading) {
            if view == .Map {
                LayoutWidget(key: "mapLayers", hideable: false) {
                    Button { layers = true } label: {
                        Image(.layers)
                            .foregroundStyle(theme.colors.onSurface)
                            .frame(width: MAP_LAYERS_BUTTON, height: MAP_LAYERS_BUTTON)
                            .background(theme.colors.surfaceContainerHigh, in: Circle())
                    }
                    .buttonStyle(.plain)
                    .accessibilityLabel("Map layers")
                }
                .padding(.leading, videoPipOpen ? Space.s3 * 2 + MINIMAP_WIDTH : Space.s3)
                .padding(.bottom, videoPipOpen ? Space.s3 + MINIMAP_HEIGHT - MAP_LAYERS_BUTTON : layersBottom)
                .frame(width: inner.width, height: inner.height, alignment: .bottomLeading)
            }
            FlyChromeLayout(
                top: {
                    AnyView(
                        VStack(alignment: .leading, spacing: Space.s2) {
                            HStack(spacing: Space.s2) { status() }
                                .frame(maxWidth: .infinity, minHeight: STATUS_ROW_HEIGHT)
                                .environment(\.LocalCompactStatus, true)
                            TrafficBanner()
                            ScrollView(.horizontal, showsIndicators: false) {
                                HStack(spacing: Space.s2) { keyRow() }
                            }
                        }
                        .background(TrafficSheetHost())
                        .padding(.top, safe.top)
                        .padding(.horizontal, Space.s3)
                        .padding(.vertical, Space.s2)
                    )
                },
                overlays: {
                    AnyView(
                        VStack(alignment: .leading, spacing: Space.s2) { overlays() }
                            .padding(.leading, ACTION_RAIL_CLEARANCE)
                            .padding(.trailing, Space.s3)
                            .frame(maxWidth: .infinity, alignment: .leading)
                            .clipped()
                            .environment(\.LocalAvoidedByVideoMessage, true)
                    )
                },
                bottom: { AnyView(EmptyView()) },
                bottomAlignment: .trailing,
                overlaysAboveBottom: true,
                message: videoMessage ? { AnyView(FlyNoVideoMessage().osdShadow()) } : nil,
                messageAboveBottom: false,
                obstacles: { Array(flyScreen.obstacles.values) },
                onInsets: { insets in if insets != flyScreen.mapInsets { flyScreen.mapInsets = insets } }
            )
            .frame(width: inner.width, height: inner.height)
            .environment(\.flyOsd, view == .Video)
            rail(mapIsPip)
                .environment(\.LocalAvoidedByVideoMessage, true)
                .environment(\.flyOsd, view == .Video)
                .padding(.top, safe.top + RAIL_CLEARANCE)
                .padding(.bottom, RAIL_CLEARANCE)
                .padding(.trailing, Space.s3)
                .frame(width: inner.width, height: inner.height, alignment: .trailing)
                .zIndex(1)
            actions(.Rail)
                .environment(\.flyOsd, view == .Video)
                .environment(\.LocalRailFloor, railFloor)
                .padding(.top, STATUS_ROW_HEIGHT + Space.s4)
                .frame(width: inner.width, height: inner.height)
                .zIndex(2)
            if hasVehicle {
                let besideMini = mapIsPip ? miniWidth(mini) : hasVideo && pipExpanded ? MINIMAP_WIDTH : PIP_TOGGLE_SIZE
                TelemetryRow(valuesShown: true, chooser: false, compact: true, stacked: true)
                    .environment(\.flyOsd, true)
                    .osdShadow()
                    .fixedSize()
                    .padding(.leading, besideMini + Space.s3 * 2)
                    .padding(.bottom, Space.s3)
                    .frame(width: inner.width, height: inner.height, alignment: .bottomLeading)
                    .zIndex(2)
            }
        }
        .frame(width: inner.width, height: inner.height)
        .offset(x: cutout.leading)
    }
}

struct FleetCard: View {
    @Environment(FlyScreenState.self) private var flyScreen
    @Environment(\.LocalRootSize) private var root
    @Environment(\.flyOsd) private var flyOsd
    @Environment(\.theme) private var theme
    @QgcPath(VEHICLES_VIEW) private var vehiclesJson
    @QgcPath(MULTI_VEHICLE_PANEL) private var panelJson

    var body: some View {
        if root.width >= FLEET_CARD_MIN_SCREEN, !flyScreen.guidedPanelOpen,
           fleetPanelShown(vehicleChoices(vehiclesJson).choices.count, multiVehiclePanelEnabled(panelJson)) {
            ViewThatFits(in: .vertical) {
                panel
                ScrollView { panel }
            }
            .frame(maxWidth: FLEET_CARD_MAX_WIDTH, maxHeight: FLEET_CARD_MAX_HEIGHT)
            .background(osdBackdrop(theme.colors.surfaceContainer, flyOsd), in: RoundedRectangle(cornerRadius: Corner.large))
            .clipShape(RoundedRectangle(cornerRadius: Corner.large))
        }
    }

    private var panel: some View { FleetPanel().padding(.vertical, Space.s2) }
}

let MAP_LAYERS_CLEARANCE: CGFloat = MAP_LAYERS_BUTTON + Space.s3 + Space.s2
private let FLEET_CARD_MAX_WIDTH: CGFloat = 440
private let FLEET_CARD_MAX_HEIGHT: CGFloat = 320
private let FLEET_CARD_MIN_SCREEN: CGFloat = 600

struct MapInsets: Equatable {
    var top: CGFloat
    var bottom: CGFloat
}

private struct FlyChromeLayout: View {
    let top: () -> AnyView
    let overlays: () -> AnyView
    let bottom: () -> AnyView
    let bottomAlignment: HorizontalAlignment
    let overlaysAboveBottom: Bool
    let message: (() -> AnyView)?
    let messageAboveBottom: Bool
    let obstacles: () -> [CGRect]
    let onInsets: (MapInsets) -> Void
    @State private var topHeight: CGFloat = 0
    @State private var overlayHeight: CGFloat = 0
    @State private var bottomHeight: CGFloat = 0

    var body: some View {
        GeometryReader { geometry in
            let width = geometry.size.width
            let height = geometry.size.height
            let origin = geometry.frame(in: .global).origin
            let overlayRoom = max(height - topHeight - (overlaysAboveBottom ? bottomHeight : 0), 0)
            let free = CGRect(
                x: Space.s3,
                y: topHeight,
                width: width - Space.s3 * 2,
                height: height - Space.s3 - (messageAboveBottom ? bottomHeight : 0) - topHeight
            )
            let region = messageRegion(free, obstacles().map { $0.offsetBy(dx: -origin.x, dy: -origin.y) }, Space.s3, NO_VIDEO_PILL_WIDTH, NO_VIDEO_PILL_HEIGHT)
                .map { centredRegion($0, width / 2, NO_VIDEO_PILL_WIDTH) }
            ZStack(alignment: .topLeading) {
                if let message, let region {
                    message()
                        .frame(width: region.width, height: region.height)
                        .offset(x: region.minX, y: region.minY)
                }
                top()
                    .frame(width: width, alignment: .topLeading)
                    .fixedSize(horizontal: false, vertical: true)
                    .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { topHeight = $0 }
                overlays()
                    .frame(width: width, alignment: .topLeading)
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxHeight: overlayRoom, alignment: .top)
                    .clipped()
                    .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { overlayHeight = $0 }
                    .offset(y: topHeight)
                bottom()
                    .fixedSize()
                    .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { bottomHeight = $0 }
                    .frame(width: width, height: height, alignment: Alignment(horizontal: bottomAlignment, vertical: .bottom))
            }
            .frame(width: width, height: height, alignment: .topLeading)
        }
        .onChange(of: MapInsets(top: topHeight + overlayHeight, bottom: bottomHeight), initial: true) { _, insets in onInsets(insets) }
    }
}

struct AvoidedByVideoMessage: ViewModifier {
    let key: String
    let active: Bool
    @Environment(FlyScreenState.self) private var flyScreen

    func body(content: Content) -> some View {
        content
            .onGeometryChange(for: CGRect.self) { $0.frame(in: .global) } action: { rect in
                guard active, flyScreen.obstacles[key] != rect else { return }
                flyScreen.obstacles[key] = rect
            }
            .onDisappear { flyScreen.obstacles[key] = nil }
    }
}

extension View {
    func avoidedByVideoMessage(_ key: String) -> some View { modifier(AvoidedByVideoMessage(key: key, active: true)) }
}

struct MenuDestination: Identifiable {
    let label: String
    let icon: Icon
    let current: Bool
    let onSelect: () -> Void

    var id: String { label }
}

struct FlyTabMenu: View {
    let destinations: [MenuDestination]
    @Environment(\.theme) private var theme
    @Environment(\.flyOsd) private var flyOsd

    var body: some View {
        Menu {
            ForEach(destinations) { destination in
                Button {
                    if !destination.current { destination.onSelect() }
                } label: {
                    Label(destination.label, systemImage: destination.current ? "checkmark" : destination.icon.rawValue)
                }
            }
        } label: {
            Image(.menu)
                .font(.system(size: 18, weight: .medium))
                .foregroundStyle(theme.aircast.outdoorForeground)
                .frame(width: STATUS_ROW_HEIGHT + Space.s2, height: STATUS_ROW_HEIGHT + Space.s2)
                .background(osdBackdrop(Color.black.opacity(FLY_SCRIM_ALPHA), flyOsd), in: Circle())
        }
        .accessibilityLabel("Menu")
    }
}
