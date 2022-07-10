var test = [1, 2, 3, 4, 5]
var mapped = test.map{|n| 
    System.print("Map: %(n)")
    return n * 2
}.take(2)
test[1] = 4

for (n in mapped) {
    System.print(n)
}

var other = 1

var otherValue = {"test": "other", other: "test"}
System.print(otherValue)

var kv = otherValue.keys.map {|key| "%(key): %(otherValue[key])"}
System.print("{%(kv.join(", "))}")
