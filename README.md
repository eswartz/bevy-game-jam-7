
# Fever

# WARNING
# THIS IS JANK and a Proof of Concept.
# It was published here AFTER the Jam submission deadline.
# It is a minimal effort to validate wasm builds and deployment.
# Contains some moderate to severe usability bugs not present in desktop.
# WARNING

This is my Bevy Game Jam 7 entry, ported to wasm.

## Wasm Build

I based this build on the submission-time source, changing:

* fix logging for wasm (added `console_log` crate)
* fix bugs preventing wasm+webgl2 from launching at all (turned off TSAA default)
* workaround webgl limitations (`Bloom` effects are used in an intentionally glitchy way using negative intensity, but the webgl2 GLSL conversion of the code resulted in mostly black lighting; using different settings there).
* In the intervening builds, I was trying webgpu, and soon revisiting the WebGPU config steps in various browsers, and discovering a slew of unresearched startup issues -- I decided to use the more reliable webgl2.

And adapting the webpage to be scary enough to deter bug reports ;)
