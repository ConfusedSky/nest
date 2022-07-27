foreign class BigInt {
    static ZERO { __ZERO || (__ZERO = BigInt.new(0)) }
    static ONE { __ONE || (__ONE = BigInt.new(1)) }

    construct new() {}
    static new(value) {
        var res = BigInt.new()
        res.setValue(value)
        return res
    }
    foreign toString
    // foreign +(value)
    // foreign -(value)
    // foreign *(value)
    foreign setValue(value)
}

foreign class Test {
    construct new() {}
}