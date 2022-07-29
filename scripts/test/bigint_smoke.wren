// TODO: replace and remove test when we have more foreign types
import "bigint" for BigInt, Test

System.print(BigInt.ZERO) // expect: 0
System.print(BigInt.ONE) // expect: 1
System.print(BigInt.new(10)) // expect: 10
System.print(BigInt.new("123456789")) // expect: 123456789
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
// expect: BigInt.setValue expects a BigInt, String or an Integer

System.print(Fiber.new {
    b.setValue(Test.new())
    System.print(b)
}.try())
// expect: BigInt.setValue expects a BigInt, String or an Integer

System.print(Fiber.new {
    b.setValue("This is a potato")
    System.print(b)
}.try())
// expect: Failed to parse "This is a potato" as an integer!

System.print(Fiber.new {
    b.setValue(12345)
    b = b + "12345"
    System.print(b)
}.try())
// expect: BigInt.+(_) expects a BigInt or an Integer

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
System.print(BigInt.fib(200))
// expect: 280571172992510140037611932413038677189525
System.print(BigInt.fastfib(200))
// expect: 280571172992510140037611932413038677189525

var fib5Helper
fib5Helper = Fn.new {|n|
    var ZERO = BigInt.ZERO
    var ONE = BigInt.ONE
    if (n == 0) {
        return [ZERO, ONE]
    } else {
        var res = fib5Helper.call((n / 2).floor)
        var c = res[0] * (res[1] * 2 - res[0])
        var d = res[0] * res[0] + res[1] * res[1]
        if (n % 2 == 0) {
            return [c, d]
        } else {
            return [d, c + d]
        }
    }
}
var fib5 = Fn.new {|n| 
    if (n < 1) {
        Fiber.abort("We don't support negative or 0 numbers here")
    } else {
        return fib5Helper.call(n)[0]
    }
}

System.print(fib5.call(1000))
// expect: 43466557686937456435688527675040625802564660517371780402481729089536555417949051890403879840079255169295922593080322634775209689623239873322471161642996440906533187938298969649928516003704476137795166849228875