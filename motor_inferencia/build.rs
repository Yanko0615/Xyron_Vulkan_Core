use std::env;
use std::path::PathBuf;

fn main() {
    // 1. Detectar automáticamente la ruta del Vulkan SDK en Windows
    if let Ok(vulkan_sdk) = env::var("VULKAN_SDK") {
        let vulkan_lib_path = PathBuf::from(vulkan_sdk).join("Lib");
        println!("cargo:rustc-link-search=native={}", vulkan_lib_path.display());
    } else {
        println!("cargo:rustc-link-search=native=C:/VulkanSDK/Lib");
    }

    // 2. Apuntar a nuestra carpeta local de librerías nativas
    println!("cargo:rustc-link-search=native=native");

    // 3. Enlazar la librería principal de Stable Diffusion y sus motores base de GGML
    println!("cargo:rustc-link-lib=static=stable-diffusion");
    println!("cargo:rustc-link-lib=static=ggml");
    println!("cargo:rustc-link-lib=static=ggml-base");
    println!("cargo:rustc-link-lib=static=ggml-cpu");
    println!("cargo:rustc-link-lib=static=ggml-vulkan");

    // 4. Dependencias obligatorias del sistema operativo y de gráficos en Windows
    println!("cargo:rustc-link-lib=dylib=vulkan-1");
    println!("cargo:rustc-link-lib=dylib=gdi32");
    println!("cargo:rustc-link-lib=dylib=user32");
    println!("cargo:rustc-link-lib=dylib=ole32");
    println!("cargo:rustc-link-lib=dylib=ntdll");
    println!("cargo:rustc-link-lib=dylib=advapi32");
    println!("cargo:rustc-link-lib=dylib=shell32");
}