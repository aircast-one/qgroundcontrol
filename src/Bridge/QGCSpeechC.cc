#include "QGCSpeechC.h"

#include <atomic>

namespace
{
std::atomic<QGCSpeechHandler> g_handler{nullptr};
}

void qgc_set_speech_handler(QGCSpeechHandler handler)
{
    g_handler.store(handler, std::memory_order_release);
}

void qgc_speak(const char *text, double volume)
{
    const QGCSpeechHandler handler = g_handler.load(std::memory_order_acquire);
    if (handler && text) {
        handler(text, volume);
    }
}

int qgc_speech_available(void)
{
    return g_handler.load(std::memory_order_acquire) != nullptr;
}
