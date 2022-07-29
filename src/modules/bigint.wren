foreign class BigInt {
    foreign static ZERO
    foreign static ONE

    construct new() {}
    foreign toString
    foreign +(value)
    foreign -(value)
    foreign *(value)
    foreign pow(value)
    foreign setValue(value)

    foreign static fib(n)
    foreign static fastfib(n)
    foreign static new(value) 
}

foreign class Test {
    construct new() {}
}