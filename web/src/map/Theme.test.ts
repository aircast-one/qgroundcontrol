import { describe, expect, it } from 'vitest';
import { AircastTypography } from './Theme';

describe('TypographyTokensTest', () => {
  it('every text style carries the Penpot zero letter spacing and its size', () => {
    const styles = [
      AircastTypography.displayLarge, AircastTypography.displayMedium, AircastTypography.displaySmall,
      AircastTypography.headlineLarge, AircastTypography.headlineMedium, AircastTypography.headlineSmall,
      AircastTypography.titleLarge, AircastTypography.titleMedium, AircastTypography.titleSmall,
      AircastTypography.bodyLarge, AircastTypography.bodyMedium, AircastTypography.bodySmall,
      AircastTypography.labelLarge, AircastTypography.labelMedium, AircastTypography.labelSmall,
    ];
    expect(styles.map((it) => it.letterSpacing)).toEqual(styles.map(() => 0));
    expect([AircastTypography.headlineSmall, AircastTypography.titleLarge, AircastTypography.bodyLarge, AircastTypography.bodyMedium, AircastTypography.labelMedium, AircastTypography.labelSmall].map((it) => it.fontSize)).toEqual([24, 22, 16, 14, 12, 11]);
  });
});
