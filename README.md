# Xyron Vulkan Core ⚙️

**Xyron Vulkan Core** is a high-performance local inference engine written natively in Rust. It is designed to maximize VRAM usage on graphics cards (especially optimized for the 16GB AMD Radeon RX 580) using the Vulkan API, enabling 100% local AI image and video generation without relying on the cloud.

## 🚫 Zero CUDA Dependencies (Hardware Freedom)

Unlike the vast majority of modern AI video generation tools that lock developers into the NVIDIA ecosystem, **Xyron Vulkan Core** is built from the ground up on Vulkan. 
* **No CUDA Required:** Run heavy models like Wan 2.1 I2V without needing an expensive RTX card.
* **AMD Friendly:** Fully optimized to squeeze every megabyte of VRAM out of AMD GPUs.
* **Bare-Metal Performance:** Direct hardware access via Rust and the Vulkan API beats brute-force hardware requirements through smart, low-level optimization.

## 🧠 System Architecture (v1.0)

This project is divided into two main components that separate the interface logic from the heavy GPU processing:

* **The Graphics Core (`lib.rs` -> `motor.dll`):** 
  This is the heart of the system. Written in Rust and compiled as a dynamic library (DLL), it handles direct communication with Vulkan. It loads model tensors in `.gguf` format directly into video memory for fast inference execution. It supports diffusion models such as **Flux** (image generation) and **Wan 2.1 I2V** (video animation).
* **The Orchestrator (`main.rs`):** 
  The main executable that acts as a bridge. It currently connects via HTTP requests to **LM Studio**, processing the directives of the "Aivi" assistant to translate them into rendering commands (`cmd:vulkan` and `cmd:wan`) that are sent to `motor.dll`.

## 🚀 Key Features

- **Hybrid Rendering:** Generation of static images (Text-to-Image) and video sequences (Image-to-Video).
- **Aggressive Memory Management:** Keeps the model in VRAM only during inference to prevent system crashes on mid-range hardware.
- **Batch Processing:** Ability to process entire folders of images (`cmd:batch_wan`), automating the creation of animated episodes.
- **Local LLM Integration:** Native connection with language models via LM Studio for automated scene direction.

## 📂 Required Directory Structure

For the engine to work, the runtime environment must respect this structure:

```text
xyron_vulkan_core/
├── src/                # Rust source code (main.rs, lib.rs)
├── models/             # (Not included) .gguf models go here (e.g., flux, wan2.1)
├── personajes/         # Input folder for images to animate
├── aivi.exe            # Main compiled executable
└── motor.dll           # Dynamic library compiled from lib.rs