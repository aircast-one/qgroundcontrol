#pragma once

#include <cstdint>
#include <functional>
#include <sstream>
#include <string>
#include <string_view>

enum class LogLevel {
    Info,
    Debug,
    Warn,
    Error,
};

class GuiInterface {
public:
    static GuiInterface &Instance() {
        static GuiInterface instance;
        return instance;
    }

    template <typename... Args>
    void PutLog(LogLevel level, const std::string_view message, Args... format_items) {
        if (onLog) {
            std::string out(message);
            (SubstituteFirstBrace(out, format_items), ...);
            onLog(level, std::move(out));
        }
    }

    void UpdateCount() {
        if (onStats) {
            onStats(wifiFrameCount_, wfbngFrameCount_, rtpPktCount_);
        }
    }

    void ShowTip(std::string msg, bool bad_news) {
        if (onLog) {
            onLog(bad_news ? LogLevel::Warn : LogLevel::Info, std::move(msg));
        }
    }

    void NotifyRtpStream(int pt, uint16_t ssrc, int port, const std::string &codec) {
        if (onRtpStream) {
            onRtpStream(port, codec);
        }
    }

    void EmitWifiStopped() {
        if (onWifiStopped) {
            onWifiStopped();
        }
    }

    std::function<void(LogLevel, std::string)> onLog;
    std::function<void(long long wifi, long long wfb, long long rtp)> onStats;
    std::function<void(int port, std::string codec)> onRtpStream;
    std::function<void()> onWifiStopped;

    template <typename T>
    static void SubstituteFirstBrace(std::string &text, const T &value) {
        const auto pos = text.find("{}");
        if (pos == std::string::npos) {
            return;
        }
        std::ostringstream oss;
        oss << value;
        text.replace(pos, 2, oss.str());
    }

    long long wifiFrameCount_ = 0;
    long long wfbngFrameCount_ = 0;
    long long rtpPktCount_ = 0;
    int playerPort = 5600;
    std::string playerCodec;
    int drone_fec_level_ = 0;
};
