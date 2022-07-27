// TODO: replace and remove test when we have more foreign types
import "bigint" for BigInt, Test

System.print(BigInt.ONE) // expect: 1
System.print(BigInt.ZERO) // expect: 0
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
