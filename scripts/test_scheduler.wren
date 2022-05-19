import "scheduler" for Scheduler
// import "timer" for Timer

var x = 10
Scheduler.add {
    x = x * 2
}
System.print(x)
// Timer.sleep(100)
// System.print(x)
