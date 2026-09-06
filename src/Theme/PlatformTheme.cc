/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#include "PlatformTheme.h"
#include "ApplePlatformTheme.h"
#include "FluentPlatformTheme.h"
#include "MaterialPlatformTheme.h"

#include <QtGui/QFontDatabase>

PlatformTheme *PlatformTheme::instance()
{
#if defined(Q_OS_MACOS) || defined(Q_OS_IOS)
    static ApplePlatformTheme theme;
#elif defined(Q_OS_ANDROID)
    static MaterialPlatformTheme theme;
#else
    static FluentPlatformTheme theme;
#endif
    return &theme;
}

QString PlatformTheme::controlStyle() const
{
    return QStringLiteral("Basic");
}

QString PlatformTheme::fontFamily() const
{
    const QString family = QFontDatabase::systemFont(QFontDatabase::GeneralFont).family();
    return family.isEmpty() ? QStringLiteral("Open Sans") : family;
}

void PlatformTheme::applySystemChrome(Appearance appearance, const Tones &tones)
{
    Q_UNUSED(appearance);
    Q_UNUSED(tones);
}
