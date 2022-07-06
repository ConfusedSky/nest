- [x] Have wren_sys automatically build wren
- [ ] Allow wren to set a exit code
- [ ] Create a safe wrapper for foreign methods
  - Also allow normal foreign pointers to be passed in as well?
- [x] Add additional contexts (Native/Foreign/Unknown) to make sure that we don't
      accidentally call from a foreign method. Right now if you do there is
      undefined behavior
- [ ] Figure out a way to allow calling back into wren from a foreign method
  - That is by default illegal but we want to be able to call wren methods when we are
    validating our methods
  - Currently this can be done by
- [x] Create try_get for safe access of values
- [x] Add unknown location because abort fiber is only usable from within a foreign context
- [ ] Test at compile time that all foreign methods and classes are defined and hooked up
  - Not sure if this should happen as a build step or automatic test generation
- [x] Make sure handles are only usable in the VM that created them
- [x] Be able to run tests on both the rust side and the wren side
- [ ] Compiler doesn't warn when there are mutliple vm's trying to share a handle
