# macOS head: Qt paths still passed through

Measured from macos/Sources by the regex on every `Bridge.*`/`BridgeWatch.*` string
literal: **148 distinct paths**, of which **84 reach Qt** (62 commands, 3 writes, 19 reads), 61 are core `view.*` and 3 host.

Coordination measured 68 Qt / 57 commands on the same tree. The difference is the
interpolated paths -- `\(path).appendVertex` and the eight others whose literal is a
fragment -- which a literal-string count either keeps or drops. Counted here, because a
path assembled at runtime still reaches Qt. **Neither number is the deliverable: what
each path BECAME is.**

Status values: `passthrough` = still Qt, no core equivalent asked for yet;
`requested <date>` = one-line request sent to the Rust session; `-> view.x` = converted.

**Measured before starting: none of the 84 can be converted unilaterally today.** The core
serves 84 view roots and this head already reads or explicitly excuses every one of them
(view-fields.py: "0 are neither"), so there is no existing core view sitting unused that any
of these paths could be switched to. Every conversion below needs the core to serve something
it does not serve yet -- which is what coordination says the Rust session is about to do for
the commands. The list is therefore a REQUEST list first and a conversion list second, and
that is the honest starting state rather than a count I could walk down on my own.

## Commands (`Bridge.invoke`)

| path | file | status |
|---|---|---|
| `\(link.path).link.disconnect` | Links.swift | passthrough |
| `\(path).appendVertex` | Mission.swift | passthrough |
| `\(polygon.path).\(polygon.adjustInvokable)` | Mission.swift | passthrough |
| `\(polygon.path).\(polygon.removeInvokable)` | Mission.swift | passthrough |
| `\(polygon.path).\(polygon.splitInvokable)` | Mission.swift | passthrough |
| `geoTag.cancelTagging` | GeoTag.swift | passthrough |
| `geoTag.startTagging` | GeoTag.swift | passthrough |
| `links.createConnectedLink` | Links.swift | passthrough |
| `links.createMavlinkForwardingSupportLink` | RemoteSupport.swift | passthrough |
| `links.endMavlinkForwardingSupportLink` | RemoteSupport.swift | passthrough |
| `links.removeConfiguration` | Links.swift | passthrough |
| `logDownload.cancel` | LogDownload.swift | passthrough |
| `logDownload.download` | LogDownload.swift | passthrough |
| `logDownload.eraseAll` | LogDownload.swift | passthrough |
| `logDownload.refresh` | LogDownload.swift | passthrough |
| `mavlinkInspector.setMessageInterval` | MavlinkInspector.swift | passthrough |
| `mission.insert` | Mission.swift | passthrough |
| `mission.remove` | Mission.swift | passthrough |
| `missionCommandTree.categoriesForVehicle` | Mission.swift | passthrough |
| `missionCommandTree.getCommandsForCategory` | Mission.swift | passthrough |
| `plan.geoFenceController.\(circle ? ` | FenceRally.swift | passthrough |
| `plan.geoFenceController.deleteCircle` | FenceRally.swift | passthrough |
| `plan.geoFenceController.deletePolygon` | FenceRally.swift | passthrough |
| `plan.loadFromFile` | Mission.swift | passthrough |
| `plan.loadFromVehicle` | FenceRally.swift,Mission.swift | passthrough |
| `plan.missionController.insertComplexMissionItem` | Mission.swift | passthrough |
| `plan.missionController.insertComplexMissionItemFromKMLOrSHP` | Mission.swift | passthrough |
| `plan.missionController.insertLandItem` | Mission.swift | passthrough |
| `plan.missionController.insertTakeoffItem` | Mission.swift | passthrough |
| `plan.missionController.setCurrentPlanViewSeqNum` | Mission.swift | passthrough |
| `plan.missionController.visualItems.\(item.index).setMapCenterHintForCommandChange` | Mission.swift | passthrough |
| `plan.rallyPointController.addPoint` | FenceRally.swift | passthrough |
| `plan.rallyPointController.removePoint` | FenceRally.swift | passthrough |
| `plan.redo` | Mission.swift | passthrough |
| `plan.removeAll` | Mission.swift | passthrough |
| `plan.saveToCurrent` | Mission.swift | passthrough |
| `plan.saveToFile` | Mission.swift | passthrough |
| `plan.saveToKml` | Mission.swift | passthrough |
| `plan.sendToVehicle` | Mission.swift | passthrough |
| `plan.undo` | Mission.swift | passthrough |
| `planFly.missionController.resumeMission` | GuidedActions.swift | passthrough |
| `radioCal.cancelButtonClicked` | Radio.swift | passthrough |
| `radioCal.nextButtonClicked` | Radio.swift | passthrough |
| `radioCal.skipButtonClicked` | Radio.swift | passthrough |
| `sensorsCal.cancelCalibration` | Sensors.swift | passthrough |
| `sensorsCal.nextClicked` | Sensors.swift | passthrough |
| `vehicle.\(command)` | GuidedActions.swift | passthrough |
| `vehicle.abortLanding` | GuidedActions.swift | passthrough |
| `vehicle.emergencyStop` | GuidedActions.swift | passthrough |
| `vehicle.forceArm` | GuidedActions.swift | passthrough |
| `vehicle.guidedModeChangeAltitude` | GuidedActions.swift | passthrough |
| `vehicle.guidedModeLand` | GuidedActions.swift | passthrough |
| `vehicle.guidedModeRTL` | GuidedActions.swift | passthrough |
| `vehicle.guidedModeTakeoff` | GuidedActions.swift | passthrough |
| `vehicle.motorTest` | Motors.swift | passthrough |
| `vehicle.parameterManager.parameterNames` | Parameters.swift | passthrough |
| `vehicle.pauseVehicle` | Mission.swift | passthrough |
| `vehicle.sendGripperAction` | GuidedActions.swift | passthrough |
| `vehicle.startMission` | GuidedActions.swift | passthrough |
| `vehicle.stopGuidedModeROI` | GuidedActions.swift | passthrough |
| `video.setNativeRendering` | Video.swift | passthrough |
| `video.switchActiveVideoSource` | Video.swift | passthrough |

## Writes (`Bridge.set`)

| path | file | status |
|---|---|---|
| `logDownload.model.\(entry.index).selected` | LogDownload.swift | passthrough |
| `mavlinkInspector.activeSystem.selected` | MavlinkInspector.swift | passthrough |
| `plan.undoTracking` | Mission.swift | passthrough |

## Reads (`Bridge.group`/`get`)

| path | file | status |
|---|---|---|
| `geoTag` | GeoTag.swift | passthrough |
| `links` | Links.swift | passthrough |
| `mavlinkInspector.activeSystem` | MavlinkInspector.swift | passthrough |
| `mavlinkInspector.activeSystem.messages.\(current.index).fields` | MavlinkInspector.swift | passthrough |
| `plan` | Mission.swift | passthrough |
| `plan.geoFenceController` | FenceRally.swift | passthrough |
| `plan.missionController` | Mission.swift | passthrough |
| `plan.missionController.visualItems.\(item.index)` | Mission.swift | passthrough |
| `plan.missionController.visualItems.\(item.index).\(list)` | Mission.swift | passthrough |
| `plan.missionController.visualItems.\(item.index).\(property)` | Mission.swift | passthrough |
| `plan.missionController.visualItems.\(item.index).cameraCalc` | Mission.swift | passthrough |
| `plan.missionController.visualItems.\(item.index).speedSection` | Mission.swift | passthrough |
| `settings.flyViewSettings.goToLocationRequiresConfirmInGuided` | MapClick.swift | passthrough |
| `settings.flyViewSettings.keepMapCenteredOnVehicle` | Fly.swift | passthrough |
| `vehicle` | Fly.swift,Instruments.swift,MapClick.swift,Mission.swift,Parameters.swift | passthrough |
| `vehicle.\(group)` | Instruments.swift | passthrough |
| `vehicle.gps` | Fly.swift | passthrough |
| `vehicle.parameterManager` | Parameters.swift | passthrough |
| `vehicle.terrain` | Fly.swift | passthrough |
