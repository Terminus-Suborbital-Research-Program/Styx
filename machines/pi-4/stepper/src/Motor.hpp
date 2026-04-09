#include <iostream>
#include <gpiod.hpp>
#include <vector>
#include <chrono>
#include <filesystem>
#include <thread>
#include <algorithm>
#include <atomic>

#include "MotorClock.hpp"

//External dependency: libgpiod w/ C++ bindings

/*
Motor class

Controls a step/dir stepper motor driver using libgpiod.
A background thread generates step pulses based on a timing source (MotorClock).

Optional PID control allows dynamic adjustment of the step period in order to
reach either a time-based or step-based setpoint.
*/
class Motor
{
public:

    /*
    Defines what kind of setpoint the controller is targeting.
    */
    enum class SetpointType
    {
        kSTEP,   // Stop after a number of microsteps
        kTIMER,  // Stop after a specified time duration
        kNONE    // No setpoint active
    };

private:

    /*
    Internal PID controller used to adjust motor step timing.

    The PID output is interpreted as a delay duration between steps.
    A negative output indicates that the motor should reverse direction.
    */
    struct PID
    {
        double P_constant, I_constant, D_constant;  // PID coefficients

        // Time tracking used for timer setpoints
        std::chrono::time_point<std::chrono::steady_clock> time_sp, last_time, now_time;

        // Error values for time-based control
        std::chrono::microseconds time_sp_error, last_time_sp_error;

        // Time between PID updates
        std::chrono::microseconds time_difference;

        // Maximum allowed magnitude of PID output
        std::chrono::microseconds output_max;

        // Step setpoint and error tracking
        int step_sp, last_step, last_step_sp_error, step_sp_error;

        // Reference to the motor's step counter
        std::atomic<int>& steps;

        // Reference to the motor direction flag
        std::atomic<bool>& reverse;

        // Integral accumulator
        std::chrono::microseconds i_sum;

        // Current setpoint type
        SetpointType sp_type;

        // Reference to shared timing object
        MotorClock& clk;

        PID(double P, double I, double D, MotorClock& clock,
            std::atomic<bool>& reverse,
            std::atomic<int>& steps,
            std::chrono::microseconds output_max);

        std::chrono::microseconds outputP();
        std::chrono::microseconds outputI();
        std::chrono::microseconds outputD();

        /*
        Computes the PID output.

        Returns a delay duration for the motor clock.
        Sign of the output determines motor direction.
        */
        std::chrono::microseconds calculate();
    };

    // GPIO chip interface (libgpiod)
    gpiod::chip m_chip;

    // Timing controller used for step delays
    MotorClock m_clk;

    // Control flags shared with the drive thread
    std::atomic<bool> m_driving;
    std::atomic<bool> m_reverse;

    // Enables/disables PID control
    bool m_using_pid;

    // Steps per revolution of the motor
    const unsigned int m_resolution;

    // Total microsteps executed (atomic because thread updates it)
    std::atomic<int> m_microsteps;

    // Additional step tracking variables
    int m_microstep_point, m_revs, m_step_setpoint;

    // GPIO offsets for the step and direction pins
    gpiod::line::offset m_step_pin, m_dir_pin;

    // Active GPIO request controlling the lines
    gpiod::line_request* m_request;

    // Active setpoint type
    SetpointType m_setpoint_type;

    // Embedded PID controller
    PID m_pid;

    // Worker thread responsible for generating step pulses
    std::thread m_drive_thread;

    /*
    Thread loop responsible for producing motor step pulses.
    */
    void drive_thread_func();

    /*
    Set STEP pin HIGH while updating direction pin.
    */
    void stepHigh();

    /*
    Set STEP pin LOW while updating direction pin.
    */
    void stepLow();

public:

    /*
    Constructor

    @param step_resolution  steps per revolution
    @param step_pin         GPIO line offset for step signal
    @param dir_pin          GPIO line offset for direction signal
    @param PWM              initial step delay
    @param gpio_chip_path   path to the GPIO chip (e.g. /dev/gpiochip0)
    */
    Motor(const unsigned int step_resolution,
          const gpiod::line::offset step_pin,
          const gpiod::line::offset dir_pin,
          const std::chrono::microseconds PWM,
          std::filesystem::path gpio_chip_path);

    /*
    Destructor

    Stops the drive thread and releases GPIO ownership.
    */
    ~Motor();

    /*
    Starts the drive thread if it is not already running.
    */
    void drive();

    /*
    Signals the drive thread to stop.
    */
    void stop();

    // Configuration methods

    void setSetpointType(SetpointType type);
    void setStepSetpoint(int steps, bool set_type = false);
    void setRevSetpoint(int revs, bool set_type = false);
    void setTimerSetpoint(std::chrono::microseconds time_delay, bool set_type = false);
    void setPWM(std::chrono::microseconds PWM);
    void setPID(double P, double I, double D, std::chrono::microseconds output_max);

    // Enable or disable PID control
    void usePID(bool state);

    /*
    Returns true when the configured setpoint condition has been reached.
    */
    bool atSetpoint();

    /*
    Returns the step counter value.
    */
    int getSteps();

    /*
    Returns a copy of the current MotorClock.
    */
    MotorClock getCurrentClock();
};