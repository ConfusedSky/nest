import "random" for Random 

// Single number seeds aren't determinisitc because they rely
// on srand, so different platforms might result in a different
// value
var random = Random.new([12, 345, 7])
System.print(random.float(0))     // expect: 0
System.print(random.float(100))   // expect:  58.0689036433
System.print(random.float(-100))  // expect: -96.948249553706

random = Random.new([12, 345, 7])
System.print(random.float(3, 4))    // expect: 3.8333154359434
System.print(random.float(-10, 10)) // expect: 1.61378072866
System.print(random.float(-4, 2))   // expect: 1.8168949732224

random = Random.new([12, 345, 7])
var list = (1..5).toList
random.shuffle(list)
System.print(list) // expect: [5, 4, 1, 2, 3]

for (i in 0..1e4) {
    var f1 = random.float(-4, 2)
    var f2 = random.float(2, -4)
    if (f1 < -4 || f1 > 2) {
        System.print("f1 is a failure")
    }

    if (f2 < -4 || f2 > 2) {
        System.print("f2 is a failure")
    }
}