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
            return pprint(o, depth+1)
        } else {
            return o.toString
        }
    }

    static pprint(map, depth) {
        var output = "{\n"

        for (entry in map) {
            output = output + "  "*(depth + 1) + entry.key.toString + ": "
            output = output + printObject(entry.value, depth)
            output = output + ",\n"
        }

        return output + "  "*depth + "}"
    }

    static pprint(map) { 
        return pprint(map, 0)
    }

    static test() {
        var self = pprint(attributes.self)
        var methods = pprint(attributes.methods)
        System.print(self)
        System.print(methods)
    }

    //!place_generated
}

Example.test()