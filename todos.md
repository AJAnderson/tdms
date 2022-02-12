Todo items ("MVP"):
- [x] Drill into the iteration logic for segments and nail it down, handle early return for malformed segments (DONE)
- [ ] interleaved data (new test file needed)
- [x] interleaved read setup
- [ ] interleaved data presentation (read_vector update) 
- [x] understand and verify chunk size handling for channels which are added (new test file needed) (DONE)
- [ ] ~~refactor algorithm to avoid multiple passes over object list (related to chunk size handling) (WONT DO)~~
- [x] separate out algorithm concerns into functions, i.e. update index step (WONT DO)
- [x] Handle DAQmx data types (DONE)
- [ ] Test cases for each data type (specifically need to verify things like time stamps)
- [ ] Test case for flexlogger
- [ ] String vector read is incorrect
- [x] Read data vector is overloaded, the entry gates and object finding stuff shouldn't live there.
- [ ] Understand how strings actually interleave, is length at the start of every byte block?

Future state:
- [ ] Smart sub-sampling for speed (based on window size in pixels?)
- [ ] Box zoom
- [ ] Pretty up the channel interface (tick box to activate channels?)
- [ ] Speed comparison against Speedo