/****************************************************************************
 *
 * (c) 2009-2024 QGROUNDCONTROL PROJECT <http://www.qgroundcontrol.org>
 *
 * QGroundControl is licensed according to the terms in the file
 * COPYING.md in the root of the source code directory.
 *
 ****************************************************************************/

#pragma once

#include "PlatformTheme.h"

class ApplePlatformTheme : public PlatformTheme
{
public:
    QString name() const final { return QStringLiteral("Apple"); }
    Tones tones(Appearance appearance) const final;
    Shape shape() const final;
};
