#include "bridge_protocol.hpp"

#include <array>
#include <cstddef>
#include <cstdint>

namespace {

template <std::size_t Size>
bool matches(
    const std::array<std::uint8_t, opensimdash::scs_bridge::kPacketSize>& packet,
    const std::size_t offset,
    const std::array<std::uint8_t, Size>& expected) {
    for (std::size_t index = 0; index < Size; ++index) {
        if (packet[offset + index] != expected[index]) {
            return false;
        }
    }
    return true;
}

}  // namespace

int main() {
    using namespace opensimdash::scs_bridge;

    TelemetryFrame frame{};
    frame.game = Game::kAts;
    frame.session_nonce = UINT64_C(0x0102030405060708);
    frame.frame_sequence = UINT32_C(0x0a0b0c0d);
    frame.speed_mps = -12.5F;
    frame.rpm = 1024.0F;
    frame.rpm_max = 2048.0F;
    frame.displayed_gear = -1;
    frame.throttle = 0.5F;
    frame.brake = 0.25F;
    frame.navigation_distance_m = 1.5F;
    frame.navigation_time_s = 2.5F;
    frame.navigation_speed_limit_mps = 3.5F;
    frame.fuel_liters = 4.5F;
    frame.fuel_capacity_liters = 5.5F;
    frame.fuel_range_km = 6.5F;
    frame.light_bits = UINT16_C(0x0193);
    frame.state_bits = UINT16_C(0x000f);
    frame.delivery_time = UINT32_C(0x11223344);
    frame.planned_distance_km = UINT32_C(0x55667788);
    frame.income = UINT64_C(0x0102030405060708);
    frame.cargo_mass_kg = 7.5F;
    frame.cargo[0] = 'C';
    frame.cargo[1] = 'a';
    frame.cargo[2] = 'r';
    frame.cargo[3] = 'g';
    frame.cargo[4] = 'o';
    frame.source_city[0] = 'A';
    frame.destination_city[0] = 'B';

    const auto packet = encode(frame);
    constexpr std::array<std::uint8_t, 44> expected_base{
        0x4f, 0x53, 0x44, 0x00, 0x02, 0x02, 0x00, 0x00,
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
        0x0d, 0x0c, 0x0b, 0x0a,
        0x00, 0x00, 0x48, 0xc1,
        0x00, 0x00, 0x80, 0x44,
        0x00, 0x00, 0x00, 0x45,
        0xff, 0xff, 0xff, 0xff,
        0x00, 0x00, 0x00, 0x3f,
        0x00, 0x00, 0x80, 0x3e,
    };
    if (!matches(packet, 0, expected_base)) {
        return 1;
    }

    constexpr std::array<std::uint8_t, 24> expected_navigation_and_fuel{
        0x00, 0x00, 0xc0, 0x3f,
        0x00, 0x00, 0x20, 0x40,
        0x00, 0x00, 0x60, 0x40,
        0x00, 0x00, 0x90, 0x40,
        0x00, 0x00, 0xb0, 0x40,
        0x00, 0x00, 0xd0, 0x40,
    };
    if (!matches(packet, 44, expected_navigation_and_fuel)) {
        return 2;
    }

    constexpr std::array<std::uint8_t, 24> expected_flags_and_job_numbers{
        0x93, 0x01, 0x0f, 0x00,
        0x44, 0x33, 0x22, 0x11,
        0x88, 0x77, 0x66, 0x55,
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
        0x00, 0x00, 0xf0, 0x40,
    };
    if (!matches(packet, 68, expected_flags_and_job_numbers)) {
        return 3;
    }

    constexpr std::array<std::uint8_t, 6> expected_cargo{'C', 'a', 'r', 'g', 'o', 0x00};
    if (!matches(packet, 92, expected_cargo) || packet[124] != 'A' || packet[125] != 0 ||
        packet[156] != 'B' || packet[157] != 0 || packet[187] != 0) {
        return 4;
    }

    frame.game = Game::kEts2;
    if (encode(frame)[5] != 1) {
        return 5;
    }
    return 0;
}
