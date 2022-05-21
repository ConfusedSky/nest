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

    Scheduler.awaitAll()

    Scheduler.add {
        System.print("Will this one be scheduled?")
    }
}

System.print(x)
Scheduler.runScheduled()
System.print(x)

Scheduler.awaitAll()
System.print("Finished")

// This is required here because the async loop has been stopped
// and we need to hand control back to the scheduler to make sure
// all tasks are completed after await all on line 32
Scheduler.runScheduled()

// Maybe the scheduler logic should all be moved over to the runtime so
// we aren't that we can guarentee all of the scheduled items are run 
// before the program exits
