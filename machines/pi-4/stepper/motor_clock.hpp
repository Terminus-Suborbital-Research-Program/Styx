#include <chrono>

// 200 steps per revolution

class motor_clock
{
    std::chrono::steady_clock clock;
    std::chrono::time_point<std::chrono::steady_clock> last, now;
    std::chrono::microseconds delay;


    public:

    /*
    * @param delay_length how long to delay between each microstep
    */
    inline motor_clock(std::chrono::microseconds delay_length)
    : delay(delay_length)
    {

        last = clock.now();
        now = clock.now();
    }

    inline bool pastDelay()
    {
        now = clock.now();

        if ( (now - last) >= delay )
        {
            last = clock.now();
            return true;
        }

        return false;
    }

    inline std::chrono::steady_clock getClock() { return clock; }
    inline long getTimeDifference() { return (now - last).count(); }
    inline long getDelay() { return delay.count(); }
};