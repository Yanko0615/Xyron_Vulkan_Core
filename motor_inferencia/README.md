# ⚠️ IMPORTANT: Custom Modified Version

**DO NOT** replace or update the contents of this folder with the upstream/original `stable-diffusion.cpp` repository.

This specific version of `stable-diffusion.cpp` has been **heavily modified and restructured** at the source code level to enable compatibility and optimal execution of **Flux** models through the Xyron Vulkan Core engine. 

Overwriting these files with the standard repository will break the inference engine, Vulkan memory allocation, and the custom Rust bindings (`motor.dll`).
