import "os" for Process, Platform
import "io" for Stdout

System.print(Process.allArguments)
System.print(Process.arguments)
System.print(Process.version)
System.print(Process.cwd)
System.print(Process.pid)

System.print(Platform.isPosix)
System.print(Platform.name)
System.print(Platform.homePath)
Stdout.flush()
