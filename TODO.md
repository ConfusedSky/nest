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
    - This may not be true anymore, if we can use reflection from the C API this should not be a huge deal
  - Currently this can be done by using the scheduler without a
    promise
- [x] Create try_get for safe access of values
- [x] Add unknown location because abort fiber is only usable from within a foreign context
- [ ] Test at compile time that all foreign methods and classes are defined and hooked up
  - Not sure if this should happen as a build step or automatic test generation
- [x] Make sure handles are only usable in the VM that created them
- [x] Be able to run tests on both the rust side and the wren side
- [ ] Compiler doesn't warn when there are mutliple vm's trying to share a handle
- [ ] Make sure ForeignMethods must be of the same type of their context
- [ ] Use some methods that aren't in the public api to make better abstractions?
      Looks like it's possible to do typechecking in foreign functions after all
      It's just not possible with the current public API
      [mirror](https://github.com/joshgoebel/wren-essentials/blob/main/src/modules/mirror.c)
- [ ] Look into removing location dependency for GetValue and SetValue since it looks like
      it could be possible to do runtime typechecking and conversion without needing to call
      back into the vm
- [ ] Make relative imports canonicalize into paths relative to the cwd
      Right now they become absolute paths which makes stack traces harder to read
