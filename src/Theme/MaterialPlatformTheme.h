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

#include <optional>

class MaterialPlatformTheme : public PlatformTheme
{
public:
    QString name() const final { return QStringLiteral("Material"); }
    Tones tones(Appearance appearance) const final;
    Shape shape() const final;
    QString controlStyle() const final;
    void applySystemChrome(Appearance appearance, const Tones &tones) final;

private:
    static Tones _baselineTones(Appearance appearance);
    static bool _applyDynamicColor(Appearance appearance, Tones &tones);

    mutable std::optional<Tones> _cache[2];
};
