#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace opencarpanel::scs_bridge {

inline constexpr std::size_t kPacketSize = 188;
inline constexpr std::size_t kJobTextSize = 32;
inline constexpr std::uint8_t kProtocolVersion = 2;
inline constexpr std::uint16_t kDestinationPort = 20777;

inline constexpr std::uint16_t kLightParking = 1U << 0U;
inline constexpr std::uint16_t kLightLowBeam = 1U << 1U;
inline constexpr std::uint16_t kLightHighBeam = 1U << 2U;
inline constexpr std::uint16_t kLightBeacon = 1U << 3U;
inline constexpr std::uint16_t kLightBrake = 1U << 4U;
inline constexpr std::uint16_t kLightReverse = 1U << 5U;
inline constexpr std::uint16_t kLightLeftIndicator = 1U << 6U;
inline constexpr std::uint16_t kLightRightIndicator = 1U << 7U;
inline constexpr std::uint16_t kLightHazard = 1U << 8U;

inline constexpr std::uint16_t kStateFuelWarning = 1U << 0U;
inline constexpr std::uint16_t kStateJobActive = 1U << 1U;
inline constexpr std::uint16_t kStateCargoLoaded = 1U << 2U;
inline constexpr std::uint16_t kStateSpecialJob = 1U << 3U;

enum class Game : std::uint8_t {
    kEts2 = 1,
    kAts = 2,
};

struct TelemetryFrame {
    Game game;
    std::uint64_t session_nonce;
    std::uint32_t frame_sequence;
    float speed_mps;
    float rpm;
    float rpm_max;
    std::int32_t displayed_gear;
    float throttle;
    float brake;
    float navigation_distance_m;
    float navigation_time_s;
    float navigation_speed_limit_mps;
    float fuel_liters;
    float fuel_capacity_liters;
    float fuel_range_km;
    std::uint16_t light_bits;
    std::uint16_t state_bits;
    std::uint32_t delivery_time;
    std::uint32_t planned_distance_km;
    std::uint64_t income;
    float cargo_mass_kg;
    std::array<char, kJobTextSize> cargo;
    std::array<char, kJobTextSize> source_city;
    std::array<char, kJobTextSize> destination_city;
};

[[nodiscard]] std::array<std::uint8_t, kPacketSize> encode(const TelemetryFrame& frame) noexcept;

}  // namespace opencarpanel::scs_bridge
