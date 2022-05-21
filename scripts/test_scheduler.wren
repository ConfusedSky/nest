import "scheduler" for Scheduler
import "timer" for Timer

var x = 10
Scheduler.add {
    x = x * 2
}

Scheduler.add {
    Timer.sleep(1000)
    System.print("In Task 1")
    Timer.sleep(500)
    System.print("End Task 1")
}

Scheduler.add {
    System.print("In Task 2")
    Timer.sleep(50)
    System.print("End Task 2")

    Scheduler.add {
        System.print("This should happen almost immediately")
    }
}

Scheduler.add {
    System.print("In Task 3")
    Timer.sleep(1000)
    System.print("End Task 3")

    Scheduler.add {
        System.print("Will this one be scheduled?")
    }
}

System.print(x)
Timer.sleep(100)
System.print(x)
Scheduler.awaitAll()
System.print("Finished")
