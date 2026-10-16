const UNKNOWN_SSID = '<unknown ssid>';

const unquoted = (text: string): string => (text.length >= 2 && text.startsWith('"') && text.endsWith('"') ? text.slice(1, -1) : text);

export const phoneSsid = (raw: string | null): string | null => {
  const ssid = raw === null ? null : unquoted(raw.trim()).trim();
  return ssid !== null && ssid !== '' && ssid !== UNKNOWN_SSID && ssid !== '0x' ? ssid : null;
};

export type PhoneWifi = { ssid: string | null; secured: boolean | null };

export const WifiInfo = { SECURITY_TYPE_UNKNOWN: -1, SECURITY_TYPE_OPEN: 0, SECURITY_TYPE_WEP: 1, SECURITY_TYPE_PSK: 2, SECURITY_TYPE_OWE: 6 } as const;

export const securedFrom = (securityType: number): boolean | null => {
  if (securityType === WifiInfo.SECURITY_TYPE_UNKNOWN) return null;
  return !(securityType === WifiInfo.SECURITY_TYPE_OPEN || securityType === WifiInfo.SECURITY_TYPE_OWE);
};

const UNREADABLE_WIFI: PhoneWifi = { ssid: null, secured: null };

export const useRememberPhoneWifi = (): PhoneWifi => UNREADABLE_WIFI;

type NetworkInformation = { type?: string; saveData?: boolean };

export const phoneIsOnMeteredNetwork = (): boolean => {
  const connection = (navigator as Navigator & { connection?: NetworkInformation }).connection;
  return connection?.type === 'cellular' || connection?.saveData === true;
};
