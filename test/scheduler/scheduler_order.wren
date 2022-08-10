import "scheduler" for Scheduler
import "timer" for Timer

var x = 10

Scheduler.add {
    x = x * 2
}

Scheduler.add {
    Timer.sleep(10)
    System.print("In Task 1") // expect[6]: In Task 1
    Timer.sleep(5)
    System.print("End Task 1") // expect[8]: End Task 1
}

Scheduler.add {
    System.print("In Task 2") // expect[1]: In Task 2
    Timer.sleep(2)
    System.print("End Task 2") // expect[4]: End Task 2

    Scheduler.add {
        System.print("This should happen almost immediately") 
        // expect[5]: This should happen almost immediately
    }
}

Scheduler.add {
    System.print("In Task 3") // expect[2]: In Task 3
    Timer.sleep(10)
    System.print("End Task 3") // expect[7]: End Task 3

    Scheduler.awaitAll()

    Scheduler.add {
        System.print("Will this one be scheduled?")
        // NOTE: an expecation with no number
        // will be checked in line number
        // order after all the numbered ones
        // So this should happen last
        // expect: Will this one be scheduled?
    }
}

System.print(x) // expect[0]: 10
Scheduler.runScheduled()
System.print(x) // expect[3]: 20

Scheduler.awaitAll()
System.print("Finished") // expect[9]: Finished
