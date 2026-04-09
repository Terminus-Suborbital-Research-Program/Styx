#include <chrono>

// 200 steps per revolution

class MotorClock
{
    std::chrono::steady_clock clock;
    std::chrono::time_point<std::chrono::steady_clock> last, now, timer_point;
    std::chrono::microseconds delay;


    public:

    /*
    * @param delay_length how long to delay between each microstep
    */
    inline MotorClock(std::chrono::microseconds delay_length)
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

    inline bool pastTimer() 
    {
        now = clock.now();

        return now >= timer_point;

    }

    inline void setTimer(std::chrono::microseconds timer_delay) { timer_point = std::chrono::time_point<std::chrono::steady_clock>(now + timer_delay); }
    inline void setDelay(std::chrono::microseconds _delay) { delay = _delay; }

    inline std::chrono::steady_clock getClock() { return clock; }
    inline long getTimeDifference() { return (now - last).count(); }
    inline std::chrono::microseconds getDelay() { return delay; }
};