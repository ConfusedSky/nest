var factorial = Fn.new {|x| 
    var result = 1
    while (x >= 2) {
        result = result * x
        x = x - 1
    }
    return result
}

System.print(factorial.call(10)) // expect: 3628800