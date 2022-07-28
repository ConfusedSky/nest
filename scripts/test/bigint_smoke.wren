// TODO: replace and remove test when we have more foreign types
import "bigint" for BigInt, Test

System.print(BigInt.ZERO) // expect: 0
System.print(BigInt.ONE) // expect: 1
System.print(BigInt.new(10)) // expect: 10
var b = BigInt.new()
b.setValue(10)
System.print(b)
// expect: 10
b.setValue(BigInt.new(25))
System.print(b)
// expect: 25

System.print(Fiber.new {
    b.setValue(12.5)
}.try())
// expect: BigInt.setValue expects a BigInt or an Integer

System.print(Fiber.new {
    b.setValue(Test.new())
    System.print(b)
}.try())
// expect: BigInt.setValue expects a BigInt or an Integer

System.print(Fiber.new {
    b.setValue("This is a potato")
    System.print(b)
}.try())
// expect: BigInt.setValue expects a BigInt or an Integer

var fib1 = Fn.new {|n|
    var a = BigInt.ZERO
    var b = BigInt.ONE

    for (i in 0...n) {
        var c = a
        a = b
        b = b + c
    }

    return a
}

System.print(fib1.call(200))
// expect: 280571172992510140037611932413038677189525