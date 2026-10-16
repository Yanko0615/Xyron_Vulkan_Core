use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=kernel.asm");

    // Ruta exacta del ejecutable de FASM dentro de tu proyecto
    let fasm_path = r"G:\Aivi\fasm\fasm.exe"; 

    let status = Command::new(fasm_path)
        .args(["kernel.asm", "kernel.obj"])
        .status()
        .expect("No se pudo encontrar fasm.exe en G:\\Aivi\\fasm\\");

    assert!(status.success(), "La compilación de FASM falló.");

    // Indicar al enlazador de Rust dónde encontrar el archivo objeto
    println!("cargo:rustc-link-arg=kernel.obj");
}