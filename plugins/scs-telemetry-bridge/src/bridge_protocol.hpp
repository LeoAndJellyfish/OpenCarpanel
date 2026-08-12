#pragma once

#include <array>
#include <cstddef>
#include <cstdint>

namespace opencarpanel::scs_bridge {

inline constexpr std::size_t kPacketSize = 44;
inline constexpr std::uint8_t kProtocolVersion = 1;
inline constexpr std::uint16_t kDestinationPort = 20777;

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
};

[[nodiscard]] std::array<std::uint8_t, kPacketSize> encode(const TelemetryFrame& frame) noexcept;

}  // namespace opencarpanel::scs_bridge
