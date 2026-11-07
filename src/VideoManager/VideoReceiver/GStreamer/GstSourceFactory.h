#pragma once

#include <QtCore/QString>
#include <gst/gstelement.h>

namespace GStreamer::SourceFactory {

enum class JitterBuffer
{
    None,
    DropOnLatency,
    Buffered,
};

struct Config
{
    JitterBuffer jitterBuffer = JitterBuffer::DropOnLatency;
    int latencyMs = 80;
    bool doRetransmission = true;
    QString authToken;
};

GstElement* create(const QString& uri, const Config& config);

bool isWhepUri(const QString& uri);

guint adaptJitterLatencyMs(guint currentMs, guint floorMs, bool degraded, guint rttMs);

}  // namespace GStreamer::SourceFactory
