#include "bridge_protocol.hpp"

#include <cstring>
#include <limits>

namespace opencarpanel::scs_bridge {
namespace {

static_assert(sizeof(float) == sizeof(std::uint32_t));
static_assert(std::numeric_limits<float>::is_iec559);

void write_u32(
    std::array<std::uint8_t, kPacketSize>& packet,
    std::size_t& offset,
    const std::uint32_t value) noexcept {
    for (unsigned shift = 0; shift < 32; shift += 8) {
        packet[offset++] = static_cast<std::uint8_t>(value >> shift);
    }
}

void write_u64(
    std::array<std::uint8_t, kPacketSize>& packet,
    std::size_t& offset,
    const std::uint64_t value) noexcept {
    for (unsigned shift = 0; shift < 64; shift += 8) {
        packet[offset++] = static_cast<std::uint8_t>(value >> shift);
    }
}

void write_float(
    std::array<std::uint8_t, kPacketSize>& packet,
    std::size_t& offset,
    const float value) noexcept {
    std::uint32_t bits = 0;
    std::memcpy(&bits, &value, sizeof(bits));
    write_u32(packet, offset, bits);
}

}  // namespace

std::array<std::uint8_t, kPacketSize> encode(const TelemetryFrame& frame) noexcept {
    std::array<std::uint8_t, kPacketSize> packet{};
    packet[0] = 'O';
    packet[1] = 'C';
    packet[2] = 'P';
    packet[3] = 0;
    packet[4] = kProtocolVersion;
    packet[5] = static_cast<std::uint8_t>(frame.game);
    packet[6] = 0;
    packet[7] = 0;

    std::size_t offset = 8;
    write_u64(packet, offset, frame.session_nonce);
    write_u32(packet, offset, frame.frame_sequence);
    write_float(packet, offset, frame.speed_mps);
    write_float(packet, offset, frame.rpm);
    write_float(packet, offset, frame.rpm_max);
    write_u32(packet, offset, static_cast<std::uint32_t>(frame.displayed_gear));
    write_float(packet, offset, frame.throttle);
    write_float(packet, offset, frame.brake);
    return packet;
}

}  // namespace opencarpanel::scs_bridge
