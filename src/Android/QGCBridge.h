#pragma once

namespace QGCBridge
{
    void setNativeMethods();
    bool setSystemBarAppearance(bool lightBars);

    constexpr const char *kJniQGCBridgeClassName = "org/mavlink/qgroundcontrol/QGCBridge";
};
