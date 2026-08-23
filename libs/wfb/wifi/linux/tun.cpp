#include "tun.h"

#ifdef __linux__

    #include <errno.h>
    #include <fcntl.h>
    #include <linux/if_tun.h>
    #include <net/if.h>
    #include <poll.h>
    #include <stdio.h>
    #include <stdlib.h>
    #include <string.h>
    #include <sys/ioctl.h>
    #include <unistd.h>

    #include <cstring>
    #include <stdexcept>

int tun_connect(const char *iface_name, short flags, char *iface_name_out) {
    size_t iface_name_len;

    if (iface_name != NULL) {
        iface_name_len = strlen(iface_name);
        if (iface_name_len >= IFNAMSIZ) {
            errno = EINVAL;
            return -1;
        }
    }

    int tun_fd = open("/dev/net/tun", O_RDWR | O_CLOEXEC);
    if (tun_fd == -1) {
        return -1;
    }

    ifreq setiff_request{};
    setiff_request.ifr_flags = flags;
    if (iface_name != NULL) {
        memcpy(setiff_request.ifr_name, iface_name, iface_name_len + 1);
    }

    int rc = ioctl(tun_fd, TUNSETIFF, &setiff_request);
    if (rc == -1) {
        int ioctl_errno = errno;
        close(tun_fd);
        errno = ioctl_errno;
        return -1;
    }

    if (iface_name_out != NULL) {
        memcpy(iface_name_out, setiff_request.ifr_name, IFNAMSIZ);
    }

    return tun_fd;
}

    #include <arpa/inet.h>
    #include <linux/if.h>
    #include <linux/netlink.h>
    #include <linux/rtnetlink.h>

int netlink_connect() {
    struct sockaddr_nl sockaddr;

    const int netlink_fd = socket(AF_NETLINK, SOCK_RAW | SOCK_CLOEXEC, NETLINK_ROUTE);
    if (netlink_fd == -1) {
        return -1;
    }

    memset(&sockaddr, 0, sizeof sockaddr);
    sockaddr.nl_family = AF_NETLINK;
    int rc = bind(netlink_fd, (struct sockaddr *)&sockaddr, sizeof sockaddr);
    if (rc == -1) {
        int bind_errno = errno;
        close(netlink_fd);
        errno = bind_errno;
        return -1;
    }
    return netlink_fd;
}

int netlink_set_addr_ipv4(int netlink_fd, const char *iface_name, const char *address, uint8_t network_prefix_bits) {
    struct {
        struct nlmsghdr header;
        struct ifaddrmsg content;
        char attributes_buf[64];
    } request;

    size_t attributes_buf_avail = sizeof request.attributes_buf;

    memset(&request, 0, sizeof request);
    request.header.nlmsg_len = NLMSG_LENGTH(sizeof request.content);
    request.header.nlmsg_flags = NLM_F_REQUEST | NLM_F_EXCL | NLM_F_CREATE;
    request.header.nlmsg_type = RTM_NEWADDR;
    request.content.ifa_index = if_nametoindex(iface_name);
    request.content.ifa_family = AF_INET;
    request.content.ifa_prefixlen = network_prefix_bits;

    /* request.attributes[IFA_LOCAL] = address */
    rtattr *request_attr = IFA_RTA(&request.content);
    request_attr->rta_type = IFA_LOCAL;
    request_attr->rta_len = RTA_LENGTH(sizeof(struct in_addr));
    request.header.nlmsg_len += request_attr->rta_len;
    inet_pton(AF_INET, address, RTA_DATA(request_attr));

    /* request.attributes[IFA_ADDRESS] = address */
    request_attr = RTA_NEXT(request_attr, attributes_buf_avail);
    request_attr->rta_type = IFA_ADDRESS;
    request_attr->rta_len = RTA_LENGTH(sizeof(struct in_addr));
    request.header.nlmsg_len += request_attr->rta_len;
    inet_pton(AF_INET, address, RTA_DATA(request_attr));

    if (send(netlink_fd, &request, request.header.nlmsg_len, 0) == -1) {
        return -1;
    }
    return 0;
}

int netlink_link_up(int netlink_fd, const char *iface_name) {
    struct {
        struct nlmsghdr header;
        struct ifinfomsg content;
    } request;

    memset(&request, 0, sizeof request);
    request.header.nlmsg_len = NLMSG_LENGTH(sizeof request.content);
    request.header.nlmsg_flags = NLM_F_REQUEST;
    request.header.nlmsg_type = RTM_NEWLINK;
    request.content.ifi_index = if_nametoindex(iface_name);
    request.content.ifi_flags = IFF_UP;
    request.content.ifi_change = 1;

    if (send(netlink_fd, &request, request.header.nlmsg_len, 0) == -1) {
        return -1;
    }
    return 0;
}

int Tun::run_proxy(int tun_fd, int send_fd, int recv_fd) const {
    pollfd poll_fds[2];
    poll_fds[0].fd = tun_fd;
    poll_fds[0].events = POLLIN;
    poll_fds[1].fd = recv_fd;
    poll_fds[1].events = POLLIN;

    char recv_buf[UINT16_MAX];

    while (!should_stop) {
        if (const int rc = poll(poll_fds, 2, 0) == -1; rc < 0) {
            if (errno == EINTR || errno == EAGAIN) {
                continue;
            }
            throw std::runtime_error("poll error");
        }

        // 1) Thread to capture [ TUN → local localport:8001 UDP ] → rtl8812
        if ((poll_fds[0].revents & POLLIN) != 0) {
            // Receive from TUN
            ssize_t count = read(tun_fd, recv_buf, UINT16_MAX);
            if (count < 0) {
                return -1;
            }

            // Prepend the packet size in network byte order
            char size_bytes[2];
            size_bytes[0] = (char)((count >> 8) & 0xFF); // High byte
            size_bytes[1] = (char)(count & 0xFF);        // Low byte

            // Combine the size bytes and actual packet data
            char new_buf[2 + count];
            memcpy(new_buf, size_bytes, 2);
            memcpy(new_buf + 2, recv_buf, count);

            // Forward to UDP:8001 on localhost
            send(send_fd, new_buf, 2 + count, 0);

            printf("TUN -> localhost:8001, data size %lu+2\n", count);
        }

        // 2) Thread to capture rtl8812 → [ localport:8000 UDP → TUN ]
        if ((poll_fds[1].revents & POLLIN) != 0) {
            const ssize_t count = recv(recv_fd, recv_buf, UINT16_MAX, 0);
            if (count < 0) {
                return -1;
            }

            if (write(tun_fd, recv_buf + 2, count - 2) == -1) {
                return -1;
            }

            printf("localhost:8001 -> TUN\n");
        }
    }

    close_fds();

    return 0;
}

int connect_localhost_udp(const uint16_t port) {
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd == -1) {
        return -1;
    }

    sockaddr_in addr = {};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = inet_addr("127.0.0.1");

    if (int rc = connect(fd, (struct sockaddr *)&addr, sizeof addr); rc == -1) {
        int connect_errno = errno;
        close(fd);
        errno = connect_errno;
        return -1;
    }

    return fd;
}

int bind_localhost_udp(const uint16_t port) {
    int fd = socket(AF_INET, SOCK_DGRAM, 0);
    if (fd == -1) {
        return -1;
    }

    sockaddr_in addr = {};
    addr.sin_family = AF_INET;
    addr.sin_port = htons(port);
    addr.sin_addr.s_addr = inet_addr("127.0.0.1");

    if (int rc = bind(fd, (struct sockaddr *)&addr, sizeof addr); rc == -1) {
        int bind_errno = errno;
        close(fd);
        errno = bind_errno;
        return -1;
    }

    return fd;
}

Tun::~Tun() {
    stop();
}

bool Tun::init(const char *address, uint8_t prefix_bits, uint16_t send_port, uint16_t recv_port) {
    char iface_name[IFNAMSIZ];

    // Whatever received from the IP address will be forwarded to localhost:send_port
    send_fd = connect_localhost_udp(send_port);
    if (send_fd == -1) {
        fprintf(stderr, "Failed to bind_localhost_udp(%u) failed!", send_port);
        close_fds();
        return false;
    }

    // Whatever sent to localhost:recv_port will be forwarded to the IP address
    recv_fd = bind_localhost_udp(recv_port);
    if (recv_fd == -1) {
        fprintf(stderr, "connect_localhost_udp(%u) failed!", recv_port);
        close_fds();
        return false;
    }

    tun_fd = tun_connect(NULL, IFF_TUN | IFF_NO_PI, iface_name);
    if (tun_fd == -1) {
        fprintf(stderr, "tun_connect failed!");
        close_fds();
        return false;
    }

    const int netlink_fd = netlink_connect();
    if (netlink_fd == -1) {
        fprintf(stderr, "netlink_connect failed!");
        close_fds();
        return false;
    }

    int rc = netlink_set_addr_ipv4(netlink_fd, iface_name, address, prefix_bits);
    if (rc == -1) {
        fprintf(stderr, "netlink_set_addr_ipv4 failed!");
        close_fds();
        close(netlink_fd);
        return false;
    }
    rc = netlink_link_up(netlink_fd, iface_name);
    if (rc == -1) {
        fprintf(stderr, "netlink_link_up failed!");
        close_fds();
        close(netlink_fd);
        return false;
    }

    close(netlink_fd);

    return true;
}

bool Tun::start() {
    tun_thread = std::make_unique<std::thread>([=, this] { run_proxy(tun_fd, send_fd, recv_fd); });

    tun_thread->detach();

    return true;
}

void Tun::stop() {
    should_stop = true;
}

void Tun::close_fds() const {
    if (send_fd != -1) {
        close(send_fd);
    }
    if (recv_fd != -1) {
        close(recv_fd);
    }
    if (tun_fd != -1) {
        close(tun_fd);
    }
}

#endif
