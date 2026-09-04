/*
 * A tiny, dependency-free TCP <-> stdio bridge for OpenHarmony validation.
 *
 * Supported command lines:
 *
 *   tcp_stdio_bridge -l -p PORT       listen once on IPv4 loopback
 *   tcp_stdio_bridge 127.0.0.1 PORT   connect to one IPv4 endpoint
 *
 * The bridge deliberately handles one connection only.  Bytes read from
 * stdin are written to the socket and bytes read from the socket are written
 * to stdout.  The poll loop keeps both directions live without depending on
 * shell-specific pipeline or background-job behavior.
 */

#define _POSIX_C_SOURCE 200809L

#include <arpa/inet.h>
#include <errno.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <poll.h>
#include <signal.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/socket.h>
#include <unistd.h>

#define BUFFER_CAP 65536U
#define IO_CHUNK 16384U

struct buffer {
    unsigned char data[BUFFER_CAP];
    size_t off;
    size_t len;
};

static void usage(const char *program)
{
    fprintf(stderr,
            "usage: %s -l -p PORT | %s IPV4 PORT\n",
            program,
            program);
}

static int parse_port(const char *text, uint16_t *port)
{
    char *end = NULL;
    unsigned long value;

    if (text == NULL || *text == '\0') {
        return -1;
    }
    errno = 0;
    value = strtoul(text, &end, 10);
    if (errno != 0 || end == text || *end != '\0' || value == 0 || value > 65535UL) {
        return -1;
    }
    *port = (uint16_t)value;
    return 0;
}

static int set_nonblocking(int fd)
{
    int flags = fcntl(fd, F_GETFL, 0);
    if (flags < 0 || fcntl(fd, F_SETFL, flags | O_NONBLOCK) < 0) {
        return -1;
    }
    return 0;
}

static int listen_once(uint16_t port)
{
    int listener = -1;
    int client = -1;
    int reuse = 1;
    struct sockaddr_in address;

    listener = socket(AF_INET, SOCK_STREAM, 0);
    if (listener < 0) {
        perror("socket");
        return -1;
    }
    (void)setsockopt(listener, SOL_SOCKET, SO_REUSEADDR, &reuse, sizeof(reuse));

    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    /* The HDC fport/rport data path reaches the device's loopback socket. */
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    address.sin_port = htons(port);
    if (bind(listener, (struct sockaddr *)&address, sizeof(address)) < 0) {
        perror("bind");
        close(listener);
        return -1;
    }
    if (listen(listener, 1) < 0) {
        perror("listen");
        close(listener);
        return -1;
    }

    do {
        client = accept(listener, NULL, NULL);
    } while (client < 0 && errno == EINTR);
    if (client < 0) {
        perror("accept");
    }
    close(listener);
    return client;
}

static int connect_once(const char *host, uint16_t port)
{
    int fd;
    struct sockaddr_in address;

    fd = socket(AF_INET, SOCK_STREAM, 0);
    if (fd < 0) {
        perror("socket");
        return -1;
    }

    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_port = htons(port);
    if (inet_pton(AF_INET, host, &address.sin_addr) != 1) {
        fprintf(stderr, "invalid IPv4 address: %s\n", host);
        close(fd);
        return -1;
    }
    do {
        if (connect(fd, (struct sockaddr *)&address, sizeof(address)) == 0) {
            return fd;
        }
    } while (errno == EINTR);

    perror("connect");
    close(fd);
    return -1;
}

static void compact_buffer(struct buffer *buffer)
{
    if (buffer->off == 0) {
        return;
    }
    if (buffer->len != 0) {
        memmove(buffer->data, buffer->data + buffer->off, buffer->len);
    }
    buffer->off = 0;
}

static int append_from_fd(int fd, struct buffer *buffer, int *open)
{
    unsigned char chunk[IO_CHUNK];
    size_t available;
    size_t wanted;
    ssize_t count;

    compact_buffer(buffer);
    available = BUFFER_CAP - buffer->len;
    if (available == 0) {
        return 0;
    }
    wanted = available < sizeof(chunk) ? available : sizeof(chunk);
    do {
        count = read(fd, chunk, wanted);
    } while (count < 0 && errno == EINTR);
    if (count > 0) {
        memcpy(buffer->data + buffer->off + buffer->len, chunk, (size_t)count);
        buffer->len += (size_t)count;
        return 0;
    }
    if (count == 0) {
        *open = 0;
        return 0;
    }
    if (errno == EAGAIN || errno == EWOULDBLOCK) {
        return 0;
    }
    return -1;
}

static int flush_to_fd(int fd, struct buffer *buffer, int *open)
{
    ssize_t count;

    while (buffer->len != 0) {
        do {
            count = write(fd, buffer->data + buffer->off, buffer->len);
        } while (count < 0 && errno == EINTR);
        if (count > 0) {
            buffer->off += (size_t)count;
            buffer->len -= (size_t)count;
            if (buffer->len == 0) {
                buffer->off = 0;
            }
            continue;
        }
        if (count < 0 && (errno == EAGAIN || errno == EWOULDBLOCK)) {
            return 0;
        }
        *open = 0;
        return -1;
    }
    return 0;
}

static int bridge(int socket_fd)
{
    struct buffer to_socket = { { 0 }, 0, 0 };
    struct buffer to_stdout = { { 0 }, 0, 0 };
    int stdin_open = 1;
    int stdout_open = 1;
    int socket_read_open = 1;
    int socket_write_open = 1;
    int socket_write_shutdown = 0;

    if (set_nonblocking(STDIN_FILENO) < 0 || set_nonblocking(STDOUT_FILENO) < 0 ||
        set_nonblocking(socket_fd) < 0) {
        perror("fcntl");
        close(socket_fd);
        return 1;
    }

    while (socket_read_open || to_stdout.len != 0 || socket_write_open) {
        struct pollfd pollfds[3];
        nfds_t nfds = 0;
        int stdin_index = -1;
        int socket_index = -1;
        int stdout_index = -1;
        int result;

        if (stdin_open && to_socket.len < BUFFER_CAP) {
            stdin_index = (int)nfds;
            pollfds[nfds].fd = STDIN_FILENO;
            pollfds[nfds].events = POLLIN | POLLHUP | POLLERR;
            pollfds[nfds].revents = 0;
            nfds++;
        }

        if (socket_read_open || (socket_write_open && to_socket.len != 0)) {
            socket_index = (int)nfds;
            pollfds[nfds].fd = socket_fd;
            pollfds[nfds].events = POLLERR | POLLHUP;
            if (socket_read_open && to_stdout.len < BUFFER_CAP) {
                pollfds[nfds].events |= POLLIN;
            }
            if (socket_write_open && to_socket.len != 0) {
                pollfds[nfds].events |= POLLOUT;
            }
            pollfds[nfds].revents = 0;
            nfds++;
        }

        if (stdout_open && to_stdout.len != 0) {
            stdout_index = (int)nfds;
            pollfds[nfds].fd = STDOUT_FILENO;
            pollfds[nfds].events = POLLOUT | POLLHUP | POLLERR;
            pollfds[nfds].revents = 0;
            nfds++;
        }

        if (nfds == 0) {
            break;
        }
        do {
            result = poll(pollfds, nfds, -1);
        } while (result < 0 && errno == EINTR);
        if (result < 0) {
            perror("poll");
            break;
        }

        if (stdin_index >= 0 &&
            (pollfds[stdin_index].revents & (POLLIN | POLLHUP | POLLERR))) {
            if (append_from_fd(STDIN_FILENO, &to_socket, &stdin_open) < 0) {
                stdin_open = 0;
            }
        }

        if (socket_index >= 0) {
            short events = pollfds[socket_index].revents;
            if (socket_read_open && (events & (POLLIN | POLLHUP | POLLERR))) {
                if (append_from_fd(socket_fd, &to_stdout, &socket_read_open) < 0) {
                    socket_read_open = 0;
                }
            }
            if (socket_write_open && to_socket.len != 0 && (events & (POLLOUT | POLLERR | POLLHUP))) {
                if (flush_to_fd(socket_fd, &to_socket, &socket_write_open) < 0) {
                    socket_write_open = 0;
                }
            }
        }

        if (stdout_index >= 0 &&
            (pollfds[stdout_index].revents & (POLLOUT | POLLHUP | POLLERR))) {
            (void)flush_to_fd(STDOUT_FILENO, &to_stdout, &stdout_open);
        }

        if (!stdin_open && to_socket.len == 0 && socket_write_open && !socket_write_shutdown) {
            (void)shutdown(socket_fd, SHUT_WR);
            socket_write_shutdown = 1;
            socket_write_open = 0;
        }
        if (!stdout_open) {
            break;
        }
        if (!socket_read_open && to_stdout.len == 0 && !socket_write_open) {
            break;
        }
    }

    close(socket_fd);
    return 0;
}

int main(int argc, char **argv)
{
    int socket_fd;
    uint16_t port;

    if (signal(SIGPIPE, SIG_IGN) == SIG_ERR) {
        perror("signal");
        return 1;
    }

    if (argc == 4 && strcmp(argv[1], "-l") == 0 && strcmp(argv[2], "-p") == 0) {
        if (parse_port(argv[3], &port) < 0) {
            fprintf(stderr, "invalid port: %s\n", argv[3]);
            usage(argv[0]);
            return 2;
        }
        socket_fd = listen_once(port);
    } else if (argc == 3) {
        if (parse_port(argv[2], &port) < 0) {
            fprintf(stderr, "invalid port: %s\n", argv[2]);
            usage(argv[0]);
            return 2;
        }
        socket_fd = connect_once(argv[1], port);
    } else {
        usage(argv[0]);
        return 2;
    }

    if (socket_fd < 0) {
        return 1;
    }
    return bridge(socket_fd);
}
