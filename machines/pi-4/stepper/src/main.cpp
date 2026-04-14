#include <iostream>

#include "Motor.hpp"
#include "constants.hpp"

int main()
{

    bool turnForeward = false;

    Motor motor(constants::MICROSTEPS_PER_REV, constants::STEP_PIN, constants::DIR_PIN, constants::INIT_PWM_DELAY, constants::GPIO_CONTROLLER_PATH, turnForeward);

    motor.drive();
    
    while (motor.getSteps() != 2000 || !motor.atSetpoint())
    {
        std::cout << motor.getSteps() << "\n\r";
    }


    motor.~Motor();
}