import "random" for Random 

var random = Random.new(12345)
System.print(random.float(0))     //> 0
System.print(random.float(100))   //> 20.180515043262
System.print(random.float(-100))  //> -43.371948658705

random = Random.new(12345)
System.print(random.float(3, 4))    //> 3.5317879598062
System.print(random.float(-10, 10)) //> -5.9638969913476
System.print(random.float(-4, 2))   //> -1.3976830804777

random = Random.new(12345)
var list = (1..5).toList
random.shuffle(list)
System.print(list) //> [3, 2, 4, 1, 5]

for (i in 0..1e6) {
    var f1 = random.float(-4, 2)
    var f2 = random.float(2, -4)
    if (f1 < -4 || f1 > 2) {
        System.print("f1 is a failure")
    }

    if (f2 < -4 || f2 > 2) {
        System.print("f2 is a failure")
    }
}