import { ACTUATORS_SCREEN } from '@/ui/ActuatorsScreen';
import { APM_AIRFRAME_SCREEN } from '@/ui/ApmAirframeScreen';
import { APM_FOLLOW_SCREEN } from '@/ui/ApmFollowScreen';
import { APM_SERVOS_SCREEN } from '@/ui/ApmServosScreen';
import { APM_SUB_FRAME_SCREEN } from '@/ui/ApmSubFrameScreen';
import { APM_SUB_MOTORS_SCREEN } from '@/ui/ApmSubMotorsScreen';
import { ESP_BRIDGE_SCREEN } from '@/ui/EspBridgeScreen';
import { JOYSTICK_SCREEN } from '@/ui/JoystickScreen';
import { OPTICAL_FLOW_SCREEN } from '@/ui/OpticalFlowScreen';
import { PX4_AIRFRAME_SCREEN } from '@/ui/Px4AirframeScreen';
import { SCRIPTING_SCREEN } from '@/ui/ScriptingScreen';
import type { SetupComponent } from '@/ui/SetupScreen';
import type { SetupPage } from '@/ui/SetupView';
import { SYSLINK_SCREEN } from '@/ui/SyslinkScreen';

export const SENSORS = 'Sensors';
export const SAFETY_SETUP_PAGE = 'Safety';
export const RADIO = 'Radio';
export const REMOTE_SUPPORT = 'Remote Support';
export const MOTORS = 'Motors';
export const FLIGHT_MODES_PAGE = 'Flight Modes';
export const PX4_TUNING_SCREEN = 'px4Tuning';
export const NOT_SUPPORTED_SCREEN = 'notSupported';

export const KNOWN_PAGES: ReadonlyMap<string, string> = new Map([
  ['sensors', SENSORS],
  ['radio', RADIO],
  ['flightModes', FLIGHT_MODES_PAGE],
]);

export const headPage = (component: SetupComponent): string => (component.known === null ? null : KNOWN_PAGES.get(component.known)) ?? component.name;

export const headCanOpen = (page: SetupPage | null, name: string): boolean =>
  page !== null &&
  ([SENSORS, RADIO, REMOTE_SUPPORT, MOTORS].includes(name) ||
    page.parameterSections ||
    [
      PX4_TUNING_SCREEN,
      PX4_AIRFRAME_SCREEN,
      ACTUATORS_SCREEN,
      APM_SERVOS_SCREEN,
      APM_FOLLOW_SCREEN,
      SCRIPTING_SCREEN,
      JOYSTICK_SCREEN,
      ESP_BRIDGE_SCREEN,
      APM_SUB_FRAME_SCREEN,
      APM_AIRFRAME_SCREEN,
      SYSLINK_SCREEN,
      APM_SUB_MOTORS_SCREEN,
      OPTICAL_FLOW_SCREEN,
      NOT_SUPPORTED_SCREEN,
    ].includes(page.screen));
