var factorial 
factorial = Fn.new{|x| 
    if (x == 0) return 1
    return x * factorial.call(x-1)
}
System.print(factorial.call(15))
