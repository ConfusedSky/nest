var factorial1 = Fn.new {|x| (1..x).reduce {|y, z| y * z} }

var factorial2 
factorial2 = Fn.new{|x| 
    if (x == 0) return 1
    return x * factorial2.call(x-1)
}

var factorial3 = Fn.new {|x| 
    var result = 1
    for (i in (2..x)) {
        result = result * i
    }
    return result
}

var factorial4 = Fn.new {|x| 
    var result = 1
    while (x >= 2) {
        result = result * x
        x = x - 1
    }
    return result
}


var count = 5000000
var value = 10

System.print(factorial1.call(value))
System.print(factorial2.call(value))
System.print(factorial3.call(value))
System.print(factorial4.call(value))

var start = System.clock
for (i in (0..count)) {
    factorial1.call(value)
}
System.print(System.clock - start)
start = System.clock
for (i in (0..count)) {
    factorial2.call(value)
}
System.print(System.clock - start)
start = System.clock
for (i in (0..count)) {
    factorial3.call(value)
}
System.print(System.clock - start)
start = System.clock
for (i in (0..count)) {
    factorial4.call(value)
}
System.print(System.clock - start)