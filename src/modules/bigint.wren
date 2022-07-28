foreign class BigInt {
    foreign static ZERO
    foreign static ONE

    construct new() {}
    static new(value) {
        var res = BigInt.new()
        res.setValue(value)
        return res
    }
    foreign toString
    foreign +(value)
    foreign -(value)
    foreign *(value)
    foreign setValue(value)

    foreign static fib(n)
    foreign static fastfib(n)
}

foreign class Test {
    construct new() {}
}