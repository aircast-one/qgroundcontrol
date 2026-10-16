import { useQgcPath } from '@/bridge/QgcState';
import { openUri } from '@/kotlin/Platform';
import { ListItem, Text } from '@/m3';
import { md } from '@/theme/Colors';
import { ADVANCED_UI_PATH, advancedUiShown, toggleAdvancedUi } from '@/ui/AdvancedUi';
import type { HelpLink } from '@/ui/SettingsScreen';
import { SectionHeader } from '@/ui/SetupRow';

const VERSION_NAME: string = import.meta.env.VITE_APP_VERSION ?? 'dev';
const IS_64_BIT = false;

export const AboutPage = ({ links, className }: { links: readonly HelpLink[]; className?: string }) => {
  const advancedView = useQgcPath(ADVANCED_UI_PATH);
  return (
    <div className={`flex flex-col gap-1 py-2 ${className ?? ''}`}>
      <SectionHeader text="About" />
      <ListItem
        headlineContent="Aircast version"
        trailingContent={<Text typography="bodyMedium">{qgcVersion()}</Text>}
        onClick={advancedView !== null ? () => void toggleAdvancedUi(advancedUiShown(advancedView)) : undefined}
      />
      <SectionHeader text="Support" />
      {aboutLinks(links).map((link) => (
        <ListItem key={link.url} headlineContent={link.name} supportingContent={<Text typography="bodyMedium" color={md('primary')}>{link.host}</Text>} onClick={() => openUri(link.url)} />
      ))}
    </div>
  );
};

export const PRIVACY_POLICY_LINK: HelpLink = { name: 'Privacy policy', url: 'https://aircast.one/privacy/android', host: 'aircast.one' };

export const aboutLinks = (links: readonly HelpLink[]): HelpLink[] => [...links, PRIVACY_POLICY_LINK];

export const qgcVersion = (name: string = VERSION_NAME, is64Bit: boolean = IS_64_BIT): string => `${name} ${is64Bit ? '64' : '32'} bit`;
