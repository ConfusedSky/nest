#!test
#!test
#!test=value
#!group(test)
#!group(test=value)
class Example {
    #!test=value
    method () { }

    #!export(name=test_call, arg="&str", arg="&str", arg="&str", arg="&str")
    static testCall(that, has, some, params) {}

    static printObject(o, depth) {
        if (o is Map) {
            this.pprint(o, depth+1)
        } else {
            System.write(o)
        }
    }

    static pprint(map, depth) {
        System.print("{")

        for (entry in map) {
            System.write("  "*(depth + 1) + entry.key.toString + ": ")
            this.printObject(entry.value, depth)
            System.print(",")
        }

        System.write("  "*depth + "}")
    }

    static pprint(map) { 
        this.pprint(map, 0)
        System.print()
    }

    static test() {
        this.pprint(attributes.self)
        this.pprint(attributes.methods)
    }

    //!place_generated
}

Example.test()