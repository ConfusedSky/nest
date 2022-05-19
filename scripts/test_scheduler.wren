import "scheduler" for Scheduler

var x = 10
Scheduler.add {
    x = x * 2
}
System.print(x)
