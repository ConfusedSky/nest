// class Stdin {
  // foreign static isRaw
  // foreign static isRaw=(value)
  // foreign static isTerminal

  // static readByte() {
    // return read_ {
      // // Peel off the first byte.
      // var byte = __buffered.bytes[0]
      // __buffered = __buffered[1..-1]
      // return byte
    // }
  // }

  // static readLine() {
    // return read_ {
      // // TODO: Handle Windows line separators.
      // var lineSeparator = __buffered.indexOf("\n")
      // if (lineSeparator == -1) return null

      // // Split the line at the separator.
      // var line = __buffered[0...lineSeparator]
      // __buffered = __buffered[lineSeparator + 1..-1]
      // return line
    // }
  // }

  // static read_(handleData) {
    // // See if we're already buffered enough to immediately produce a result.
    // if (__buffered != null && !__buffered.isEmpty) {
      // var result = handleData.call()
      // if (result != null) return result
    // }

    // if (__isClosed == true) Fiber.abort("Stdin was closed.")

    // // Otherwise, we need to wait for input to come in.
    // __handleData = handleData

    // // TODO: Error if other fiber is already waiting.
    // readStart_()

    // __waitingFiber = Fiber.current
    // var result = Scheduler.runNextScheduled_()

    // readStop_()
    // return result
  // }

  // static onData_(data) {
    // // If data is null, it means stdin just closed.
    // if (data == null) {
      // __isClosed = true
      // readStop_()

      // if (__buffered != null) {
        // // TODO: Is this correct for readByte()?
        // // Emit the last remaining bytes.
        // var result = __buffered
        // __buffered = null
        // __waitingFiber.transfer(result)
      // } else {
        // __waitingFiber.transferError("Stdin was closed.")
      // }
    // }

    // // Append to the buffer.
    // if (__buffered == null) {
      // __buffered = data
    // } else {
      // // TODO: Instead of concatenating strings each time, it's probably faster
      // // to keep a list of buffers and flatten lazily.
      // __buffered = __buffered + data
    // }

    // // Ask the data handler if we have a complete result now.
    // var result = __handleData.call()
    // if (result != null) __waitingFiber.transfer(result)
  // }

  // foreign static readStart_()
  // foreign static readStop_()
// }

class Stdout {
  foreign static flush()
}