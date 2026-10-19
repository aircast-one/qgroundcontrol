#pragma once

#include <QtCore/QString>

#include "AircastTokens.h"

namespace AircastTheme {

enum Appearance
{
    LightAppearance,
    DarkAppearance
};

const AircastTokens::Scheme& scheme(Appearance appearance);
qreal controlRadiusRatio();
QString fontFamily();
QString monoFontFamily();
void applySystemChrome(Appearance appearance);

}  // namespace AircastTheme
