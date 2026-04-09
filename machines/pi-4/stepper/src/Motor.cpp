#include "Motor.hpp"

using namespace std::chrono_literals;

/*
Constructor

Initializes the Motor object, sets up GPIO control for the STEP and DIR pins,
and prepares the internal timing and PID controller used to drive the motor.
*/
Motor::Motor(const unsigned int step_resolution, const gpiod::line::offset step_pin, const gpiod::line::offset dir_pin, const std::chrono::microseconds PWM,
        std::filesystem::path gpio_chip_path)
    : m_chip(gpio_chip_path)          // Open the specified GPIO chip (e.g., /dev/gpiochip0)
    , m_driving(false)                // Drive thread initially inactive
    , m_using_pid(false)              // PID disabled by default
    , m_resolution(step_resolution)   // Motor steps per revolution
    , m_microstep_point(0)
    , m_revs(0)
    , m_microsteps(0)                 // Total steps executed since start
    , m_step_setpoint(0)
    , m_setpoint_type(Motor::SetpointType::kNONE)
    , m_step_pin(step_pin)            // STEP signal GPIO
    , m_dir_pin(dir_pin)              // DIR signal GPIO
    , m_reverse(false)                // Direction flag (false = forward)
    , m_clk(PWM)                      // MotorClock controlling delay between steps
    , m_pid(0.0, 0.0, 0.0, m_clk, m_reverse, m_microsteps, 0us)
    , m_drive_thread()
{
    /*
    Configure GPIO pins for output.
    Both STEP and DIR pins are controlled through a single line request.
    */

    gpiod::line_settings settings;
    settings.set_direction(gpiod::line::direction::OUTPUT);  // Configure lines as outputs
    settings.set_output_value(gpiod::line::value::INACTIVE); // Default LOW

    gpiod::line_config config;

    // Add both step and direction lines to the configuration
    config.add_line_settings(gpiod::line::offsets({m_step_pin, m_dir_pin}), settings);

    /*
    Prepare the request that transfers control of the GPIO lines to this process.
    */
    gpiod::request_builder builder = m_chip.prepare_request();
    builder.set_line_config(config);

    // Allocate the request dynamically
    m_request = new gpiod::line_request(builder.do_request());
}

/*
Destructor

Stops the drive thread and releases ownership of the GPIO lines.
Failing to do this can leave the lines locked by the process.
*/
Motor::~Motor()
{
    m_driving = false;

    // Ensure the worker thread has terminated before destroying resources
    if (m_drive_thread.joinable())
    {
        m_drive_thread.join();
    }

    // Release GPIO control
    m_request->release();
    delete m_request;
}

/*
PID Constructor

Initializes the PID controller used to dynamically adjust step timing.
*/
Motor::PID::PID(double P, double I, double D, MotorClock& clock, std::atomic<bool>& reverse, std::atomic<int>& steps, std::chrono::microseconds output_max)
: P_constant(P)
, I_constant(I)
, D_constant(D)
, last_time()
, now_time()
, time_sp()
, time_sp_error()
, last_time_sp_error()
, step_sp(0)
, last_step(0)
, step_sp_error(0)
, last_step_sp_error(0)
, steps(steps)      // Reference to the motor's step counter
, reverse(reverse)  // Reference to direction flag
, time_difference()
, i_sum()
, sp_type(SetpointType::kNONE)
, clk(clock)
{
    // Initialize time references
    now_time = last_time = clk.getClock().now();
}

/*
Proportional component of PID.
Produces output proportional to the current error.
*/
std::chrono::microseconds Motor::PID::outputP()
{
    switch (sp_type)
    {
        case SetpointType::kTIMER:
            return std::chrono::microseconds(int(P_constant * double(time_sp_error.count())));

        case SetpointType::kSTEP:
            return std::chrono::microseconds(int(P_constant * step_sp_error));

        default:
            return std::chrono::microseconds(0);
    }
}

/*
Integral component of PID.

Accumulates past errors over time to eliminate steady-state error.
*/
std::chrono::microseconds Motor::PID::outputI()
{
    std::chrono::microseconds add;

    switch (sp_type)
    {
        case SetpointType::kTIMER:
            add = std::chrono::microseconds(int(
                I_constant * double(time_difference.count()) *
                (time_sp_error.count())));
            break;

        case SetpointType::kSTEP:
            add = std::chrono::microseconds(int(
                I_constant * double(time_difference.count()) *
                (double(step_sp_error))));
            break;

        default:
            return std::chrono::microseconds(0);
    }

    // Accumulate integral error
    i_sum += add;

    return i_sum;
}

/*
Derivative component of PID.

Responds to the rate of change of the error.
Helps damp oscillations and improve stability.
*/
std::chrono::microseconds Motor::PID::outputD()
{
    // Prevent divide-by-zero
    if (time_difference.count() == 0)
    {
        return std::chrono::microseconds(0);
    }

    switch (sp_type)
    {
        case SetpointType::kTIMER:
            return std::chrono::microseconds(int(
                D_constant *
                double((time_sp_error - last_time_sp_error).count()) /
                double(time_difference.count())));

        case SetpointType::kSTEP:
            return std::chrono::microseconds(int(
                D_constant *
                double(step_sp_error - last_step_sp_error) /
                double(time_difference.count())));

        case SetpointType::kNONE:
            return std::chrono::microseconds(0);
    }
}

/*
Compute the final PID output.

The output represents a delay value used to control motor speed.
A negative output indicates reverse direction.
*/
std::chrono::microseconds Motor::PID::calculate()
{
    now_time = clk.getClock().now();

    // Calculate errors
    time_sp_error = std::chrono::microseconds((time_sp - last_time).count());
    time_difference = std::chrono::microseconds((now_time - last_time).count());
    step_sp_error = step_sp - steps;

    // Sum PID components
    std::chrono::microseconds output = outputP() + outputI() + outputD();

    // Store previous values for next derivative calculation
    last_time = now_time;
    last_step = steps;
    last_step_sp_error = step_sp_error;
    last_time_sp_error = time_sp_error;

    // Limit output to prevent instability
    output = std::clamp(output, -output_max, output_max);

    // Determine motor direction
    reverse = (output < 0us);

    // Return absolute delay value
    return std::chrono::abs(output);
}

/*
Check whether the current control target has been reached.
*/
bool Motor::atSetpoint()
{
    switch (m_setpoint_type)
    {
        case SetpointType::kTIMER:
            return m_clk.pastTimer();

        case SetpointType::kSTEP:
            return (m_microsteps >= m_step_setpoint);

        default:
            return false;
    }
}

/*
 Returns the step counter value.
*/
int Motor::getSteps()
{
    return m_microsteps;
}

/*
Enable or disable PID control.
*/
void Motor::usePID(bool state)
{
    m_using_pid = state;
}

/*
Raise the STEP signal.

A rising edge on STEP causes the stepper driver to advance one step.
DIR is updated at the same time to ensure correct direction.
*/
void Motor::stepHigh()
{
    m_request->set_values(gpiod::line::value_mappings({
        gpiod::line::value_mapping(m_step_pin, gpiod::line::value(1)),
        gpiod::line::value_mapping(m_dir_pin, gpiod::line::value(int(m_reverse)))
    }));
}

/*
Lower the STEP signal.

Completes the step pulse cycle.
*/
void Motor::stepLow()
{
    m_request->set_values(gpiod::line::value_mappings({
        gpiod::line::value_mapping(m_step_pin, gpiod::line::value(0)),
        gpiod::line::value_mapping(m_dir_pin, gpiod::line::value(int(m_reverse)))
    }));
}

/*
Drive thread loop.

Continuously generates step pulses while m_driving is true.
Behavior depends on whether PID control is enabled.
*/
void Motor::drive_thread_func()
{
    while (m_driving)
    {
        if (m_using_pid)
        {
            /*
            PID Mode

            Motor speed dynamically adjusts based on PID output.
            */

            if (atSetpoint())
            {
                // Stop stepping when the target condition is reached
                stepLow();

                // Reset to a safe idle delay
                m_clk.setDelay(200us);
            }
            else
            {
                // Update step timing based on PID output
                m_clk.setDelay(m_pid.calculate());
            }
        }
        else
        {
            /*
            Simple PWM Mode

            Step pulses are generated at a constant rate defined by m_clk.
            */

            stepHigh();
            std::this_thread::sleep_for(m_clk.getDelay());
            stepLow();
        }
    }
}

/*
Start the drive thread if it is not already running.
*/
void Motor::drive()
{
    if (m_drive_thread.joinable())
        return;

    m_driving = true;

    m_drive_thread = std::thread(&Motor::drive_thread_func, this);
}

/*
Signal the drive thread to stop.
*/
void Motor::stop()
{
    m_driving = false;
}

/*
Set the type of setpoint used by both the motor and the PID controller.
*/
void Motor::setSetpointType(SetpointType type)
{
    m_setpoint_type = type;
    m_pid.sp_type = type;
}

/*
Set a step-count target.
*/
void Motor::setStepSetpoint(int steps, bool set_type)
{
    m_step_setpoint = steps;

    if (set_type)
    {
        setSetpointType(Motor::SetpointType::kSTEP);
    }
}

/*
Set a target number of revolutions.
*/
void Motor::setRevSetpoint(int revs, bool set_type)
{
    setStepSetpoint(revs * m_resolution, set_type);
}

/*
Set a timer-based stopping condition.
*/
void Motor::setTimerSetpoint(std::chrono::microseconds time_delay, bool set_type)
{
    m_clk.setTimer(time_delay);

    if (set_type)
    {
        setSetpointType(Motor::SetpointType::kTIMER);
    }
}

/*
Directly set the step delay (used when PID is disabled).
*/
void Motor::setPWM(std::chrono::microseconds PWM)
{
    m_clk.setDelay(PWM);
}

/*
Configure PID constants and output limits.
*/
void Motor::setPID(double P, double I, double D, std::chrono::microseconds output_max)
{
    m_pid.P_constant = P;
    m_pid.I_constant = I;
    m_pid.D_constant = D;
    m_pid.output_max = output_max;
}
