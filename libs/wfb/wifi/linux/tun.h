#pragma once

#include <memory>
#include <thread>

class Tun {
public:
    Tun() = default;

    ~Tun();

    ///
    /// @param address
    /// @param prefix_bits
    /// @param send_port Port to send data
    /// @param recv_port Port to listen to, receiving data
    /// @return
    bool init(const char *address, uint8_t prefix_bits, uint16_t send_port, uint16_t recv_port);

    bool start();

    void stop();

private:
    void close_fds() const;

    int run_proxy(int tun_fd, int send_fd, int recv_fd) const;

    bool should_stop = false;
    const char *address = nullptr;
    uint8_t prefix_bits = 0;
    uint16_t send_port = 0;
    uint16_t recv_port = 0;
    int tun_fd = -1;
    int send_fd = -1;
    int recv_fd = -1;

    std::unique_ptr<std::thread> tun_thread;
};
