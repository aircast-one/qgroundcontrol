import Foundation

let SENSORS = "Sensors"
let SAFETY_SETUP_PAGE = "Safety"
let RADIO = "Radio"
let REMOTE_SUPPORT = "Remote Support"
let MOTORS = "Motors"
let FLIGHT_MODES_PAGE = "Flight Modes"
let PX4_TUNING_SCREEN = "px4Tuning"
let NOT_SUPPORTED_SCREEN = "notSupported"

let KNOWN_PAGES: [String: String] = [
    "sensors": SENSORS,
    "radio": RADIO,
    "flightModes": FLIGHT_MODES_PAGE,
]

func headPage(_ component: SetupComponent) -> String {
    component.known.flatMap { KNOWN_PAGES[$0] } ?? component.name
}

private var headScreens: [String] {
    [
        PX4_TUNING_SCREEN, PX4_AIRFRAME_SCREEN, ACTUATORS_SCREEN, APM_SERVOS_SCREEN, APM_FOLLOW_SCREEN, SCRIPTING_SCREEN,
        JOYSTICK_SCREEN, ESP_BRIDGE_SCREEN, APM_SUB_FRAME_SCREEN, APM_AIRFRAME_SCREEN, SYSLINK_SCREEN, APM_SUB_MOTORS_SCREEN,
        OPTICAL_FLOW_SCREEN, NOT_SUPPORTED_SCREEN,
    ]
}

func headCanOpen(_ page: SetupPage?, _ name: String) -> Bool {
    guard let page else { return false }
    return [SENSORS, RADIO, REMOTE_SUPPORT, MOTORS].contains(name) || page.parameterSections || headScreens.contains(page.screen)
}
