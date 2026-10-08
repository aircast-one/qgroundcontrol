#include "QGCVideoC.h"

#include <algorithm>
#include <array>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <mutex>
#include <string>
#include <utility>
#include <vector>

#ifdef QGC_GST_STREAMING
#include <gst/app/gstappsink.h>
#include <gst/gst.h>
#include <gst/video/video.h>
#include <gst/video/videooverlay.h>
#endif

#ifndef QGC_VIDEO_FORMAT
#define QGC_VIDEO_FORMAT "BGRA"
#endif

namespace {

#ifdef QGC_GST_STREAMING
constexpr const char *kRecordingTee = "nativerec";
constexpr auto kRecordingDrain = std::chrono::seconds(3);

struct Recording {
    GstElement *queue = nullptr;
    GstElement *parse = nullptr;
    GstElement *mux = nullptr;
    GstElement *file = nullptr;
    GstElement *reparse = nullptr;
    GstPad *teePad = nullptr;
};
#endif

struct Channel {
    std::mutex frameMutex;
    std::vector<uint8_t> latestFrame;
    int frameWidth = 0;
    int frameHeight = 0;
    int frameStride = 0;
    int64_t frameCount = 0;
    const void *frameOwner = nullptr;
    std::atomic<int64_t> sourceBuffers{0};
    std::string lastError;
    std::mutex streamErrorMutex;
    std::string streamError;
    std::mutex overlayMutex;
    void *overlayWindow = nullptr;
#ifdef QGC_GST_STREAMING
    GstElement *pipeline = nullptr;
    GstElement *sink = nullptr;
    GstElement *overlaySink = nullptr;
    GstElement *appsink = nullptr;
    Recording recording;
    std::mutex drainMutex;
    std::condition_variable drained;
    bool drainedEos = false;
#endif

    void resetFrame()
    {
        latestFrame.clear();
        frameWidth = 0;
        frameHeight = 0;
        frameStride = 0;
        frameCount = 0;
    }

    void ownFrames(const void *owner)
    {
        const std::lock_guard<std::mutex> lock(frameMutex);
        frameOwner = owner;
        resetFrame();
    }

    void disownFrames(const void *owner)
    {
        const std::lock_guard<std::mutex> lock(frameMutex);
        frameOwner = (frameOwner == owner) ? nullptr : frameOwner;
        resetFrame();
    }

    void setStreamError(std::string text)
    {
        const std::lock_guard<std::mutex> lock(streamErrorMutex);
        streamError = std::move(text);
    }
};

std::array<Channel, QGC_VIDEO_CHANNELS> channels;
std::atomic<qgc_video_frame_callback> frameCallback{nullptr};
std::atomic<qgc_video_pipeline_callback> pipelineCallback{nullptr};

Channel *channelAt(int channel)
{
    return (channel >= 0 && channel < QGC_VIDEO_CHANNELS) ? &channels[static_cast<size_t>(channel)] : nullptr;
}

#ifdef QGC_GST_STREAMING
int indexOf(const Channel *channel)
{
    return static_cast<int>(channel - channels.data());
}

const char *muxerFor(int format)
{
    switch (format) {
    case 0:
        return "matroskamux";
    case 1:
        return "qtmux";
    case 2:
        return "mp4mux";
    default:
        return nullptr;
    }
}

const char *reparserFor(GstPad *pad)
{
    GstCaps *const caps = gst_pad_query_caps(pad, nullptr);
    const gchar *const name = gst_caps_is_empty(caps) ? "" : gst_structure_get_name(gst_caps_get_structure(caps, 0));
    const char *const parser = g_str_equal(name, "video/x-h264") ? "h264parse" : g_str_equal(name, "video/x-h265") ? "h265parse" : nullptr;
    gst_caps_unref(caps);
    return parser;
}

void onParsedPad(GstElement *, GstPad *pad, gpointer data)
{
    Channel *const channel = static_cast<Channel *>(data);
    Recording &recording = channel->recording;
    GstPad *const target = gst_element_request_pad_simple(recording.mux, "video_%u");
    if (!target) {
        return;
    }
    const char *const parser = reparserFor(pad);
    recording.reparse = parser ? gst_element_factory_make(parser, nullptr) : nullptr;
    if (recording.reparse && channel->pipeline) {
        gst_bin_add(GST_BIN(channel->pipeline), recording.reparse);
        gst_element_sync_state_with_parent(recording.reparse);
        GstPad *const reparseSink = gst_element_get_static_pad(recording.reparse, "sink");
        GstPad *const reparseSrc = gst_element_get_static_pad(recording.reparse, "src");
        gst_pad_link(pad, reparseSink);
        gst_pad_link(reparseSrc, target);
        gst_object_unref(reparseSink);
        gst_object_unref(reparseSrc);
    } else {
        gst_pad_link(pad, target);
    }
    gst_object_unref(target);
}

GstPadProbeReturn onRecordingEos(GstPad *, GstPadProbeInfo *info, gpointer data)
{
    if (GST_EVENT_TYPE(GST_PAD_PROBE_INFO_EVENT(info)) == GST_EVENT_EOS) {
        Channel *const channel = static_cast<Channel *>(data);
        const std::lock_guard<std::mutex> lock(channel->drainMutex);
        channel->drainedEos = true;
        channel->drained.notify_all();
        return GST_PAD_PROBE_DROP;
    }
    return GST_PAD_PROBE_OK;
}

void dropRecording(Channel &channel)
{
    Recording &recording = channel.recording;
    GstElement *const elements[] = {recording.queue, recording.parse, recording.reparse, recording.mux, recording.file};
    for (GstElement *element : elements) {
        if (element) {
            gst_element_set_state(element, GST_STATE_NULL);
            if (channel.pipeline) {
                gst_bin_remove(GST_BIN(channel.pipeline), element);
            }
        }
    }
    if (recording.teePad) {
        gst_object_unref(recording.teePad);
    }
    recording = Recording{};
}

constexpr const char *kWhepLatencyPrefix = "whep_latency_";

GstElement *findByFactory(GstBin *bin, const char *factory)
{
    GstIterator *const it = gst_bin_iterate_recurse(bin);
    GValue item = G_VALUE_INIT;
    GstElement *found = nullptr;
    while (!found && gst_iterator_next(it, &item) == GST_ITERATOR_OK) {
        GstElement *const element = GST_ELEMENT(g_value_get_object(&item));
        GstElementFactory *const made = gst_element_get_factory(element);
        if (made && g_strcmp0(gst_plugin_feature_get_name(GST_PLUGIN_FEATURE(made)), factory) == 0) {
            found = GST_ELEMENT(gst_object_ref(element));
        }
        g_value_reset(&item);
    }
    g_value_unset(&item);
    gst_iterator_free(it);
    return found;
}

constexpr gint64 kJitterAdaptIntervalUs = G_USEC_PER_SEC;
constexpr guint kJitterStepUpMs = 40;
constexpr guint kJitterStepDownMs = 20;
constexpr guint kJitterRtxMarginMs = 50;
constexpr guint kJitterCapMs = 500;
constexpr gint64 kJitterCleanWindowMs = 10000;

struct JitterAdapter
{
    GstElement *jitterBuffer;
    guint floorMs;
    guint currentMs;
    guint64 lost;
    guint64 late;
    gint64 lastCheckUs;
    gint64 cleanSinceUs;
};

guint adaptJitterLatencyMs(guint currentMs, guint floorMs, bool degraded, gint64 cleanForMs, guint rttMs)
{
    if (degraded) {
        const guint rtxTarget = (rttMs > 0) ? (rttMs + kJitterRtxMarginMs) : 0;
        return std::min(kJitterCapMs, std::max({currentMs + kJitterStepUpMs, rtxTarget, floorMs}));
    }
    if (cleanForMs >= kJitterCleanWindowMs && currentMs > floorMs) {
        return std::max(floorMs, (currentMs > kJitterStepDownMs) ? (currentMs - kJitterStepDownMs) : 0u);
    }
    return currentMs;
}

GstPadProbeReturn adaptJitterLatency(GstPad *, GstPadProbeInfo *, gpointer data)
{
    auto *const adapter = static_cast<JitterAdapter *>(data);
    const gint64 now = g_get_monotonic_time();
    if ((now - adapter->lastCheckUs) < kJitterAdaptIntervalUs) {
        return GST_PAD_PROBE_OK;
    }
    adapter->lastCheckUs = now;
    GstStructure *stats = nullptr;
    g_object_get(adapter->jitterBuffer, "stats", &stats, nullptr);
    if (!stats) {
        return GST_PAD_PROBE_OK;
    }
    guint64 lost = 0;
    guint64 late = 0;
    guint64 rttNs = 0;
    gst_structure_get_uint64(stats, "num-lost", &lost);
    gst_structure_get_uint64(stats, "num-late", &late);
    gst_structure_get_uint64(stats, "rtx-rtt", &rttNs);
    gst_structure_free(stats);
    const bool degraded = (lost > adapter->lost) || (late > adapter->late);
    adapter->lost = lost;
    adapter->late = late;
    if (degraded) {
        adapter->cleanSinceUs = now;
    }
    const guint next = adaptJitterLatencyMs(adapter->currentMs, adapter->floorMs, degraded, (now - adapter->cleanSinceUs) / 1000, static_cast<guint>(rttNs / GST_MSECOND));
    if (next != adapter->currentMs) {
        g_object_set(adapter->jitterBuffer, "latency", next, nullptr);
        adapter->currentMs = next;
        adapter->cleanSinceUs = now;
    }
    return GST_PAD_PROBE_OK;
}

void adaptNewJitterBuffer(GstElement *, GstElement *jitterBuffer, guint, guint, gpointer data)
{
    guint currentMs = 0;
    g_object_get(jitterBuffer, "latency", &currentMs, nullptr);
    const gint64 now = g_get_monotonic_time();
    auto *const adapter = new JitterAdapter{jitterBuffer, GPOINTER_TO_UINT(data), currentMs, 0, 0, now, now};
    g_object_set_data_full(G_OBJECT(jitterBuffer), "qgc-jitter-adapter", adapter, [](gpointer p) { delete static_cast<JitterAdapter *>(p); });
    if (GstPad *const src = gst_element_get_static_pad(jitterBuffer, "src")) {
        gst_pad_add_probe(src, GST_PAD_PROBE_TYPE_BUFFER, adaptJitterLatency, adapter, nullptr);
        gst_object_unref(src);
    }
}

void requestRetransmission(GstElement *, GObject *transceiver, gpointer)
{
    g_object_set(transceiver, "do-nack", TRUE, nullptr);
}

void singleThreadSoftwareHevc(GstBin *, GstBin *, GstElement *element, gpointer)
{
    GstElementFactory *const factory = gst_element_get_factory(element);
    if (factory && g_str_equal(GST_OBJECT_NAME(factory), "avdec_h265")) {
        g_object_set(element, "max-threads", 1, nullptr);
    }
}

void applyWhepLatency(GstBin *bin)
{
    GstElement *const source = findByFactory(bin, "whepsrc");
    if (!source) {
        return;
    }
    gchar *const name = gst_element_get_name(source);
    if (name && g_str_has_prefix(name, kWhepLatencyPrefix) && GST_IS_BIN(source)) {
        const guint latency = static_cast<guint>(g_ascii_strtoull(name + strlen(kWhepLatencyPrefix), nullptr, 10));
        if (GstElement *const webrtcbin = findByFactory(GST_BIN(source), "webrtcbin")) {
            g_object_set(webrtcbin, "latency", latency, nullptr);
            g_signal_connect(webrtcbin, "on-new-transceiver", G_CALLBACK(requestRetransmission), nullptr);
            if (GstElement *const rtpbin = findByFactory(GST_BIN(webrtcbin), "rtpbin")) {
                g_signal_connect(rtpbin, "new-jitterbuffer", G_CALLBACK(adaptNewJitterBuffer), GUINT_TO_POINTER(latency));
                gst_object_unref(rtpbin);
            }
            gst_object_unref(webrtcbin);
        }
    }
    g_free(name);
    gst_object_unref(source);
}

GstBusSyncReply onBusMessage(GstBus *, GstMessage *message, gpointer data)
{
    if (GST_MESSAGE_TYPE(message) == GST_MESSAGE_ERROR) {
        GError *error = nullptr;
        gchar *debug = nullptr;
        gst_message_parse_error(message, &error, &debug);
        static_cast<Channel *>(data)->setStreamError(std::string(error ? error->message : "") + (debug ? std::string(" ") + debug : ""));
        g_clear_error(&error);
        g_free(debug);
    }
    return GST_BUS_PASS;
}

GstPadProbeReturn onSourceBuffer(GstPad *, GstPadProbeInfo *, gpointer data)
{
    static_cast<Channel *>(data)->sourceBuffers.fetch_add(1, std::memory_order_relaxed);
    return GST_PAD_PROBE_OK;
}

GstFlowReturn onNewSample(GstAppSink *appsink, gpointer data)
{
    GstSample *const sample = gst_app_sink_pull_sample(appsink);
    if (!sample) {
        return GST_FLOW_OK;
    }

    Channel *const channel = static_cast<Channel *>(data);
    GstCaps *const caps = gst_sample_get_caps(sample);
    GstBuffer *const buffer = gst_sample_get_buffer(sample);
    GstVideoInfo info;
    if (caps && buffer && gst_video_info_from_caps(&info, caps)) {
        GstMapInfo map;
        if (gst_buffer_map(buffer, &map, GST_MAP_READ)) {
            const std::lock_guard<std::mutex> lock(channel->frameMutex);
            if (channel->frameOwner == appsink) {
                channel->frameWidth = GST_VIDEO_INFO_WIDTH(&info);
                channel->frameHeight = GST_VIDEO_INFO_HEIGHT(&info);
                channel->frameStride = static_cast<int>(GST_VIDEO_INFO_PLANE_STRIDE(&info, 0));
                channel->latestFrame.assign(map.data, map.data + map.size);
                channel->frameCount += 1;
                if (const qgc_video_frame_callback callback = frameCallback.load()) {
                    callback(indexOf(channel), channel->latestFrame.data(), channel->frameWidth, channel->frameHeight, channel->frameStride);
                }
            }
            gst_buffer_unmap(buffer, &map);
        }
    }

    gst_sample_unref(sample);
    return GST_FLOW_OK;
}

void adoptAppsink(Channel &channel, GstAppSink *appsink)
{
    channel.ownFrames(appsink);
    GstCaps *const caps = gst_caps_new_simple("video/x-raw", "format", G_TYPE_STRING, QGC_VIDEO_FORMAT, nullptr);
    gst_app_sink_set_caps(appsink, caps);
    gst_caps_unref(caps);
    gst_app_sink_set_max_buffers(appsink, 1);
    gst_app_sink_set_drop(appsink, TRUE);

    GstAppSinkCallbacks callbacks = {};
    callbacks.new_sample = onNewSample;
    gst_app_sink_set_callbacks(appsink, &callbacks, &channel, nullptr);
}

void dropAppsink(GstElement *appsink)
{
    GstAppSinkCallbacks none = {};
    gst_app_sink_set_callbacks(GST_APP_SINK(appsink), &none, nullptr, nullptr);
    gst_object_unref(appsink);
}

void releaseOverlay(GstElement *&sink)
{
    gst_video_overlay_set_window_handle(GST_VIDEO_OVERLAY(sink), 0);
    gst_object_unref(std::exchange(sink, nullptr));
}

void releaseElsewhere(const Channel &keeper, GstElement *element)
{
    std::for_each(channels.begin(), channels.end(), [&keeper, element](Channel &other) {
        if (&other == &keeper) {
            return;
        }
        {
            const std::lock_guard<std::mutex> lock(other.overlayMutex);
            if (other.overlaySink == element) {
                releaseOverlay(other.overlaySink);
            }
        }
        if (other.appsink == element) {
            other.disownFrames(element);
            gst_object_unref(std::exchange(other.appsink, nullptr));
        }
    });
}
#endif

}

int qgc_video_abi_version(void)
{
    return 2;
}

void qgc_video_set_frame_callback(qgc_video_frame_callback callback)
{
    frameCallback.store(callback);
}

void qgc_video_set_pipeline_callback(qgc_video_pipeline_callback callback)
{
    pipelineCallback.store(callback);
}

bool qgc_video_set_window(int channel, void *native_window)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return false;
    }
#ifdef QGC_GST_STREAMING
    const std::lock_guard<std::mutex> lock(target->overlayMutex);
    target->overlayWindow = native_window;
    if (target->overlaySink) {
        gst_video_overlay_set_window_handle(GST_VIDEO_OVERLAY(target->overlaySink),
                                            reinterpret_cast<guintptr>(native_window));
    }
    target->lastError.clear();
    return true;
#else
    (void)native_window;
    target->lastError = "this build has no GStreamer";
    return false;
#endif
}

bool qgc_video_attach_overlay(int channel, void *element)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return false;
    }
#ifdef QGC_GST_STREAMING
    if (!element || !GST_IS_VIDEO_OVERLAY(element)) {
        target->lastError = "the sink is not a video overlay";
        return false;
    }

    GstElement *const sink = GST_ELEMENT(element);
    releaseElsewhere(*target, sink);
    const std::lock_guard<std::mutex> lock(target->overlayMutex);
    if (target->overlaySink != sink) {
        if (target->overlaySink) {
            releaseOverlay(target->overlaySink);
        }
        target->overlaySink = GST_ELEMENT(gst_object_ref(sink));
        if (target->overlayWindow) {
            gst_video_overlay_set_window_handle(GST_VIDEO_OVERLAY(sink), reinterpret_cast<guintptr>(target->overlayWindow));
        }
    }
    target->lastError.clear();
    return true;
#else
    (void)element;
    target->lastError = "this build has no GStreamer";
    return false;
#endif
}

bool qgc_video_attach_appsink(int channel, void *appsink)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return false;
    }
#ifdef QGC_GST_STREAMING
    if (!appsink) {
        target->lastError = "no appsink given";
        return false;
    }

    GstElement *const sink = GST_ELEMENT(appsink);
    if (target->appsink != sink) {
        adoptAppsink(*target, GST_APP_SINK(sink));
        releaseElsewhere(*target, sink);
        if (target->appsink) {
            dropAppsink(target->appsink);
        }
        target->appsink = GST_ELEMENT(gst_object_ref(sink));
    }
    target->lastError.clear();
    return true;
#else
    (void)appsink;
    target->lastError = "this build has no GStreamer";
    return false;
#endif
}

void qgc_video_detach_appsink(int channel)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return;
    }
#ifdef QGC_GST_STREAMING
    target->disownFrames(target->appsink);
    if (target->appsink) {
        dropAppsink(std::exchange(target->appsink, nullptr));
    }
    const std::lock_guard<std::mutex> lock(target->overlayMutex);
    if (target->overlaySink) {
        releaseOverlay(target->overlaySink);
    }
#endif
}

bool qgc_video_available(void)
{
#ifdef QGC_GST_STREAMING
    return true;
#else
    return false;
#endif
}

bool qgc_video_start(int channel, const char *pipelineDescription)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return false;
    }
#ifdef QGC_GST_STREAMING
    if (!pipelineDescription) {
        target->lastError = "no pipeline given";
        return false;
    }

    qgc_video_stop(channel);

    if (!gst_is_initialized()) {
        gst_init(nullptr, nullptr);
    }

    GError *error = nullptr;
    target->pipeline = gst_parse_launch_full(pipelineDescription, nullptr, GST_PARSE_FLAG_FATAL_ERRORS, &error);
    if (!target->pipeline) {
        target->lastError = error ? error->message : "gst_parse_launch failed";
        if (error) {
            g_error_free(error);
        }
        return false;
    }
    if (error) {
        g_error_free(error);
    }

    target->sink = gst_bin_get_by_name(GST_BIN(target->pipeline), "nativesink");
    if (!target->sink) {
        target->lastError = "the pipeline has no appsink named nativesink";
        qgc_video_stop(channel);
        return false;
    }

    applyWhepLatency(GST_BIN(target->pipeline));
    g_signal_connect(target->pipeline, "deep-element-added", G_CALLBACK(singleThreadSoftwareHevc), nullptr);

    if (GstBus *const bus = gst_element_get_bus(target->pipeline)) {
        gst_bus_set_sync_handler(bus, onBusMessage, target, nullptr);
        gst_object_unref(bus);
    }

    if (GstElement *const tee = gst_bin_get_by_name(GST_BIN(target->pipeline), kRecordingTee)) {
        if (GstPad *const teeSink = gst_element_get_static_pad(tee, "sink")) {
            gst_pad_add_probe(teeSink, static_cast<GstPadProbeType>(GST_PAD_PROBE_TYPE_BUFFER | GST_PAD_PROBE_TYPE_BUFFER_LIST), onSourceBuffer, target, nullptr);
            gst_object_unref(teeSink);
        }
        gst_object_unref(tee);
    }

    adoptAppsink(*target, GST_APP_SINK(target->sink));

    if (const qgc_video_pipeline_callback callback = pipelineCallback.load()) {
        callback(channel, target->pipeline);
    }

    if (gst_element_set_state(target->pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE) {
        target->lastError = "the pipeline refused to play";
        qgc_video_stop(channel);
        return false;
    }

    target->lastError.clear();
    return true;
#else
    (void)pipelineDescription;
    target->lastError = "this build has no GStreamer";
    return false;
#endif
}

bool qgc_video_start_recording(int channel, const char *file, int format)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return false;
    }
#ifdef QGC_GST_STREAMING
    Recording &recording = target->recording;
    const char *const muxer = muxerFor(format);
    if (!file || !*file || !muxer) {
        target->lastError = "a recording needs a file and a known format";
        return false;
    }
    if (!target->pipeline) {
        target->lastError = "no video is playing";
        return false;
    }
    if (recording.file) {
        target->lastError = "already recording";
        return false;
    }
    GstElement *const tee = gst_bin_get_by_name(GST_BIN(target->pipeline), kRecordingTee);
    if (!tee) {
        target->lastError = "the pipeline has no recording tee";
        return false;
    }
    recording.queue = gst_element_factory_make("queue", nullptr);
    recording.parse = gst_element_factory_make("parsebin", nullptr);
    recording.mux = gst_element_factory_make(muxer, nullptr);
    recording.file = gst_element_factory_make("filesink", nullptr);
    if (!recording.queue || !recording.parse || !recording.mux || !recording.file) {
        target->lastError = std::string("this build cannot record with ") + muxer;
        GstElement *const made[] = {recording.queue, recording.parse, recording.mux, recording.file};
        for (GstElement *element : made) {
            if (element) {
                gst_object_unref(element);
            }
        }
        recording = Recording{};
        gst_object_unref(tee);
        return false;
    }
    g_object_set(recording.file, "location", file, "async", FALSE, nullptr);
    gst_bin_add_many(GST_BIN(target->pipeline), recording.queue, recording.parse, recording.mux, recording.file, nullptr);
    g_signal_connect(recording.parse, "pad-added", G_CALLBACK(onParsedPad), target);
    const bool linked = gst_element_link(recording.queue, recording.parse) && gst_element_link(recording.mux, recording.file);
    GstPad *const fileSinkPad = gst_element_get_static_pad(recording.file, "sink");
    gst_pad_add_probe(fileSinkPad, GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM, onRecordingEos, target, nullptr);
    gst_object_unref(fileSinkPad);
    gst_element_sync_state_with_parent(recording.file);
    gst_element_sync_state_with_parent(recording.mux);
    gst_element_sync_state_with_parent(recording.parse);
    gst_element_sync_state_with_parent(recording.queue);
    recording.teePad = gst_element_request_pad_simple(tee, "src_%u");
    GstPad *const queueSink = gst_element_get_static_pad(recording.queue, "sink");
    const bool attached = linked && recording.teePad && gst_pad_link(recording.teePad, queueSink) == GST_PAD_LINK_OK;
    gst_object_unref(queueSink);
    gst_object_unref(tee);
    if (!attached) {
        target->lastError = "the recording branch would not link";
        qgc_video_stop_recording(channel);
        return false;
    }
    target->lastError.clear();
    return true;
#else
    (void)file;
    (void)format;
    target->lastError = "this build has no GStreamer";
    return false;
#endif
}

void qgc_video_stop_recording(int channel)
{
#ifdef QGC_GST_STREAMING
    Channel *const target = channelAt(channel);
    if (!target || !target->recording.queue) {
        return;
    }
    Recording &recording = target->recording;
    GstElement *const tee = target->pipeline ? gst_bin_get_by_name(GST_BIN(target->pipeline), kRecordingTee) : nullptr;
    GstPad *const queueSink = gst_element_get_static_pad(recording.queue, "sink");
    if (recording.teePad) {
        gst_pad_unlink(recording.teePad, queueSink);
        if (tee) {
            gst_element_release_request_pad(tee, recording.teePad);
        }
    }
    {
        const std::lock_guard<std::mutex> lock(target->drainMutex);
        target->drainedEos = false;
    }
    gst_pad_send_event(queueSink, gst_event_new_eos());
    gst_object_unref(queueSink);
    {
        std::unique_lock<std::mutex> lock(target->drainMutex);
        target->drained.wait_for(lock, kRecordingDrain, [target] { return target->drainedEos; });
    }
    if (tee) {
        gst_object_unref(tee);
    }
    dropRecording(*target);
#else
    (void)channel;
#endif
}

bool qgc_video_recording(int channel)
{
#ifdef QGC_GST_STREAMING
    const Channel *const target = channelAt(channel);
    return target && target->recording.file != nullptr;
#else
    (void)channel;
    return false;
#endif
}

void qgc_video_stop(int channel)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return;
    }
#ifdef QGC_GST_STREAMING
    qgc_video_stop_recording(channel);
    if (target->pipeline) {
        gst_element_set_state(target->pipeline, GST_STATE_NULL);
    }
    target->disownFrames(target->sink);
    if (target->sink) {
        gst_object_unref(target->sink);
        target->sink = nullptr;
    }
    if (target->pipeline) {
        gst_object_unref(target->pipeline);
        target->pipeline = nullptr;
    }
    target->sourceBuffers.store(0, std::memory_order_relaxed);
#endif
    target->setStreamError({});
}

bool qgc_video_running(int channel)
{
#ifdef QGC_GST_STREAMING
    const Channel *const target = channelAt(channel);
    return target && target->pipeline != nullptr;
#else
    (void)channel;
    return false;
#endif
}

int qgc_video_width(int channel)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return 0;
    }
    const std::lock_guard<std::mutex> lock(target->frameMutex);
    return target->frameWidth;
}

int qgc_video_height(int channel)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return 0;
    }
    const std::lock_guard<std::mutex> lock(target->frameMutex);
    return target->frameHeight;
}

int64_t qgc_video_frames(int channel)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return 0;
    }
    const std::lock_guard<std::mutex> lock(target->frameMutex);
    return target->frameCount;
}

int64_t qgc_video_source_buffers(int channel)
{
    const Channel *const target = channelAt(channel);
    return target ? target->sourceBuffers.load(std::memory_order_relaxed) : 0;
}

const char *qgc_video_last_error(int channel)
{
    const Channel *const target = channelAt(channel);
    return target ? target->lastError.c_str() : "no such video channel";
}

const char *qgc_video_stream_error(int channel)
{
    thread_local std::string copy;
    Channel *const target = channelAt(channel);
    if (!target) {
        return "";
    }
    const std::lock_guard<std::mutex> lock(target->streamErrorMutex);
    copy = target->streamError;
    return copy.c_str();
}

bool qgc_video_copy_frame(int channel, void *destination, int capacity, int *width, int *height, int *stride)
{
    Channel *const target = channelAt(channel);
    if (!target) {
        return false;
    }
    const std::lock_guard<std::mutex> lock(target->frameMutex);
    if (target->latestFrame.empty() || !destination || capacity < static_cast<int>(target->latestFrame.size())) {
        return false;
    }
    memcpy(destination, target->latestFrame.data(), target->latestFrame.size());
    if (width) {
        *width = target->frameWidth;
    }
    if (height) {
        *height = target->frameHeight;
    }
    if (stride) {
        *stride = target->frameStride;
    }
    return true;
}
