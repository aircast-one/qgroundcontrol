import Foundation

private let linksCommit = "links.commitLinkConfigurations"
private let linksBluetoothScan = "links.bluetoothScan"
private let linksConnect = "links.createConnectedLink"
private let linksRemove = "links.removeConfiguration"
private let linksCreateCloud = "links.createAircastCloudLink"
private let linksCreateReplay = "links.createLogReplayConfiguration"
private let linksCreateBluetooth = "links.createBluetoothLink"
private let linksCreateSerial = "links.createSerialConfiguration"
private let linksCreateSupport = "links.createMavlinkForwardingSupportLink"
private let linksStartMock = "links.startMockLink"
private let accountState = "account"
private let accountApiBase = "account.apiBase"
private let accountSignIn = "account.signIn"
private let accountSignOut = "account.signOut"
private let accountCancelSignIn = "account.cancelSignIn"
private let planRedo = "plan.redo"
private let planUndoTracking = "plan.undoTracking"
private let planRemoveAllFromVehicle = "plan.removeAllFromVehicle"
private let planResumeMission = "planFly.missionController.resumeMission"
private let planCommandCategories = "missionCommandTree.categoriesForVehicle"
private let planCommandsForCategory = "missionCommandTree.getCommandsForCategory"
private let missionCompleteDismiss = "missionComplete.dismiss"
private let videoNativeRendering = "video.setNativeRendering"
private let videoInitNative = "video.initNative"
private let videoActiveSource = "video.setActiveVideoSource"
private let videoStartRecording = "video.startRecording"
private let videoStopRecording = "video.stopRecording"
private let videoRestart = "video.restart"
private let videoDeviceCameraRotation = "video.setDeviceCameraRotation"
private let videoPictureInPicture = "settings.videoSettings.multiViewEnabled"
private let videoPipShown = "video.setPipShown"
private let videoStreamEnabled = "settings.videoSettings.streamEnabled"
private let camerasAdd = "cameras.add"
private let camerasUpdate = "cameras.update"
private let camerasRemove = "cameras.remove"
private let camerasMove = "cameras.move"
private let camerasClassify = "cameras.classify"
private let vehicleFieldsPath = "vehicle"
private let escCalibrationStart = "escCalibration.start"
private let escCalibrationClose = "escCalibration.close"
private let subMotorsOpen = "apmSubMotors.open"
private let subMotorsTest = "apmSubMotors.test"
private let inspectorMessageInterval = "mavlinkInspector.setMessageInterval"
private let inspectorActiveSystem = "mavlinkInspector.setActiveSystem"
private let gimbalControl = "gimbal.control"
private let firstRunShown = "firstRun.markShown"
private let subtitlesInstruments = "subtitles.setInstruments"
private let hostAcknowledge = "host.acknowledgeThrough"

enum LinkCommands {
    @discardableResult static func commitConfigurations() -> Bool { Qgc.invoke(linksCommit) }
    @discardableResult static func scanBluetooth(_ scanning: Bool) -> Bool { Qgc.invoke(linksBluetoothScan, scanning) }
    @discardableResult static func connect(_ configuration: String) -> Bool { Qgc.invoke(linksConnect, configuration) }
    @discardableResult static func remove(_ configuration: String) -> Bool { Qgc.invoke(linksRemove, configuration) }
    @discardableResult static func createAircastCloud(_ name: String, apiBase: String, deviceId: String) -> Bool { Qgc.invokeResult(linksCreateCloud, name, apiBase, deviceId) == .bool(true) }
    @discardableResult static func createLogReplay(_ name: String, log: String) -> Bool { Qgc.invokeResult(linksCreateReplay, name, log) == .bool(true) }
    @discardableResult static func createBluetooth(_ name: String, deviceName: String, address: String) -> Bool { Qgc.invokeResult(linksCreateBluetooth, name, deviceName, address) == .bool(true) }
    @discardableResult static func createSerial(_ name: String, port: String, baud: Int) -> Bool { Qgc.invokeResult(linksCreateSerial, name, port, baud) == .bool(true) }
    @discardableResult static func createSupportForwarding() -> Bool { Qgc.invoke(linksCreateSupport) }
    @discardableResult static func startMock(_ arguments: [Any]) -> Bool { Qgc.call(linksStartMock, arguments: arguments)?["result"] == .bool(true) }
}

enum AccountCommands {
    static func state() -> JSON { Qgc.get(accountState) }
    @discardableResult static func setApiBase(_ apiBase: String) -> Bool { Qgc.set(accountApiBase, apiBase) }
    @discardableResult static func signIn() -> Bool { Qgc.invoke(accountSignIn) }
    @discardableResult static func signOut() -> Bool { Qgc.invoke(accountSignOut) }
    @discardableResult static func cancelSignIn() -> Bool { Qgc.invoke(accountCancelSignIn) }
}

enum PlanCommands {
    @discardableResult static func redo() -> Bool { Qgc.invoke(planRedo) }
    @discardableResult static func setUndoTracking(_ tracking: Bool) -> Bool { Qgc.set(planUndoTracking, tracking) }
    @discardableResult static func removeAllFromVehicle() -> Bool { Qgc.invoke(planRemoveAllFromVehicle) }
    @discardableResult static func resumeMission(_ waypoint: Int) -> Bool { Qgc.invoke(planResumeMission, waypoint) }
    @discardableResult static func dismissMissionComplete(_ notice: Int64) -> Bool { Qgc.invoke(missionCompleteDismiss, notice) }
    static func commandCategories() -> JSON { Qgc.invokeResult(planCommandCategories) }
    static func commandsForCategory(_ category: String) -> JSON { Qgc.invokeResult(planCommandsForCategory, nil, category, true) }
    static func setLandingHeading(_ index: Int, _ heading: Double) -> String? { Qgc.writeRefusal("plan.missionController.visualItems.\(index).landingHeading", heading) }
    static func setLandingCoordinate(_ index: Int, _ coordinate: JSON) -> String? { Qgc.writeRefusal("plan.missionController.visualItems.\(index).landingCoordinate", coordinate.any) }
    static func setAltitudesRelative(_ index: Int, _ relative: Bool) -> String? { Qgc.writeRefusal("plan.missionController.visualItems.\(index).altitudesAreRelative", relative) }
}

enum VideoCommands {
    @discardableResult static func setNativeRendering(_ native: Bool) -> Bool { Qgc.invoke(videoNativeRendering, native) }
    @discardableResult static func initNative() -> Bool { Qgc.invoke(videoInitNative) }
    @discardableResult static func setActiveSource(_ slot: Int) -> Bool { Qgc.invoke(videoActiveSource, slot) }
    @discardableResult static func setRecording(_ recording: Bool) -> Bool { Qgc.invoke(recording ? videoStartRecording : videoStopRecording) }
    @discardableResult static func restart() -> Bool {
        VideoDriver.restart(MAIN_VIDEO_CHANNEL)
        return Qgc.invoke(videoRestart)
    }
    @discardableResult static func setDeviceCameraRotation(_ degrees: Int) -> Bool { Qgc.invoke(videoDeviceCameraRotation, degrees) }
    @discardableResult static func setPictureInPicture(_ shown: Bool) -> Bool { Qgc.set(videoPictureInPicture, shown) }
    @discardableResult static func setPipShown(_ shown: Bool) -> Bool { Qgc.invoke(videoPipShown, shown) }
    @discardableResult static func turnStreamOn() -> Bool { Qgc.set(videoStreamEnabled, true) }
}

enum CameraCommands {
    static func add(_ name: String, source: String, url: String) -> String? { Qgc.refusalOf(camerasAdd, name, source, url) }
    static func update(_ slot: Int, name: String, source: String, url: String) -> String? { Qgc.refusalOf(camerasUpdate, slot, name, source, url) }
    static func remove(_ slot: Int) -> String? { Qgc.refusalOf(camerasRemove, slot) }
    static func move(_ from: Int, to: Int) -> String? { Qgc.refusalOf(camerasMove, from, to) }
    static func classify(_ address: String) -> JSON? { Qgc.call(camerasClassify, address) }
}

enum SetupCommands {
    static func vehicleFields(_ fields: [String]) -> JSON { Qgc.get(vehicleFieldsPath, fields: fields) }
    @discardableResult static func startEscCalibration() -> Bool { Qgc.invoke(escCalibrationStart) }
    @discardableResult static func closeEscCalibration() -> Bool { Qgc.invoke(escCalibrationClose) }
    @discardableResult static func openSubMotors() -> Bool { Qgc.invoke(subMotorsOpen) }
    @discardableResult static func testSubMotor(_ motor: Int, _ value: Double) -> Bool { Qgc.invoke(subMotorsTest, motor, value) }
    @discardableResult static func setInspectorMessageInterval(_ rate: Int) -> Bool { Qgc.invoke(inspectorMessageInterval, rate) }
    @discardableResult static func setInspectorSystem(_ system: Int) -> Bool { Qgc.invoke(inspectorActiveSystem, system) }
    static func takeGimbalControlRefusal() -> String? { Qgc.refusalOf(gimbalControl, true) }
}

enum AppCommands {
    @discardableResult static func markFirstRunShown() -> Bool { Qgc.invoke(firstRunShown) }
    @discardableResult static func setSubtitleInstruments(_ vehicleClass: String, _ instruments: [String]) -> Bool { Qgc.invoke(subtitlesInstruments, vehicleClass, instruments.joined(separator: ",")) }
    @discardableResult static func acknowledgeNoticesThrough(_ through: Int64) -> Bool { Qgc.invoke(hostAcknowledge, through) }
}

func coordinateJson(_ latitude: Double, _ longitude: Double, _ altitudeMetres: Double = 0) -> [String: Double] {
    ["latitude": latitude, "longitude": longitude, "altitude": altitudeMetres]
}

func coordinateJson(_ at: TrackPoint) -> [String: Double] { coordinateJson(at.latitude, at.longitude) }

@discardableResult
func setOk(_ path: String, _ value: Any?) -> Bool { Qgc.set(path, value) }

let PLAN_UNDO = "plan.undo"

@discardableResult
func invokeOk(_ path: String, _ args: Any?...) -> Bool { Qgc.call(path, arguments: args)?["ok"].bool ?? false }
