#!/usr/bin/env -S cargo run --release
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

    static spaces(depth) { "  "*depth }

    static printEntry(entry, depth) { 
        return spaces(depth+1) + entry.key.toString + ": " + printObject(entry.value, depth)
    }

    static pprint(map, depth) {
        var output = "{\n"

        var mapper = Fn.new {|entry| printEntry(entry, depth)}

        output = output + map.map(mapper).join(",\n") + "\n"

        return output + spaces(depth) + "}"
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

// expect: {
// expect:   null: {
// expect:     test: [null, null, value]
// expect:   },
// expect:   group: {
// expect:     test: [null, value]
// expect:   }
// expect: }
// expect: {
// expect:   static testCall(_,_,_,_): {
// expect:     export: {
// expect:       name: [test_call],
// expect:       arg: [&str, &str, &str, &str]
// expect:     }
// expect:   },
// expect:   method(): {
// expect:     null: {
// expect:       test: [value]
// expect:     }
// expect:   }
// expect: }