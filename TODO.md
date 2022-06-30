- [x] Have wren_sys automatically build wren
- [ ] Allow wren to set a exit code
- [ ] Create a safe wrapper for foreign methods
  - Also allow normal foreign pointers to be passed in as well?
- [ ] Add additional contexts to make sure that we don't accidentally call from a
      foreign method. Right now if you do there is undefined behavior
- [ ] Figure out a way to allow calling back into wren from a foreign method
  - That is by default illegal but we want to be able to call wren methods when we are
    validating our methods
