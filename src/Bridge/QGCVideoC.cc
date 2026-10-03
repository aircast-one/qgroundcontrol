#include "QGCVideoC.h"

#include <algorithm>
#include <atomic>
#include <chrono>
#include <condition_variable>
#include <cstring>
#include <mutex>
#include <string>
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

std::mutex frameMutex;
qgc_video_frame_callback frameCallback = nullptr;
qgc_video_pipeline_callback pipelineCallback = nullptr;
std::mutex overlayMutex;
void *overlayWindow = nullptr;
std::vector<uint8_t> latestFrame;
int frameWidth = 0;
int frameHeight = 0;
int frameStride = 0;
int64_t frameCount = 0;
std::atomic<int64_t> sourceBuffers{0};
std::string lastError;

#ifdef QGC_GST_STREAMING
GstElement *pipeline = nullptr;
GstElement *sink = nullptr;
GstElement *overlaySink = nullptr;

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

Recording recording;
std::mutex drainMutex;
std::condition_variable drained;
bool drainedEos = false;

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

void onParsedPad(GstElement *, GstPad *pad, gpointer mux)
{
    GstPad *const target = gst_element_request_pad_simple(GST_ELEMENT(mux), "video_%u");
    if (!target) {
        return;
    }
    const char *const parser = reparserFor(pad);
    recording.reparse = parser ? gst_element_factory_make(parser, nullptr) : nullptr;
    if (recording.reparse && pipeline) {
        gst_bin_add(GST_BIN(pipeline), recording.reparse);
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

GstPadProbeReturn onRecordingEos(GstPad *, GstPadProbeInfo *info, gpointer)
{
    if (GST_EVENT_TYPE(GST_PAD_PROBE_INFO_EVENT(info)) == GST_EVENT_EOS) {
        const std::lock_guard<std::mutex> lock(drainMutex);
        drainedEos = true;
        drained.notify_all();
        return GST_PAD_PROBE_DROP;
    }
    return GST_PAD_PROBE_OK;
}

void dropRecording()
{
    GstElement *const elements[] = {recording.queue, recording.parse, recording.reparse, recording.mux, recording.file};
    for (GstElement *element : elements) {
        if (element) {
            gst_element_set_state(element, GST_STATE_NULL);
            if (pipeline) {
                gst_bin_remove(GST_BIN(pipeline), element);
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

GstPadProbeReturn onSourceBuffer(GstPad *, GstPadProbeInfo *, gpointer)
{
    sourceBuffers.fetch_add(1, std::memory_order_relaxed);
    return GST_PAD_PROBE_OK;
}

GstFlowReturn onNewSample(GstAppSink *appsink, gpointer)
{
    GstSample *const sample = gst_app_sink_pull_sample(appsink);
    if (!sample) {
        return GST_FLOW_OK;
    }

    GstCaps *const caps = gst_sample_get_caps(sample);
    GstBuffer *const buffer = gst_sample_get_buffer(sample);
    GstVideoInfo info;
    if (caps && buffer && gst_video_info_from_caps(&info, caps)) {
        GstMapInfo map;
        if (gst_buffer_map(buffer, &map, GST_MAP_READ)) {
            const std::lock_guard<std::mutex> lock(frameMutex);
            frameWidth = GST_VIDEO_INFO_WIDTH(&info);
            frameHeight = GST_VIDEO_INFO_HEIGHT(&info);
            frameStride = static_cast<int>(GST_VIDEO_INFO_PLANE_STRIDE(&info, 0));
            latestFrame.assign(map.data, map.data + map.size);
            frameCount += 1;
            if (frameCallback) {
                frameCallback(latestFrame.data(), frameWidth, frameHeight, frameStride);
            }
            gst_buffer_unmap(buffer, &map);
        }
    }

    gst_sample_unref(sample);
    return GST_FLOW_OK;
}
#endif

} // namespace

void qgc_video_set_frame_callback(qgc_video_frame_callback callback)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    frameCallback = callback;
}

void qgc_video_set_pipeline_callback(qgc_video_pipeline_callback callback)
{
    pipelineCallback = callback;
}

bool qgc_video_set_window(void *native_window)
{
#ifdef QGC_GST_STREAMING
    const std::lock_guard<std::mutex> lock(overlayMutex);
    overlayWindow = native_window;
    if (overlaySink) {
        gst_video_overlay_set_window_handle(GST_VIDEO_OVERLAY(overlaySink),
                                            reinterpret_cast<guintptr>(native_window));
    }
    lastError.clear();
    return true;
#else
    (void)native_window;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

bool qgc_video_attach_overlay(void *element)
{
#ifdef QGC_GST_STREAMING
    if (!element || !GST_IS_VIDEO_OVERLAY(element)) {
        lastError = "the sink is not a video overlay";
        return false;
    }

    const std::lock_guard<std::mutex> lock(overlayMutex);
    if (overlaySink) {
        gst_object_unref(overlaySink);
    }
    overlaySink = GST_ELEMENT(gst_object_ref(element));
    if (overlayWindow) {
        gst_video_overlay_set_window_handle(GST_VIDEO_OVERLAY(overlaySink),
                                            reinterpret_cast<guintptr>(overlayWindow));
    }
    lastError.clear();
    return true;
#else
    (void)element;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

bool qgc_video_attach_appsink(void *appsink)
{
#ifdef QGC_GST_STREAMING
    if (!appsink) {
        lastError = "no appsink given";
        return false;
    }

    GstAppSink *const adopted = GST_APP_SINK(appsink);
    GstCaps *const caps = gst_caps_new_simple("video/x-raw", "format", G_TYPE_STRING, QGC_VIDEO_FORMAT, nullptr);
    gst_app_sink_set_caps(adopted, caps);
    gst_caps_unref(caps);
    gst_app_sink_set_max_buffers(adopted, 1);
    gst_app_sink_set_drop(adopted, TRUE);

    GstAppSinkCallbacks callbacks = {};
    callbacks.new_sample = onNewSample;
    gst_app_sink_set_callbacks(adopted, &callbacks, nullptr, nullptr);

    {
        const std::lock_guard<std::mutex> lock(frameMutex);
        latestFrame.clear();
        frameWidth = 0;
        frameHeight = 0;
        frameStride = 0;
        frameCount = 0;
    }

    lastError.clear();
    return true;
#else
    (void)appsink;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

void qgc_video_detach_appsink(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    latestFrame.clear();
    frameWidth = 0;
    frameHeight = 0;
    frameStride = 0;
    frameCount = 0;
}

bool qgc_video_available(void)
{
#ifdef QGC_GST_STREAMING
    return true;
#else
    return false;
#endif
}

bool qgc_video_start(const char *pipelineDescription)
{
#ifdef QGC_GST_STREAMING
    if (!pipelineDescription) {
        lastError = "no pipeline given";
        return false;
    }

    qgc_video_stop();

    if (!gst_is_initialized()) {
        gst_init(nullptr, nullptr);
    }

    GError *error = nullptr;
    pipeline = gst_parse_launch(pipelineDescription, &error);
    if (!pipeline) {
        lastError = error ? error->message : "gst_parse_launch failed";
        if (error) {
            g_error_free(error);
        }
        return false;
    }
    if (error) {
        g_error_free(error);
    }

    sink = gst_bin_get_by_name(GST_BIN(pipeline), "nativesink");
    if (!sink) {
        lastError = "the pipeline has no appsink named nativesink";
        qgc_video_stop();
        return false;
    }

    applyWhepLatency(GST_BIN(pipeline));

    if (GstElement *const tee = gst_bin_get_by_name(GST_BIN(pipeline), kRecordingTee)) {
        if (GstPad *const teeSink = gst_element_get_static_pad(tee, "sink")) {
            gst_pad_add_probe(teeSink, static_cast<GstPadProbeType>(GST_PAD_PROBE_TYPE_BUFFER | GST_PAD_PROBE_TYPE_BUFFER_LIST), onSourceBuffer, nullptr, nullptr);
            gst_object_unref(teeSink);
        }
        gst_object_unref(tee);
    }

    GstCaps *const caps = gst_caps_new_simple("video/x-raw", "format", G_TYPE_STRING, QGC_VIDEO_FORMAT, nullptr);
    gst_app_sink_set_caps(GST_APP_SINK(sink), caps);
    gst_caps_unref(caps);
    gst_app_sink_set_max_buffers(GST_APP_SINK(sink), 1);
    gst_app_sink_set_drop(GST_APP_SINK(sink), TRUE);

    GstAppSinkCallbacks callbacks = {};
    callbacks.new_sample = onNewSample;
    gst_app_sink_set_callbacks(GST_APP_SINK(sink), &callbacks, nullptr, nullptr);

    if (pipelineCallback) {
        pipelineCallback(pipeline);
    }

    if (gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE) {
        lastError = "the pipeline refused to play";
        qgc_video_stop();
        return false;
    }

    lastError.clear();
    return true;
#else
    (void)pipelineDescription;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

bool qgc_video_start_recording(const char *file, int format)
{
#ifdef QGC_GST_STREAMING
    const char *const muxer = muxerFor(format);
    if (!file || !*file || !muxer) {
        lastError = "a recording needs a file and a known format";
        return false;
    }
    if (!pipeline) {
        lastError = "no video is playing";
        return false;
    }
    if (recording.file) {
        lastError = "already recording";
        return false;
    }
    GstElement *const tee = gst_bin_get_by_name(GST_BIN(pipeline), kRecordingTee);
    if (!tee) {
        lastError = "the pipeline has no recording tee";
        return false;
    }
    recording.queue = gst_element_factory_make("queue", nullptr);
    recording.parse = gst_element_factory_make("parsebin", nullptr);
    recording.mux = gst_element_factory_make(muxer, nullptr);
    recording.file = gst_element_factory_make("filesink", nullptr);
    if (!recording.queue || !recording.parse || !recording.mux || !recording.file) {
        lastError = std::string("this build cannot record with ") + muxer;
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
    gst_bin_add_many(GST_BIN(pipeline), recording.queue, recording.parse, recording.mux, recording.file, nullptr);
    g_signal_connect(recording.parse, "pad-added", G_CALLBACK(onParsedPad), recording.mux);
    const bool linked = gst_element_link(recording.queue, recording.parse) && gst_element_link(recording.mux, recording.file);
    GstPad *const fileSinkPad = gst_element_get_static_pad(recording.file, "sink");
    gst_pad_add_probe(fileSinkPad, GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM, onRecordingEos, nullptr, nullptr);
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
        lastError = "the recording branch would not link";
        qgc_video_stop_recording();
        return false;
    }
    lastError.clear();
    return true;
#else
    (void)file;
    (void)format;
    lastError = "this build has no GStreamer";
    return false;
#endif
}

void qgc_video_stop_recording(void)
{
#ifdef QGC_GST_STREAMING
    if (!recording.queue) {
        return;
    }
    GstElement *const tee = pipeline ? gst_bin_get_by_name(GST_BIN(pipeline), kRecordingTee) : nullptr;
    GstPad *const queueSink = gst_element_get_static_pad(recording.queue, "sink");
    if (recording.teePad) {
        gst_pad_unlink(recording.teePad, queueSink);
        if (tee) {
            gst_element_release_request_pad(tee, recording.teePad);
        }
    }
    {
        const std::lock_guard<std::mutex> lock(drainMutex);
        drainedEos = false;
    }
    gst_pad_send_event(queueSink, gst_event_new_eos());
    gst_object_unref(queueSink);
    {
        std::unique_lock<std::mutex> lock(drainMutex);
        drained.wait_for(lock, kRecordingDrain, [] { return drainedEos; });
    }
    if (tee) {
        gst_object_unref(tee);
    }
    dropRecording();
#endif
}

bool qgc_video_recording(void)
{
#ifdef QGC_GST_STREAMING
    return recording.file != nullptr;
#else
    return false;
#endif
}

void qgc_video_stop(void)
{
#ifdef QGC_GST_STREAMING
    qgc_video_stop_recording();
    if (pipeline) {
        gst_element_set_state(pipeline, GST_STATE_NULL);
    }
    if (sink) {
        gst_object_unref(sink);
        sink = nullptr;
    }
    if (pipeline) {
        gst_object_unref(pipeline);
        pipeline = nullptr;
    }
    sourceBuffers.store(0, std::memory_order_relaxed);
#endif
    const std::lock_guard<std::mutex> lock(frameMutex);
    latestFrame.clear();
    frameWidth = 0;
    frameHeight = 0;
    frameStride = 0;
    frameCount = 0;
}

bool qgc_video_running(void)
{
#ifdef QGC_GST_STREAMING
    return pipeline != nullptr;
#else
    return false;
#endif
}

int qgc_video_width(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    return frameWidth;
}

int qgc_video_height(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    return frameHeight;
}

int64_t qgc_video_frames(void)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    return frameCount;
}

int64_t qgc_video_source_buffers(void)
{
#ifdef QGC_GST_STREAMING
    return sourceBuffers.load(std::memory_order_relaxed);
#else
    return 0;
#endif
}

const char *qgc_video_last_error(void)
{
    return lastError.c_str();
}

bool qgc_video_copy_frame(void *destination, int capacity, int *width, int *height, int *stride)
{
    const std::lock_guard<std::mutex> lock(frameMutex);
    if (latestFrame.empty() || !destination || capacity < static_cast<int>(latestFrame.size())) {
        return false;
    }
    memcpy(destination, latestFrame.data(), latestFrame.size());
    if (width) {
        *width = frameWidth;
    }
    if (height) {
        *height = frameHeight;
    }
    if (stride) {
        *stride = frameStride;
    }
    return true;
}
