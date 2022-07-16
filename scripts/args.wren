#!/usr/bin/env -S cargo run --release
import "os" for Process, Platform
import "io" for Stdout

System.print(Process.allArguments)
System.print(Process.arguments)
System.print(Process.version)
System.print(Process.cwd)
System.print(Process.pid)

var ppid = Fiber.new {
    return Process.ppid
}.try()
System.print(ppid)

System.print(Platform.isPosix)
System.print(Platform.name)
System.print(Platform.homePath)

Stdout.flush()
