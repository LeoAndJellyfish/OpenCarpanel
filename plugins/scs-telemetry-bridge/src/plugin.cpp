#include "bridge_protocol.hpp"

#include <algorithm>
#include <array>
#include <chrono>
#include <cstddef>
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
#define OSD_PLUGIN_EXPORT
#else
#define OSD_PLUGIN_EXPORT __attribute__((visibility("default")))
#endif

namespace {

using opensimdash::scs_bridge::Game;
using opensimdash::scs_bridge::TelemetryFrame;

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
    float navigation_distance_m = 0.0F;
    float navigation_time_s = 0.0F;
    float navigation_speed_limit_mps = 0.0F;
    float fuel_liters = 0.0F;
    float fuel_capacity_liters = 0.0F;
    float fuel_range_km = 0.0F;
    float cargo_mass_kg = 0.0F;
    std::uint64_t income = 0;
    std::uint32_t delivery_time = 0;
    std::uint32_t planned_distance_km = 0;
    std::array<char, opensimdash::scs_bridge::kJobTextSize> cargo{};
    std::array<char, opensimdash::scs_bridge::kJobTextSize> source_city{};
    std::array<char, opensimdash::scs_bridge::kJobTextSize> destination_city{};
    bool fuel_warning = false;
    bool light_parking = false;
    bool light_low_beam = false;
    bool light_high_beam = false;
    bool light_beacon = false;
    bool light_brake = false;
    bool light_reverse = false;
    bool left_indicator = false;
    bool right_indicator = false;
    bool hazard = false;
    bool job_active = false;
    bool cargo_loaded = false;
    bool special_job = false;
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
        htons(opensimdash::scs_bridge::kDestinationPort);
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

std::uint16_t light_bits() noexcept {
    using namespace opensimdash::scs_bridge;
    std::uint16_t bits = 0;
    bits |= plugin_state.light_parking ? kLightParking : 0;
    bits |= plugin_state.light_low_beam ? kLightLowBeam : 0;
    bits |= plugin_state.light_high_beam ? kLightHighBeam : 0;
    bits |= plugin_state.light_beacon ? kLightBeacon : 0;
    bits |= plugin_state.light_brake ? kLightBrake : 0;
    bits |= plugin_state.light_reverse ? kLightReverse : 0;
    bits |= plugin_state.left_indicator ? kLightLeftIndicator : 0;
    bits |= plugin_state.right_indicator ? kLightRightIndicator : 0;
    bits |= plugin_state.hazard ? kLightHazard : 0;
    return bits;
}

std::uint16_t state_bits() noexcept {
    using namespace opensimdash::scs_bridge;
    std::uint16_t bits = 0;
    bits |= plugin_state.fuel_warning ? kStateFuelWarning : 0;
    bits |= plugin_state.job_active ? kStateJobActive : 0;
    bits |= plugin_state.cargo_loaded ? kStateCargoLoaded : 0;
    bits |= plugin_state.special_job ? kStateSpecialJob : 0;
    return bits;
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
        plugin_state.navigation_distance_m,
        plugin_state.navigation_time_s,
        plugin_state.navigation_speed_limit_mps,
        plugin_state.fuel_liters,
        plugin_state.fuel_capacity_liters,
        plugin_state.fuel_range_km,
        light_bits(),
        state_bits(),
        plugin_state.delivery_time,
        plugin_state.planned_distance_km,
        plugin_state.income,
        plugin_state.cargo_mass_kg,
        plugin_state.cargo,
        plugin_state.source_city,
        plugin_state.destination_city,
    };
    const auto packet = opensimdash::scs_bridge::encode(frame);

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

SCSAPI_VOID store_bool(
    const scs_string_t,
    const scs_u32_t,
    const scs_value_t* const value,
    const scs_context_t context) {
    if (value == nullptr || value->type != SCS_VALUE_TYPE_bool || context == nullptr) {
        return;
    }
    *static_cast<bool*>(context) = value->value_bool.value != 0;
}

void copy_utf8(
    std::array<char, opensimdash::scs_bridge::kJobTextSize>& destination,
    const char* const source) noexcept {
    destination.fill('\0');
    if (source == nullptr) {
        return;
    }
    const std::size_t source_length = std::strlen(source);
    std::size_t copy_length = std::min(source_length, destination.size() - 1);
    if (copy_length < source_length) {
        while (copy_length > 0 &&
               (static_cast<unsigned char>(source[copy_length]) & 0xc0U) == 0x80U) {
            --copy_length;
        }
    }
    std::memcpy(destination.data(), source, copy_length);
}

void reset_job() noexcept {
    plugin_state.job_active = false;
    plugin_state.cargo_loaded = false;
    plugin_state.special_job = false;
    plugin_state.cargo_mass_kg = 0.0F;
    plugin_state.income = 0;
    plugin_state.delivery_time = 0;
    plugin_state.planned_distance_km = 0;
    plugin_state.cargo.fill('\0');
    plugin_state.source_city.fill('\0');
    plugin_state.destination_city.fill('\0');
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
    if (configuration->id == nullptr) {
        return;
    }

    if (std::strcmp(configuration->id, SCS_TELEMETRY_CONFIG_truck) == 0) {
        plugin_state.rpm_max = 0.0F;
        plugin_state.fuel_capacity_liters = 0.0F;
        if (configuration->attributes == nullptr) {
            return;
        }
        for (const scs_named_value_t* attribute = configuration->attributes;
             attribute->name != nullptr;
             ++attribute) {
            if (std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_rpm_limit) == 0 &&
                attribute->value.type == SCS_VALUE_TYPE_float) {
                plugin_state.rpm_max = attribute->value.value_float.value;
            } else if (
                std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_fuel_capacity) == 0 &&
                attribute->value.type == SCS_VALUE_TYPE_float) {
                plugin_state.fuel_capacity_liters = attribute->value.value_float.value;
            }
        }
        return;
    }

    if (std::strcmp(configuration->id, SCS_TELEMETRY_CONFIG_job) != 0) {
        return;
    }
    reset_job();
    if (configuration->attributes == nullptr || configuration->attributes->name == nullptr) {
        return;
    }
    plugin_state.job_active = true;
    for (const scs_named_value_t* attribute = configuration->attributes;
         attribute->name != nullptr;
         ++attribute) {
        const scs_value_t& value = attribute->value;
        if (std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_cargo) == 0 &&
            value.type == SCS_VALUE_TYPE_string) {
            copy_utf8(plugin_state.cargo, value.value_string.value);
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_source_city) == 0 &&
            value.type == SCS_VALUE_TYPE_string) {
            copy_utf8(plugin_state.source_city, value.value_string.value);
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_destination_city) == 0 &&
            value.type == SCS_VALUE_TYPE_string) {
            copy_utf8(plugin_state.destination_city, value.value_string.value);
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_cargo_mass) == 0 &&
            value.type == SCS_VALUE_TYPE_float) {
            plugin_state.cargo_mass_kg = value.value_float.value;
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_income) == 0 &&
            value.type == SCS_VALUE_TYPE_u64) {
            plugin_state.income = value.value_u64.value;
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_delivery_time) == 0 &&
            value.type == SCS_VALUE_TYPE_u32) {
            plugin_state.delivery_time = value.value_u32.value;
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_planned_distance_km) == 0 &&
            value.type == SCS_VALUE_TYPE_u32) {
            plugin_state.planned_distance_km = value.value_u32.value;
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_is_cargo_loaded) == 0 &&
            value.type == SCS_VALUE_TYPE_bool) {
            plugin_state.cargo_loaded = value.value_bool.value != 0;
        } else if (
            std::strcmp(attribute->name, SCS_TELEMETRY_CONFIG_ATTRIBUTE_special_job) == 0 &&
            value.type == SCS_VALUE_TYPE_bool) {
            plugin_state.special_job = value.value_bool.value != 0;
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
               &plugin_state.brake) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_navigation_distance,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.navigation_distance_m) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_navigation_time,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.navigation_time_s) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_navigation_speed_limit,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.navigation_speed_limit_mps) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_fuel,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.fuel_liters) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_fuel_range,
               kIndex,
               SCS_VALUE_TYPE_float,
               kFlags,
               store_float,
               &plugin_state.fuel_range_km) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_fuel_warning,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.fuel_warning) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_light_parking,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.light_parking) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_light_low_beam,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.light_low_beam) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_light_high_beam,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.light_high_beam) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_light_beacon,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.light_beacon) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_light_brake,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.light_brake) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_light_reverse,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.light_reverse) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_lblinker,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.left_indicator) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_rblinker,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.right_indicator) == SCS_RESULT_ok &&
           params.register_for_channel(
               SCS_TELEMETRY_TRUCK_CHANNEL_hazard_warning,
               kIndex,
               SCS_VALUE_TYPE_bool,
               kFlags,
               store_bool,
               &plugin_state.hazard) == SCS_RESULT_ok;
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
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_navigation_distance,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_navigation_time,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_navigation_speed_limit,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_fuel,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_fuel_range,
            kIndex,
            SCS_VALUE_TYPE_float));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_fuel_warning,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_light_parking,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_light_low_beam,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_light_high_beam,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_light_beacon,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_light_brake,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_light_reverse,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_lblinker,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_rblinker,
            kIndex,
            SCS_VALUE_TYPE_bool));
        static_cast<void>(params.unregister_from_channel(
            SCS_TELEMETRY_TRUCK_CHANNEL_hazard_warning,
            kIndex,
            SCS_VALUE_TYPE_bool));
    }
}

}  // namespace

OSD_PLUGIN_EXPORT SCSAPI_RESULT scs_telemetry_init(
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
        log_message(SCS_LOG_TYPE_error, "OpenSimDash: missing SCS game id");
        game_log = nullptr;
        return SCS_RESULT_unsupported;
    }
    if (std::strcmp(version_params->common.game_id, SCS_GAME_ID_EUT2) == 0) {
        game = Game::kEts2;
    } else if (std::strcmp(version_params->common.game_id, SCS_GAME_ID_ATS) == 0) {
        game = Game::kAts;
    } else {
        log_message(SCS_LOG_TYPE_error, "OpenSimDash: unsupported SCS game");
        game_log = nullptr;
        return SCS_RESULT_unsupported;
    }

    unregister_callbacks(*version_params);
    plugin_state.game = game;
    plugin_state.session_nonce = make_session_nonce();
    if (!open_socket()) {
        log_message(SCS_LOG_TYPE_error, "OpenSimDash: failed to open loopback UDP socket");
        game_log = nullptr;
        return SCS_RESULT_generic_error;
    }
    if (!register_events(*version_params) || !register_channels(*version_params)) {
        log_message(SCS_LOG_TYPE_error, "OpenSimDash: failed to register SCS telemetry callbacks");
        unregister_callbacks(*version_params);
        close_socket();
        game_log = nullptr;
        return SCS_RESULT_generic_error;
    }

    log_message(SCS_LOG_TYPE_message, "OpenSimDash: telemetry bridge initialized");
    return SCS_RESULT_ok;
}

OSD_PLUGIN_EXPORT SCSAPI_VOID scs_telemetry_shutdown() {
    log_message(SCS_LOG_TYPE_message, "OpenSimDash: telemetry bridge stopped");
    close_socket();
    game_log = nullptr;
}
