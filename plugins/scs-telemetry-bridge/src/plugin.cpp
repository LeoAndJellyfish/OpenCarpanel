#include "bridge_protocol.hpp"

#include <chrono>
#include <cstdint>
#include <cstring>

#if defined(_WIN32)
#include <winsock2.h>
#include <ws2tcpip.h>
#else
#include <arpa/inet.h>
#include <fcntl.h>
#include <netinet/in.h>
#include <sys/socket.h>
#include <unistd.h>
#endif

#include "amtrucks/scssdk_ats.h"
#include "common/scssdk_telemetry_common_configs.h"
#include "common/scssdk_telemetry_truck_common_channels.h"
#include "eurotrucks2/scssdk_eut2.h"
#include "scssdk_telemetry.h"

#if defined(_WIN32)
#define OCP_PLUGIN_EXPORT
#else
#define OCP_PLUGIN_EXPORT __attribute__((visibility("default")))
#endif

namespace {

using opencarpanel::scs_bridge::Game;
using opencarpanel::scs_bridge::TelemetryFrame;

#if defined(_WIN32)
using SocketHandle = SOCKET;
constexpr SocketHandle kInvalidSocket = INVALID_SOCKET;
#else
using SocketHandle = int;
constexpr SocketHandle kInvalidSocket = -1;
#endif

struct PluginState {
    Game game = Game::kEts2;
    std::uint64_t session_nonce = 0;
    std::uint32_t frame_sequence = 0;
    float speed_mps = 0.0F;
    float rpm = 0.0F;
    float rpm_max = 0.0F;
    std::int32_t displayed_gear = 0;
    float throttle = 0.0F;
    float brake = 0.0F;
    bool paused = true;
    SocketHandle socket = kInvalidSocket;
    sockaddr_in destination{};
#if defined(_WIN32)
    bool winsock_started = false;
#endif
};

PluginState plugin_state;
scs_log_t game_log = nullptr;

void log_message(const scs_log_type_t type, const char* const message) noexcept {
    if (game_log != nullptr) {
        game_log(type, message);
    }
}

void close_socket() noexcept {
    if (plugin_state.socket != kInvalidSocket) {
#if defined(_WIN32)
        closesocket(plugin_state.socket);
#else
        close(plugin_state.socket);
#endif
        plugin_state.socket = kInvalidSocket;
    }
#if defined(_WIN32)
    if (plugin_state.winsock_started) {
        WSACleanup();
        plugin_state.winsock_started = false;
    }
#endif
}

bool open_socket() noexcept {
#if defined(_WIN32)
    WSADATA winsock_data{};
    if (WSAStartup(MAKEWORD(2, 2), &winsock_data) != 0) {
        return false;
    }
    plugin_state.winsock_started = true;
#endif

    plugin_state.socket = socket(AF_INET, SOCK_DGRAM, IPPROTO_UDP);
    if (plugin_state.socket == kInvalidSocket) {
        close_socket();
        return false;
    }

#if defined(_WIN32)
    u_long nonblocking = 1;
    if (ioctlsocket(plugin_state.socket, FIONBIO, &nonblocking) != 0) {
        close_socket();
        return false;
    }
#else
    const int flags = fcntl(plugin_state.socket, F_GETFL, 0);
    if (flags == -1 || fcntl(plugin_state.socket, F_SETFL, flags | O_NONBLOCK) == -1) {
        close_socket();
        return false;
    }
#endif

    plugin_state.destination = {};
    plugin_state.destination.sin_family = AF_INET;
    plugin_state.destination.sin_port =
        htons(opencarpanel::scs_bridge::kDestinationPort);
    plugin_state.destination.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    return true;
}

std::uint64_t make_session_nonce() noexcept {
    const auto wall_clock = std::chrono::system_clock::now().time_since_epoch().count();
    const auto monotonic = std::chrono::steady_clock::now().time_since_epoch().count();
    const auto address = reinterpret_cast<std::uintptr_t>(&plugin_state);
    return static_cast<std::uint64_t>(wall_clock) ^
           (static_cast<std::uint64_t>(monotonic) << 1U) ^
           static_cast<std::uint64_t>(address);
}

void send_frame() noexcept {
    if (plugin_state.paused || plugin_state.socket == kInvalidSocket) {
        return;
    }

    const TelemetryFrame frame{
        plugin_state.game,
        plugin_state.session_nonce,
        plugin_state.frame_sequence++,
        plugin_state.speed_mps,
        plugin_state.rpm,
        plugin_state.rpm_max,
        plugin_state.displayed_gear,
        plugin_state.throttle,
        plugin_state.brake,
    };
    const auto packet = opencarpanel::scs_bridge::encode(frame);

#if defined(_WIN32)
    const int result = sendto(
        plugin_state.socket,
        reinterpret_cast<const char*>(packet.data()),
        static_cast<int>(packet.size()),
        0,
        reinterpret_cast<const sockaddr*>(&plugin_state.destination),
        static_cast<int>(sizeof(plugin_state.destination)));
#else
    const ssize_t result = sendto(
        plugin_state.socket,
        packet.data(),
        packet.size(),
        0,
        reinterpret_cast<const sockaddr*>(&plugin_state.destination),
        sizeof(plugin_state.destination));
#endif
    static_cast<void>(result);
}

SCSAPI_VOID on_frame_end(
    const scs_event_t,
    const void* const,
    const scs_context_t) {
    send_frame();
}

SCSAPI_VOID on_pause_state(
    const scs_event_t event,
    const void* const,
    const scs_context_t) {
    plugin_state.paused = event == SCS_TELEMETRY_EVENT_paused;
}

SCSAPI_VOID store_float(
    const scs_string_t,
    const scs_u32_t,
    const scs_value_t* const value,
    const scs_context_t context) {
    if (value == nullptr || value->type != SCS_VALUE_TYPE_float || context == nullptr) {
        return;
    }
    *static_cast<float*>(context) = value->value_float.value;
}

SCSAPI_VOID store_s32(
    const scs_string_t,
    const scs_u32_t,
    const scs_value_t* const value,
    const scs_context_t context) {
    if (value == nullptr || value->type != SCS_VALUE_TYPE_s32 || context == nullptr) {
        return;
    }
    *static_cast<std::int32_t*>(context) = value->value_s32.value;
}

SCSAPI_VOID on_configuration(
    const scs_event_t,
    const void* const event_info,
    const scs_context_t) {
    if (event_info == nullptr) {
        return;
    }
    const auto* const configuration =
        static_cast<const scs_telemetry_configuration_t*>(event_info);
    if (configuration->id == nullptr ||
        std::strcmp(configuration->id, SCS_TELEMETRY_CONFIG_truck) != 0) {
        return;
    }

    plugin_state.rpm_max = 0.0F;
    if (configuration->attributes == nullptr) {
        return;
    }
    for (const scs_named_value_t* attribute = configuration->attributes;
         attribute->name != nullptr;
         ++attribute) {
        if (std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_rpm_limit) == 0 &&
            attribute->value.type == SCS_VALUE_TYPE_float) {
            plugin_state.rpm_max = attribute->value.value_float.value;
            return;
        }
    }
}

bool register_events(const scs_telemetry_init_params_v101_t& params) noexcept {
    return params.register_for_event(SCS_TELEMETRY_EVENT_frame_end, on_frame_end, nullptr) ==
               SCS_RESULT_ok &&
           params.register_for_event(SCS_TELEMETRY_EVENT_paused, on_pause_state, nullptr) ==
               SCS_RESULT_ok &&
           params.register_for_event(SCS_TELEMETRY_EVENT_started, on_pause_state, nullptr) ==
               SCS_RESULT_ok &&
           params.register_for_event(
               SCS_TELEMETRY_EVENT_configuration,
               on_configuration,
               nullptr) == SCS_RESULT_ok;
}

bool register_channels(const scs_telemetry_init_params_v101_t& params) noexcept {
    constexpr scs_u32_t kIndex = SCS_U32_NIL;
    constexpr scs_u32_t kFlags = SCS_TELEMETRY_CHANNEL_FLAG_none;
    return params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_speed,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.speed_mps) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_engine_rpm,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.rpm) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_displayed_gear,
               kIndex,
               SCS_VALUE_TYPE_s32,
               kFlags,
               store_s32,
               &plugin_state.displayed_gear) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_effective_throttle,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.throttle) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_effective_brake,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.brake) == SCS_RESULT_ok;
}

void unregister_callbacks(const scs_telemetry_init_params_v101_t& params) noexcept {
    constexpr scs_u32_t kIndex = SCS_U32_NIL;
    if (params.unregister_from_event != nullptr) {
        static_cast<void>(params.unregister_from_event(SCS_TELEMETRY_EVENT_frame_end));
        static_cast<void>(params.unregister_from_event(SCS_TELEMETRY_EVENT_paused));
        static_cast<void>(params.unregister_from_event(SCS_TELEMETRY_EVENT_started));
        static_cast<void>(params.unregister_from_event(SCS_TELEMETRY_EVENT_configuration));
    }
    if (params.unregister_from_channel != nullptr) {
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_speed,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_engine_rpm,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_displayed_gear,
            kIndex,
            SCS_VALUE_TYPE_s32));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_effective_throttle,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_effective_brake,
            kIndex,
            SCS_VALUE_TYPE_float));
    }
}

}  // namespace

OCP_PLUGIN_EXPORT SCSAPI_RESULT scs_telemetry_init(
    const scs_u32_t version,
    const scs_telemetry_init_params_t* const params) {
    if (version != SCS_TELEMETRY_VERSION_1_01 || params == nullptr) {
        return SCS_RESULT_unsupported;
    }

    close_socket();
    plugin_state = {};
    const auto* const version_params =
        static_cast<const scs_telemetry_init_params_v101_t*>(params);
    game_log = version_params->common.log;

    Game game;
    if (version_params->common.game_id == nullptr) {
        log_message(SCS_LOG_TYPE_error, "OpenCarpanel: missing SCS game id");
        game_log = nullptr;
        return SCS_RESULT_unsupported;
    }
    if (std::strcmp(version_params->common.game_id, SCS_GAME_ID_EUT2) == 0) {
        game = Game::kEts2;
    } else if (std::strcmp(version_params->common.game_id, SCS_GAME_ID_ATS) == 0) {
        game = Game::kAts;
    } else {
        log_message(SCS_LOG_TYPE_error, "OpenCarpanel: unsupported SCS game");
        game_log = nullptr;
        return SCS_RESULT_unsupported;
    }

    unregister_callbacks(*version_params);
    plugin_state.game = game;
    plugin_state.session_nonce = make_session_nonce();
    if (!open_socket()) {
        log_message(SCS_LOG_TYPE_error, "OpenCarpanel: failed to open loopback UDP socket");
        game_log = nullptr;
        return SCS_RESULT_generic_error;
    }
    if (!register_events(*version_params) || !register_channels(*version_params)) {
        log_message(SCS_LOG_TYPE_error, "OpenCarpanel: failed to register SCS telemetry callbacks");
        unregister_callbacks(*version_params);
        close_socket();
        game_log = nullptr;
        return SCS_RESULT_generic_error;
    }

    log_message(SCS_LOG_TYPE_message, "OpenCarpanel: telemetry bridge initialized");
    return SCS_RESULT_ok;
}

OCP_PLUGIN_EXPORT SCSAPI_VOID scs_telemetry_shutdown() {
    log_message(SCS_LOG_TYPE_message, "OpenCarpanel: telemetry bridge stopped");
    close_socket();
    game_log = nullptr;
}
