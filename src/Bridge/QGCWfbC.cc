#include "QGCWfbC.h"

#include "gui_interface.h"
#include "wfbng_link.h"

#include <algorithm>
#include <atomic>
#include <cstdlib>
#include <cstring>
#include <exception>
#include <memory>
#include <mutex>
#include <optional>
#include <string>
#include <vector>

namespace {

constexpr int kVideoPort = 5600;

struct State {
    std::mutex lock;
    std::unique_ptr<WfbngLink> link;
    std::optional<std::string> codec;
    std::atomic<long long> rtp{0};
    std::atomic<bool> stopped{false};
};

State &state()
{
    static State instance;
    return instance;
}

char *copied(const std::string &text)
{
    char *out = static_cast<char *>(std::malloc(text.size() + 1));
    if (out) {
        std::memcpy(out, text.c_str(), text.size() + 1);
    }
    return out;
}

std::string escaped(const std::string &text)
{
    std::string out;
    for (const char c : text) {
        if (c == '"' || c == '\\') {
            out.push_back('\\');
        }
        out.push_back(c);
    }
    return out;
}

void installCallbacks()
{
    static std::once_flag once;
    std::call_once(once, []() {
        GuiInterface &gui = GuiInterface::Instance();
        gui.playerPort = kVideoPort;
        gui.onLog = [](LogLevel, std::string) {};
        gui.onStats = [](long long, long long, long long rtp) { state().rtp = rtp; };
        gui.onRtpStream = [](int, std::string codec) {
            std::lock_guard<std::mutex> guard(state().lock);
            state().codec = std::move(codec);
        };
        gui.onWifiStopped = []() { state().stopped = true; };
    });
}

std::vector<DeviceId> enumerate()
{
    try {
        return WfbngLink::get_device_list();
    } catch (const std::exception &) {
        return {};
    }
}

char *devices()
{
    std::string json = "[";
    bool first = true;
    for (const DeviceId &device : enumerate()) {
        json += std::string(first ? "" : ",") + "{\"name\":\"" + escaped(device.display_name) + "\",\"known\":" + (device.known_adapter ? "true" : "false") + "}";
        first = false;
    }
    return copied(json + "]");
}

void stopLink()
{
    std::unique_ptr<WfbngLink> link;
    {
        std::lock_guard<std::mutex> guard(state().lock);
        link = std::move(state().link);
    }
    if (link) {
        try {
            link->stop();
        } catch (const std::exception &) {
        }
    }
}

bool start(const char *adapter, uint8_t channel, int32_t channelWidth, const char *keyPath, char **error)
{
    installCallbacks();
    stopLink();
    const std::vector<DeviceId> found = enumerate();
    const auto device = std::find_if(found.begin(), found.end(), [adapter](const DeviceId &d) { return d.known_adapter && d.display_name == adapter; });
    if (device == found.end()) {
        *error = copied("the adapter is no longer attached");
        return false;
    }
    state().rtp = 0;
    state().stopped = false;
    try {
        auto link = std::make_unique<WfbngLink>();
        if (!link->start(*device, channel, channelWidth, keyPath)) {
            return false;
        }
        std::lock_guard<std::mutex> guard(state().lock);
        state().link = std::move(link);
        return true;
    } catch (const std::exception &failure) {
        *error = copied(failure.what());
        return false;
    }
}

void stop()
{
    stopLink();
}

void poll(int32_t *values)
{
    std::lock_guard<std::mutex> guard(state().lock);
    std::fill(values, values + 3 * ANTENNA_COUNT + 1, 0);
    if (!state().link) {
        return;
    }
    const auto rssi = state().link->get_rssi_level();
    const auto snr = state().link->get_snr_db();
    const auto score = state().link->get_link_score();
    std::copy(rssi.begin(), rssi.end(), values);
    std::copy(snr.begin(), snr.end(), values + ANTENNA_COUNT);
    std::copy(score.begin(), score.end(), values + 2 * ANTENNA_COUNT);
    values[3 * ANTENNA_COUNT] = state().link->get_packet_loss();
}

void adaptive(bool enabled, int32_t txPower)
{
    std::lock_guard<std::mutex> guard(state().lock);
    if (state().link) {
        state().link->enable_alink(enabled);
        state().link->set_alink_tx_power(txPower);
    }
}

int64_t rtpPackets()
{
    return state().rtp;
}

char *takeCodec()
{
    std::lock_guard<std::mutex> guard(state().lock);
    if (!state().codec) {
        return nullptr;
    }
    char *out = copied(*state().codec);
    state().codec.reset();
    return out;
}

bool takeStopped()
{
    return state().stopped.exchange(false);
}

void freeText(char *text)
{
    std::free(text);
}

}

const QGCPacketRadioNative *qgc_wfb_native(void)
{
    static const QGCPacketRadioNative native{devices, start, stop, poll, adaptive, rtpPackets, takeCodec, takeStopped, freeText};
    return &native;
}

[[maybe_unused]] static const bool registeredWithCore = qgc_core_packet_radio_register(qgc_wfb_native());
