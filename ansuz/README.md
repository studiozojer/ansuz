# Ansuz playground

The opening experience is the greyscale orbital study: rotate the sphere beneath a fixed camera, or move the camera around a fixed sphere at constant altitude. The fixed reticle highlights a cell; selection and content remain future work. “2D study” retains the tested native card playground.

`logic/src/orbit.rs` holds sphere coordinates, projection and ray intersection. `logic/src/lib.rs` handles touch/mouse motion, drift and camera transforms. `app.contract` supplies the controls and typography. Neither study loads or changes the old local-note file.

See the [repository README](../README.md) for building and testing, and [strata](../STRATA.爻) for prior experiments. Record identity remains independent of visual projection.
