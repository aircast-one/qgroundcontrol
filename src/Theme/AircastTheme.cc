#include "AircastTheme.h"

#ifdef Q_OS_ANDROID
#include "AndroidInterface.h"
#endif

namespace AircastTheme {

const AircastTokens::Scheme& scheme(Appearance appearance)
{
    return appearance == LightAppearance ? AircastTokens::light : AircastTokens::dark;
}

qreal controlRadiusRatio()
{
    return AircastTokens::radiusRem * 0.8;
}

QString fontFamily()
{
    return QStringLiteral("Geist");
}

QString monoFontFamily()
{
    return QStringLiteral("Geist Mono");
}

void applySystemChrome(Appearance appearance)
{
#ifdef Q_OS_ANDROID
    AndroidInterface::setSystemBarAppearance(appearance == LightAppearance);
#else
    Q_UNUSED(appearance);
#endif
}

}  // namespace AircastTheme
