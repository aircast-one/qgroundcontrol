import SwiftUI

private let VIDEO_ASPECT: CGFloat = 16.0 / 9.0
private let MAP_BUTTON_SCRIM_ALPHA = 0.55
private let PORTRAIT_PIP_WIDTH: CGFloat = 160
private let PORTRAIT_PIP_HEIGHT: CGFloat = 90
private let PORTRAIT_DIAL_SIZE: CGFloat = 120
private let PORTRAIT_BAR_HEIGHT: CGFloat = 64
private let SCRIM_ALPHA = 0.7
private let PIP_BORDER_ALPHA = 0.5
private let DECK_SCRIM_START = 0.4
private let MAP_BUTTON_SIZE: CGFloat = 40
private let MAP_ATTRIBUTION_CLEARANCE: CGFloat = 28
private let PORTRAIT_CAMERA_PIP_WIDTH: CGFloat = 96
private let PORTRAIT_CAMERA_PIP_HEIGHT: CGFloat = 54

func portraitSplit(_ view: FlyView, _ hasVideo: Bool) -> Bool { view == .Video && hasVideo }

func portraitShowsCamera(_ reading: VideoReading?) -> Bool {
    guard let reading else { return false }
    return reading.available || (!reading.streamEnabled && reading.sourceChosen)
}

func portraitVideoThumbnail(_ split: Bool, _ reading: VideoReading?) -> Bool { !split && reading?.available == true }

struct FlyPortrait: View {
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
    @QgcPath(VIDEO_VIEW) private var videoJson
    @CameraStepper private var cameras
    @GimbalDrags private var gimbalDrags
    @ShownPipCamera private var pipCamera
    @State private var chromeHeight: CGFloat = 0
    @State private var deckHeight: CGFloat = 0
    @State private var pipDrag = CGSize.zero
    @State private var holding = false

    var body: some View {
        GeometryReader { safeArea in
            GeometryReader { geometry in
                portrait(geometry.size, safeArea.safeAreaInsets)
            }
            .ignoresSafeArea()
        }
        .background(theme.aircast.outdoorBackground.ignoresSafeArea())
    }

    @ViewBuilder
    private func portrait(_ box: CGSize, _ safe: EdgeInsets) -> some View {
        let reading = videoReading(videoJson)
        let hasVideo = portraitShowsCamera(reading)
        let split = portraitSplit(view, hasVideo)
        let barTop = max(safe.top + PORTRAIT_BAR_HEIGHT, chromeHeight)
        let videoHeight = box.width / VIDEO_ASPECT
        let videoTop = barTop
        let mapTop = split ? barTop + videoHeight : 0
        let controlsTop = split ? mapTop : barTop
        let thumbnail = portraitVideoThumbnail(split, reading)
        let room = pipRoom(thumbnail, flyScreen.videoTucked)
        let endRoom = flyScreen.pipStart ? 0 : room
        let startRoom = flyScreen.pipStart ? room : 0
        let buttonsTop = Space.s3 + endRoom
        let overlaysTop = max(Space.s3 + endRoom + MAP_BUTTON_SIZE + Space.s2, Space.s3 + startRoom)
        let geometry = PipGeometry(
            width: box.width,
            pip: CGSize(width: PORTRAIT_PIP_WIDTH, height: PORTRAIT_PIP_HEIGHT),
            inset: Space.s3,
            pipTop: barTop + Space.s3,
            split: CGRect(x: 0, y: videoTop, width: box.width, height: videoHeight),
            full: CGRect(x: 0, y: 0, width: box.width, height: box.height > 0 ? box.height : videoTop + videoHeight)
        )
        let target = fullScreen ? geometry.full : split ? geometry.split : geometry.pip(flyScreen.pipStart, pipDrag)
        ZStack(alignment: .topLeading) {
            Group {
                if view == .ThreeD { Viewer3DPane() } else { map() }
            }
            .padding(.top, mapTop)
            .frame(width: box.width, height: box.height)
            .accessibilityHidden(fullScreen)
            if hasVideo && (split || !flyScreen.videoTucked) {
                let shape = RoundedRectangle(cornerRadius: split ? 0 : Corner.medium)
                video(split)
                    .background(Color.black)
                    .clipShape(shape)
                    .overlay(shape.stroke(theme.aircast.outdoorForeground.opacity(PIP_BORDER_ALPHA), lineWidth: split ? 0 : 1))
                    .contentShape(Rectangle())
                    .videoGestures(handlers(split, target, geometry))
                    .accessibilityAction(named: "Show the video full screen") { onFullScreen() }
                    .accessibilityActions {
                        if split {
                            Button("Make the video small") { onView(.Map) }
                        } else {
                            Button("Hide the video") { flyScreen.videoTucked = true }
                        }
                    }
                    .videoFrame(target, holding: holding, nudge: cameras.nudge)
                    .transition(.opacity.combined(with: .move(edge: .top)))
                    .zIndex(fullScreen ? FULL_SCREEN_LAYER : 0)
            }
            if !fullScreen {
                if split {
                    VStack(spacing: 0) {
                        ZStack {
                            if reading?.decoding != true { FlyNoVideoMessage().osdShadow() }
                        }
                        .frame(maxWidth: .infinity, maxHeight: .infinity)
                        ScrollView(.horizontal, showsIndicators: false) {
                            HStack(spacing: Space.s2) { keyRow() }
                        }
                        .padding(Space.s2)
                        .environment(\.flyOsd, true)
                    }
                    .frame(width: box.width, height: videoHeight)
                    .offset(y: videoTop)
                    .transition(.opacity)
                    if let camera = pipCamera {
                        CameraPipThumbnail(camera: camera)
                            .frame(width: PORTRAIT_CAMERA_PIP_WIDTH, height: PORTRAIT_CAMERA_PIP_HEIGHT)
                            .padding(Space.s2)
                            .offset(y: videoTop)
                    }
                }
                VStack(spacing: 0) {
                    controls(split, hasVideo, overlaysTop: overlaysTop, buttonsTop: buttonsTop)
                        .padding(.top, controlsTop)
                        .frame(maxHeight: .infinity)
                    actions(.Bottom)
                        .padding(.bottom, safe.bottom)
                        .frame(maxWidth: .infinity)
                        .background(
                            LinearGradient(
                                stops: [.init(color: .clear, location: 0), .init(color: Color.black.opacity(SCRIM_ALPHA), location: DECK_SCRIM_START)],
                                startPoint: .top,
                                endPoint: .bottom
                            )
                            .allowsHitTesting(false)
                        )
                        .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { deckHeight = $0 }
                }
                .frame(width: box.width, height: box.height)
                if thumbnail && flyScreen.videoTucked {
                    VideoTab(swipeDistance: VIDEO_SWIPE_DISTANCE) { flyScreen.videoTucked = false }
                        .padding(.top, barTop + Space.s3)
                        .padding(.horizontal, Space.s3)
                        .frame(width: box.width, alignment: flyScreen.pipStart ? .leading : .trailing)
                        .transition(.opacity.combined(with: .move(edge: .top)))
                }
                if split {
                    SplitHandle(
                        swipeDistance: VIDEO_SWIPE_DISTANCE,
                        onSmaller: {
                            flyScreen.videoTucked = false
                            onView(.Map)
                        },
                        onFullScreen: onFullScreen
                    )
                    .frame(width: box.width)
                    .offset(y: videoTop + videoHeight)
                    .transition(.opacity)
                }
                VStack(alignment: .leading, spacing: 0) {
                    HStack(spacing: Space.s2) { status() }
                        .padding(.horizontal, Space.s3)
                        .frame(maxWidth: .infinity)
                        .frame(height: PORTRAIT_BAR_HEIGHT)
                        .padding(.top, safe.top)
                        .background(LinearGradient(colors: [Color.black.opacity(SCRIM_ALPHA), .clear], startPoint: .top, endPoint: .bottom).allowsHitTesting(false))
                    TrafficBanner()
                        .padding(.horizontal, Space.s3)
                        .padding(.bottom, Space.s2)
                }
                .frame(width: box.width)
                .onGeometryChange(for: CGFloat.self) { $0.size.height } action: { chromeHeight = $0 }
            }
        }
        .frame(width: box.width, height: box.height, alignment: .topLeading)
        .clipped()
        .animation(.default, value: split)
        .animation(.default, value: flyScreen.videoTucked)
        .animation(.default, value: buttonsTop)
        .onChange(of: MapInsets(top: split ? 0 : barTop, bottom: deckHeight), initial: true) { _, insets in flyScreen.mapInsets = insets }
    }

    private func controls(_ split: Bool, _ hasVideo: Bool, overlaysTop: CGFloat, buttonsTop: CGFloat) -> some View {
        ZStack(alignment: .topLeading) {
            VStack(alignment: .leading, spacing: 0) {
                VStack(alignment: .leading, spacing: Space.s2) { overlays() }
                    .fixedSize(horizontal: false, vertical: true)
                    .frame(maxHeight: .infinity, alignment: .top)
                    .clipped()
                OsdCompassDial(size: PORTRAIT_DIAL_SIZE)
                    .frame(width: PORTRAIT_DIAL_SIZE, height: PORTRAIT_DIAL_SIZE)
                    .padding(.top, Space.s2)
            }
            .padding(.leading, Space.s3)
            .padding(.top, overlaysTop)
            .padding(.trailing, Space.s3)
            .padding(.bottom, MAP_ATTRIBUTION_CLEARANCE)
            .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topLeading)
            MapButtons(view: view, split: split, hasVideo: hasVideo, onView: onView)
                .padding(.trailing, Space.s3)
                .padding(.top, buttonsTop)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .topTrailing)
            rail(split)
                .padding(.trailing, Space.s3)
                .padding(.bottom, MAP_SCALE_CLEARANCE)
                .frame(maxWidth: .infinity, maxHeight: .infinity, alignment: .bottomTrailing)
        }
    }

    private func handlers(_ split: Bool, _ target: CGRect, _ geometry: PipGeometry) -> VideoGestureHandlers {
        let stepper = cameras
        let gimbal = gimbalDrags
        let screen = flyScreen
        return VideoGestureHandlers(
            owned: { !split },
            claimsSwipe: { moved in sideways(moved) && !gimbal },
            onTap: { onView(.Video) },
            onDoubleTap: { onFullScreen() },
            onSwipe: { moved in
                let swipe = videoSwipe(moved, VIDEO_SWIPE_DISTANCE)
                switch swipe {
                case .Up: if !split { screen.videoTucked = true }
                case .Down: if !split { onView(.Video) }
                case .Left, .Right: cameraStep(swipe).map(stepper.step)
                case nil: break
                }
            },
            onHold: { at in
                holding = true
                if split {
                    pipDrag = geometry.dragToCentre(screen.pipStart, CGPoint(x: target.minX + at.x, y: target.minY + at.y))
                    screen.videoTucked = false
                    onExitFullScreen()
                    onView(.Map)
                }
                return true
            },
            onHoldDrag: { delta in pipDrag = CGSize(width: pipDrag.width + delta.width, height: pipDrag.height + delta.height) },
            onHoldEnd: {
                screen.pipStart = pipOnStart(geometry.pip(screen.pipStart, pipDrag).midX, geometry.width)
                pipDrag = .zero
                holding = false
            }
        )
    }
}

let VIDEO_TAB_HEIGHT: CGFloat = 36
private let FULL_SCREEN_LAYER: Double = 1
private let HANDLE_TOUCH = CGSize(width: 96, height: 28)
private let HANDLE_BAR = CGSize(width: 40, height: 4)
private let HANDLE_ALPHA = 0.8

func pipRoom(_ thumbnail: Bool, _ tucked: Bool) -> CGFloat {
    if !thumbnail { return 0 }
    return tucked ? VIDEO_TAB_HEIGHT + Space.s3 : PORTRAIT_PIP_HEIGHT + Space.s3
}

struct PipGeometry: Equatable {
    let width: CGFloat
    let pip: CGSize
    let inset: CGFloat
    let pipTop: CGFloat
    let split: CGRect
    let full: CGRect

    func anchor(_ start: Bool) -> CGPoint { CGPoint(x: start ? inset : width - inset - pip.width, y: pipTop) }

    func pip(_ start: Bool, _ drag: CGSize) -> CGRect {
        let at = anchor(start)
        return CGRect(origin: CGPoint(x: at.x + drag.width, y: at.y + drag.height), size: pip)
    }

    func dragToCentre(_ start: Bool, _ finger: CGPoint) -> CGSize {
        let at = anchor(start)
        return CGSize(width: finger.x - at.x - pip.width / 2, height: finger.y - at.y - pip.height / 2)
    }
}

private struct VideoTab: View {
    let swipeDistance: CGFloat
    let onShow: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onShow) {
            HStack(spacing: Space.s1) {
                Image(.videocam).font(.system(size: 14))
                Text("Video").font(.labelLarge)
            }
            .padding(.horizontal, Space.s3)
            .frame(height: VIDEO_TAB_HEIGHT)
            .foregroundStyle(theme.aircast.outdoorForeground)
            .background(Color.black.opacity(SCRIM_ALPHA), in: Capsule())
        }
        .buttonStyle(.plain)
        .verticalSwipe(swipeDistance) { if $0 == .Down { onShow() } }
        .accessibilityLabel("Show the video")
    }
}

private struct SplitHandle: View {
    let swipeDistance: CGFloat
    let onSmaller: () -> Void
    let onFullScreen: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Capsule()
            .fill(theme.aircast.outdoorForeground.opacity(HANDLE_ALPHA))
            .frame(width: HANDLE_BAR.width, height: HANDLE_BAR.height)
            .osdShadow()
            .frame(width: HANDLE_TOUCH.width, height: HANDLE_TOUCH.height)
            .contentShape(Rectangle())
            .verticalSwipe(swipeDistance) { swipe in if swipe == .Up { onSmaller() } else { onFullScreen() } }
            .accessibilityElement()
            .accessibilityLabel("Video size")
            .accessibilityAction(named: "Make the video small") { onSmaller() }
            .accessibilityAction(named: "Show the video full screen") { onFullScreen() }
    }
}

private struct Viewer3DGate: Equatable {
    let enabled: Bool?
    let view: FlyView
}

private struct MapButtons: View {
    let view: FlyView
    let split: Bool
    let hasVideo: Bool
    let onView: (FlyView) -> Void
    @QgcPath(VIEWER3D_VIEW) private var viewer3dJson
    @State private var layers = false

    var body: some View {
        let enabled3d = viewer3dEnabled(viewer3dJson)
        HStack(spacing: Space.s2) {
            if split {
                MapButton(icon: .fullscreen, label: "Enlarge the map") { onView(.Map) }
            }
            if !split && hasVideo && view == .ThreeD {
                MapButton(icon: .videocam, label: "Show the camera above the map") { onView(.Video) }
            }
            MapButton(icon: .layers, label: "Map layers") { layers = true }
            if flyViewsOffered(enabled3d, view).contains(.ThreeD) {
                MapButton(icon: view == .ThreeD ? .map : .explore, label: view == .ThreeD ? "Show the map" : "Show the 3D view") {
                    onView(view == .ThreeD ? .Map : .ThreeD)
                }
            }
        }
        .background {
            if layers { MapLayersSheet { layers = false } }
        }
        .onChange(of: Viewer3DGate(enabled: enabled3d, view: view), initial: true) { _, gate in
            let allowed = flyViewAllowed(gate.enabled, gate.view)
            if allowed != gate.view { onView(allowed) }
        }
    }
}

private struct MapButton: View {
    let icon: Icon
    let label: String
    let onClick: () -> Void
    @Environment(\.theme) private var theme

    var body: some View {
        Button(action: onClick) {
            Image(icon)
                .font(.system(size: 16))
                .foregroundStyle(theme.aircast.outdoorForeground)
                .frame(width: MAP_BUTTON_SIZE, height: MAP_BUTTON_SIZE)
                .background(Color.black.opacity(MAP_BUTTON_SCRIM_ALPHA), in: Circle())
        }
        .buttonStyle(.plain)
        .accessibilityLabel(label)
    }
}
