#include <iostream>
#include <gpiod.hpp>

#include "motor_clock.hpp"
#include "constants.hpp"

using namespace std::chrono_literals;

int main()
{
    bool exit = false;

    motor_clock clk(300us); //Create a clock to guide the motor

    gpiod::chip gpio_ctrl(constants::GPIO_CONTROLLER_PATH); //GPIO chip

    gpiod::line_settings settings;
    settings.set_direction(gpiod::line::direction::OUTPUT); //Set pins for taking requests
    settings.set_output_value(gpiod::line::value::INACTIVE); //By default, be inactive

    //Top magnet creation
    gpiod::line_config top_config;
    top_config.add_line_settings(constants::TOP_MAG_PIN, settings); //configure the associated request with the top magnet GPIO pin and the preconfigured settings

    gpiod::request_builder top_builder = gpio_ctrl.prepare_request(); //Create a request builder from the GPIO chip
    top_builder.set_line_config(top_config); //Set the request builder's config to be the previously created config

    //Left magnet creation
    gpiod::line_config left_config;
    top_config.add_line_settings(constants::LEFT_MAG_PIN, settings);

    gpiod::request_builder left_builder = gpio_ctrl.prepare_request();
    top_builder.set_line_config(left_config);

    //Right magnet creation
    gpiod::line_config right_config;
    right_config.add_line_settings(constants::RIGHT_MAG_PIN, settings);

    gpiod::request_builder right_builder = gpio_ctrl.prepare_request();
    right_builder.set_line_config(left_config);

    //Bottom magnet creation
    gpiod::line_config bottom_config;
    bottom_config.add_line_settings(constants::BOTTOM_MAG_PIN, settings);

    gpiod::request_builder bottom_builder = gpio_ctrl.prepare_request();
    bottom_builder.set_line_config(left_config);

    //An array of line requests for each magnet.
    gpiod::line_request mag_requests[4] = { top_builder.do_request(), right_builder.do_request(),
                                             bottom_builder.do_request(), left_builder.do_request() };

    unsigned int microstep_point = 0;
    unsigned int revs = 0;
    unsigned int microsteps = 0;

    while (!exit)
    {

        if (clk.pastDelay())
        {
            //Apply step sequence value to each magnet
            for (int i = 0; i < 4; i++) //4 magnets
            {
                mag_requests[i].set_value(constants::MAG_OFFSETS[i], gpiod::line::value(constants::STEP_SEQUENCE[microstep_point][i]));
            }

            microstep_point = (microstep_point + 1) % 8;
            microsteps++;

            if (microsteps % constants::MICROSTEPS_PER_REV == 0)
            {
                revs++;
                std::cout << "Revolutions: " << revs << std::endl; //Output revs
            }
        }

        //Exit conditions(s)
        exit = revs >= 40;
    }

    // Release line requests
    for (int i = 0; i < 4; i++)
    {
        mag_requests[i].release();
    }
}