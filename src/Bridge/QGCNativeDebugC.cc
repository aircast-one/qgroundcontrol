#include "QGCNativeDebugC.h"

namespace {
QGCNativeDebugHooks s_hooks{};
bool s_installed = false;
}

void qgc_native_debug_install(const QGCNativeDebugHooks *hooks)
{
    s_installed = hooks != nullptr;
    s_hooks = hooks ? *hooks : QGCNativeDebugHooks{};
}

const QGCNativeDebugHooks *qgc_native_debug_hooks(void)
{
    return s_installed ? &s_hooks : nullptr;
}
