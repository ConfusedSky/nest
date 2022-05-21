class Scheduler {
  static add(callable) {
    if (__scheduled == null) __scheduled = []

    __scheduled.add(Fiber.new {
      callable.call()
      runNextScheduled_()
    })
  }

  // Hands control from this Fiber to the scheduler and lets it run
  // Effectively does the same thing as `Timer.sleep(0)`
  // TODO: Implement this without requiring a timer to be built
  static runScheduled() {
    return Timer.sleep(0)
  }

  // Wait for all scheduled async tasks to complete before rescheduling this fiber to
  // return from this function
  static awaitAll() {
    if (__waiting == null) __waiting = []
    __waiting.add(Fiber.current)
    awaitAll_()
    return runNextScheduled_()
  }

  // Called by native code.
  static resume_(fiber) { fiber.transfer() }
  static resume_(fiber, arg) { fiber.transfer(arg) }
  static resumeError_(fiber, error) { fiber.transferError(error) }
  static resumeWaitingFibers_() {
    if (__waiting != null) {
      // Reschedule all waiting fibers
      for (fiber in __waiting) {
        add {
          fiber.transfer()
        }
      }
      // Clear all fibers 
      __waiting.clear()
    }
    // Run the next scheduled fiber
    return runNextScheduled_()
  }

  // wait for a method to finish that has a callback on the C side
  static await_(fn) {
    fn.call()
    return runNextScheduled_()
  }

  static hasNext_ {
    return __scheduled != null && !__scheduled.isEmpty
  }

  static runNextScheduled_() {
    if (hasNext_) {
      return __scheduled.removeAt(0).transfer()
    } else {
      return Fiber.suspend()
    }
  }

  foreign static captureMethods_()
  foreign static awaitAll_()
}

// Timer must be imported after scheduler is defined because of the way imports work
import "timer" for Timer


Scheduler.captureMethods_()