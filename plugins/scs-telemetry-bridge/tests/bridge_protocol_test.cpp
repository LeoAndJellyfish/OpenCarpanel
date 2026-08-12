#include "bridge_protocol.hpp"

#include <array>
#include <cassert>
#include <cstdint>

int main() {
    using opencarpanel::scs_bridge::Game;
    using opencarpanel::scs_bridge::TelemetryFrame;
    using opencarpanel::scs_bridge::encode;

    const TelemetryFrame frame{
        Game::kAts,
        UINT64_C(0x0102030405060708),
        UINT32_C(0x0a0b0c0d),
        -12.5F,
        1024.0F,
        2048.0F,
        -1,
        0.5F,
        0.25F,
    };
    constexpr std::array<std::uint8_t, 44> expected{
        0x4f, 0x43, 0x50, 0x00, 0x01, 0x02, 0x00, 0x00,
        0x08, 0x07, 0x06, 0x05, 0x04, 0x03, 0x02, 0x01,
        0x0d, 0x0c, 0x0b, 0x0a,
        0x00, 0x00, 0x48, 0xc1,
        0x00, 0x00, 0x80, 0x44,
        0x00, 0x00, 0x00, 0x45,
        0xff, 0xff, 0xff, 0xff,
        0x00, 0x00, 0x00, 0x3f,
        0x00, 0x00, 0x80, 0x3e,
    };

    assert(encode(frame) == expected);

    const auto ets2 = encode(TelemetryFrame{Game::kEts2, 0, 0, 0, 0, 0, 0, 0, 0});
    assert(ets2[5] == 1);
    return 0;
}
