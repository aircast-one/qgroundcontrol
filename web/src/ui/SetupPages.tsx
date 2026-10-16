import type { SetupPage } from './SetupView';

export const SENSORS = 'Sensors';
export const SAFETY_SETUP_PAGE = 'Safety';
export const RADIO = 'Radio';
export const REMOTE_SUPPORT = 'Remote Support';
export const MOTORS = 'Motors';
export const FLIGHT_MODES_PAGE = 'Flight Modes';
export const PX4_TUNING_SCREEN = 'px4Tuning';
export const NOT_SUPPORTED_SCREEN = 'notSupported';

export const headCanOpen = (_page: SetupPage | null, _name: string): boolean => false;
