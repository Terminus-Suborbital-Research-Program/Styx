#include <filesystem>
#include <gpiod.hpp>

namespace constants
{
    const std::filesystem::path GPIO_CONTROLLER_PATH = "/dev/gpiochip0";

    const unsigned int TOP_MAG_PIN = 26;
    const unsigned int RIGHT_MAG_PIN = 0;
    const unsigned int LEFT_MAG_PIN = 0;
    const unsigned int BOTTOM_MAG_PIN = 0;

    const unsigned int MICROSTEPS_PER_REV = 200;

    const int STEP_SEQUENCE[8][4] = {
        {1, 0, 0, 0},
        {1, 1, 0, 0},
        {0, 1, 0, 0},
        {0, 1, 1, 0},
        {0, 0, 1, 0},
        {0, 0, 1, 1},
        {0, 0, 0, 1},
        {1, 0, 0, 1},
    };

    const gpiod::line::offset MAG_OFFSETS[4] = { gpiod::line::offset(TOP_MAG_PIN), gpiod::line::offset(RIGHT_MAG_PIN),
                                                 gpiod::line::offset(BOTTOM_MAG_PIN), gpiod::line::offset(LEFT_MAG_PIN) };
}