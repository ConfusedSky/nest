import "../hello"
// expect: I am running in Rust after being loaded from a file!

// Note that there is also a system print in the factorial module
// So we need another expectation
import "./factorial" for factorial
// expect: 3628800

System.print(factorial.call(10)) // expect: 3628800