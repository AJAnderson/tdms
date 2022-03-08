Next Steps:
- [x] Workspace or otherwise separate app from library code
- [x] Change algorithm to minimize unnecessary object clones
- [x] Tidy up error handling
- [x] Lib helper method to convert to f64, theoretically can hold any value that a numeric could hold
- [ ] Remove previous channel on load (or hold multiple files)
- [x] Legend
- [ ] Axes labels (not supported by egui, need to upstream changes)
- [ ] Time series support
- [ ] Smart sub-sampling for speed (based on window size in pixels?)

Multiplot
- [x] Change channel interface to select multiple (checkbox)
- [x] Implement multiple plots

Check
- [ ] Speed/memory comparison against Speedo

Future state:
- [ ] Implement TypeVector for FixedPoint
- [ ] Implement TypeVector for ComplexSingleFloat
- [ ] Implement TypeVector for ComplexDoubleFloat
- [ ] Implement TypeVector for DAQmxRawData
- [ ] Implement TypeVector for ExtendedFloat
- [ ] Test case for flexlogger