use std::os::raw::{c_char, c_int, c_void, c_float};
use std::ffi::{CStr, CString};
use std::fs::{self, File};
use std::io::{Read, Write, Seek, SeekFrom};
use std::path::Path;
use std::thread;
use std::ptr;

static mut GLOBAL_HWND: *mut c_void = std::ptr::null_mut();
static mut EXPERT_MODE: bool = false;

#[link(name = "user32")]
extern "C" {
    pub fn PostMessageA(hWnd: *mut c_void, Msg: u32, wParam: usize, lParam: isize) -> i32;
}

fn get_system_hwid() -> String { String::from("XYRON-FALLBACK-HWID") }

#[no_mangle]
pub extern "C" fn xyron_set_hwnd(hwnd: *mut c_void) { unsafe { GLOBAL_HWND = hwnd; } }

#[no_mangle]
pub extern "C" fn xyron_set_expert_mode(enabled: u8) { unsafe { EXPERT_MODE = enabled != 0; } }

#[no_mangle]
pub extern "C" fn xyron_get_hwid(buffer: *mut c_char, max_size: u32) {
    if buffer.is_null() || max_size == 0 { return; }
    let hwid = get_system_hwid();
    let c_string = CString::new(hwid).unwrap_or_default();
    let bytes = c_string.as_bytes_with_nul();
    let len_to_copy = std::cmp::min(bytes.len(), max_size as usize - 1);
    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), buffer as *mut u8, len_to_copy);
        *buffer.add(len_to_copy) = 0;
    }
}

#[no_mangle]
pub extern "C" fn xyron_verificar_licencia() -> u8 { 1 }

#[no_mangle]
pub extern "C" fn xyron_guardar_licencia(key_buffer: *const c_char) -> u8 {
    if key_buffer.is_null() { return 0; }
    let c_str = unsafe { CStr::from_ptr(key_buffer) };
    let key_str = c_str.to_string_lossy().into_owned();
    let _ = fs::write("xyron_license.dat", key_str);
    1 
}

#[no_mangle]
pub extern "C" fn xyron_identificar_modelo(model_path: *const c_char) -> c_int {
    if model_path.is_null() { return 1; }
    let path_str = unsafe { CStr::from_ptr(model_path).to_string_lossy() };
    let path_lower = path_str.to_lowercase();
    if path_lower.ends_with(".ckpt") || path_lower.contains("mm_sd") || path_lower.contains("motion_module") { return 4; }
    let mut file = match File::open(path_str.as_ref()) { Ok(f) => f, Err(_) => return 1, };
    let mut len_bytes = [0u8; 8];
    if file.read_exact(&mut len_bytes).is_err() { return 1; }
    let header_len = u64::from_le_bytes(len_bytes) as usize;
    if header_len > 10_000_000 { return 1; } 
    let mut header_bytes = vec![0u8; header_len];
    if file.read_exact(&mut header_bytes).is_err() { return 1; }
    let header_json = String::from_utf8_lossy(&header_bytes);
    if header_json.contains("dit") || header_json.contains("wan") || header_json.contains("flow") || path_lower.contains("flux") || path_lower.contains("wan") { return 5; }
    if header_json.contains("image_proj") || header_json.contains("unstamped") || path_lower.contains("svd") { return 3; }
    if header_json.contains("conditioner.embedders.1") || header_json.contains("text_encoder_2") { return 2; } 
    if header_json.contains("cond_stage_model") { return 1; }
    1 
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct sd_image_t {
    pub width: u32,
    pub height: u32,
    pub channel: u32,
    pub data: *mut u8,
}

#[repr(C)]
pub struct sd_lora_t {
    pub is_high_noise: bool,
    pub multiplier: c_float,
    pub path: *const c_char,
}

#[repr(C)]
pub struct sd_ctx_params_t {
    pub model_path: *const c_char,
    pub clip_l_path: *const c_char,
    pub clip_g_path: *const c_char,
    pub clip_vision_path: *const c_char,
    pub t5xxl_path: *const c_char,
    pub llm_path: *const c_char,
    pub llm_vision_path: *const c_char,
    pub diffusion_model_path: *const c_char,
    pub high_noise_diffusion_model_path: *const c_char,
    pub uncond_diffusion_model_path: *const c_char,
    pub embeddings_connectors_path: *const c_char,
    pub vae_path: *const c_char,
    pub audio_vae_path: *const c_char,
    pub audio_encoder_path: *const c_char,
    pub taesd_path: *const c_char,
    pub control_net_path: *const c_char,
    pub ip_adapter_path: *const c_char,
    pub motion_module_path: *const c_char,
    pub embeddings: *const c_void,
    pub embedding_count: u32,
    pub photo_maker_path: *const c_char,
    pub pulid_weights_path: *const c_char,
    pub tensor_type_rules: *const c_char,
    pub n_threads: c_int,
    pub wtype: c_int,
    pub rng_type: c_int,
    pub sampler_rng_type: c_int,
    pub prediction: c_int,
    pub lora_apply_mode: c_int,
    pub enable_mmap: bool,
    pub flash_attn: bool,
    pub diffusion_flash_attn: bool,
    pub tae_preview_only: bool,
    pub diffusion_conv_direct: bool,
    pub vae_conv_direct: bool,
    pub force_sdxl_vae_conv_scale: bool,
    pub vae_format: c_int,
    pub max_vram: *const c_char,
    pub disable_prefetch: bool,
    pub eager_load: bool,
    pub backend: *const c_char,
    pub params_backend: *const c_char,
    pub split_mode: *const c_char,
    pub auto_fit: bool,
    pub rpc_servers: *const c_char,
    pub model_args: *const c_char,
    pub disable_segmented_compute: bool,
    pub linear_scale: c_float,
    pub attn_scale: c_float,
    pub tokenizer: *const c_char,
    pub sage_attn: bool,
    pub conditioning_cache_size: c_int,
    pub _reserved: [u8; 256],
}

#[repr(C)]
pub struct sd_slg_params_t { pub layers: *mut c_int, pub layer_count: usize, pub layer_start: f32, pub layer_end: f32, pub scale: f32 }
#[repr(C)]
pub struct sd_guidance_params_t { pub txt_cfg: f32, pub img_cfg: f32, pub distilled_guidance: f32, pub slg: sd_slg_params_t }
#[repr(C)]
pub struct sd_sample_params_t { pub guidance: sd_guidance_params_t, pub scheduler: c_int, pub sample_method: c_int, pub sample_steps: c_int, pub eta: f32, pub shifted_timestep: c_int, pub custom_sigmas: *mut f32, pub custom_sigmas_count: c_int, pub flow_shift: f32, pub extra_sample_args: *const c_char }
#[repr(C)]
pub struct sd_img_gen_params_t { pub loras: *const c_void, pub lora_count: u32, pub prompt: *const c_char, pub negative_prompt: *const c_char, pub clip_skip: c_int, pub init_image: sd_image_t, pub ref_images: *const sd_image_t, pub ref_images_count: c_int, pub ref_image_args: *const c_char, pub mask_image: sd_image_t, pub width: c_int, pub height: c_int, pub sample_params: sd_sample_params_t, pub strength: f32, pub seed: i64, pub batch_count: c_int, pub _reserved: [u8; 1024] }

#[link(name = "stable-diffusion", kind = "static")]
extern "C" {
    pub fn sd_ctx_params_init(params: *mut sd_ctx_params_t);
    pub fn new_sd_ctx(params: *const sd_ctx_params_t) -> *mut c_void;
    pub fn free_sd_ctx(sd_ctx: *mut c_void);
    pub fn sd_img_gen_params_init(params: *mut sd_img_gen_params_t);
    pub fn sd_sample_params_init(params: *mut sd_sample_params_t);
    pub fn generate_image(sd_ctx: *mut c_void, params: *const sd_img_gen_params_t, images_out: *mut *mut sd_image_t, num_images_out: *mut c_int) -> bool;
    pub fn free_sd_images(result_images: *mut sd_image_t, num_images: c_int);
    pub fn sd_set_log_callback(cb: extern "C" fn(c_int, *const c_char, *mut c_void), data: *mut c_void);
    pub fn sd_set_progress_callback(cb: extern "C" fn(c_int, c_int, f32, *mut c_void), data: *mut c_void);
    pub fn sd_set_regional_mask(mask: *mut c_void);
}

extern "C" fn log_callback(_level: c_int, text: *const c_char, _data: *mut c_void) {
    if !text.is_null() { if let Ok(s) = unsafe { std::ffi::CStr::from_ptr(text) }.to_str() { print!("{}", s); } }
}
extern "C" fn progress_callback(step: c_int, steps: c_int, _time: f32, _data: *mut c_void) {
    if steps > 1 && step < steps {
        let percent = ((step as f32 / steps as f32) * 100.0) as i32;
        unsafe { if !GLOBAL_HWND.is_null() { PostMessageA(GLOBAL_HWND, 0x0401, percent.clamp(0, 99) as usize, 0); } }
    }
}

fn guardar_bmp(path: &str, data: &[u8], width: u32, height: u32, channels: u32) -> std::io::Result<()> {
    let mut file = File::create(path)?;
    let row_stride = ((width * 3 + 3) / 4) * 4; 
    let image_data_size = row_stride * height;
    let file_size = 54 + image_data_size;

    file.write_all(b"BM")?; file.write_all(&(file_size as u32).to_le_bytes())?;
    file.write_all(&[0, 0, 0, 0])?; file.write_all(&54u32.to_le_bytes())?;
    file.write_all(&40u32.to_le_bytes())?; file.write_all(&(width as i32).to_le_bytes())?;
    file.write_all(&(-(height as i32)).to_le_bytes())?; file.write_all(&1u16.to_le_bytes())?;
    file.write_all(&24u16.to_le_bytes())?; file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&image_data_size.to_le_bytes())?; file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?; file.write_all(&0u32.to_le_bytes())?;
    file.write_all(&0u32.to_le_bytes())?;

    let row_size_src = (width * channels) as usize;
    let padding_len = (row_stride - width * 3) as usize;
    let padding = vec![0u8; padding_len];

    for y in 0..height {
        let src_row_start = (y as usize) * row_size_src;
        for x in 0..width {
            let src_idx = src_row_start + (x as usize) * (channels as usize);
            let (r, g, b) = if channels >= 3 { (data[src_idx], data[src_idx + 1], data[src_idx + 2]) } else { let v = data[src_idx]; (v, v, v) };
            file.write_all(&[b, g, r])?;
        }
        if padding_len > 0 { file.write_all(&padding)?; }
    }
    Ok(())
}

// 🖼️ CARGADOR DE IMAGEN BASE DE ENTRADA (BMP)
fn cargar_bmp_entrada(path: &str) -> Option<(u32, u32, u32, Vec<u8>)> {
    let mut file = File::open(path).ok()?;
    let mut header = [0u8; 54];
    file.read_exact(&mut header).ok()?;
    if &header[0..2] != b"BM" { return None; }
    
    let width = i32::from_le_bytes(header[18..22].try_into().ok()?).abs() as u32;
    let height = i32::from_le_bytes(header[22..26].try_into().ok()?).abs() as u32;
    let bpp = u16::from_le_bytes(header[28..30].try_into().ok()?);
    let channels = (bpp / 8) as u32;
    let data_offset = u32::from_le_bytes(header[10..14].try_into().ok()?) as usize;
    
    file.seek(SeekFrom::Start(data_offset as u64)).ok()?;
    let row_stride = ((width * channels + 3) / 4) * 4;
    let mut raw_data = vec![0u8; (row_stride * height) as usize];
    file.read_exact(&mut raw_data).ok()?;

    let mut rgb = vec![0u8; (width * height * 3) as usize];
    for y in 0..height {
        let src_row = (y * row_stride) as usize;
        let dst_row = (y * width * 3) as usize;
        for x in 0..width {
            let src_idx = src_row + (x * channels) as usize;
            let dst_idx = dst_row + (x * 3) as usize;
            if channels >= 3 {
                rgb[dst_idx] = raw_data[src_idx + 2];     // R
                rgb[dst_idx + 1] = raw_data[src_idx + 1]; // G
                rgb[dst_idx + 2] = raw_data[src_idx];     // B
            }
        }
    }
    Some((width, height, 3, rgb))
}

#[no_mangle]
pub extern "C" fn ejecutar_motor_inferencia(
    pri_ptr: *const c_char, 
    sec_ptr: *const c_char, 
    narration_ptr: *const c_char,
    width: c_int,
    height: c_int,
    steps: c_int,
    model_ptr: *const c_char,
    cfg_scaled: c_int,
    lora_ptr: *const c_char, 
    lora_weight_scaled: c_int,
    init_img_ptr: *const c_char
) -> bool {
    let pri_string = unsafe { if pri_ptr.is_null() { "".to_string() } else { std::ffi::CStr::from_ptr(pri_ptr).to_string_lossy().into_owned() } };
    let sec_string = unsafe { if sec_ptr.is_null() { "".to_string() } else { std::ffi::CStr::from_ptr(sec_ptr).to_string_lossy().into_owned() } };
    let narra_string = unsafe { if narration_ptr.is_null() { "".to_string() } else { std::ffi::CStr::from_ptr(narration_ptr).to_string_lossy().into_owned() } };
    let lora_string_ui = unsafe { if lora_ptr.is_null() { "".to_string() } else { std::ffi::CStr::from_ptr(lora_ptr).to_string_lossy().into_owned() } };
    let model_string = unsafe { if model_ptr.is_null() { "".to_string() } else { std::ffi::CStr::from_ptr(model_ptr).to_string_lossy().into_owned() } };
    let init_img_string = unsafe { if init_img_ptr.is_null() { "".to_string() } else { std::ffi::CStr::from_ptr(init_img_ptr).to_string_lossy().into_owned() } };

    let is_expert = unsafe { EXPERT_MODE };
    let lora_weight_f32 = (lora_weight_scaled as f32) / 10.0;
    let cfg_f32 = (cfg_scaled as f32) / 10.0;

    thread::spawn(move || {
        unsafe { sd_set_log_callback(log_callback, std::ptr::null_mut()); sd_set_progress_callback(progress_callback, std::ptr::null_mut()); }

        // 🧠 SELECCIÓN DINÁMICA DE MODELO (Flux vs Wan 2.1)
        let dit_path_str = if !model_string.is_empty() && Path::new(&model_string).exists() {
            model_string.clone()
        } else if model_string.contains("wan") {
            "models/wan2.1-v3-vace-1.3b-q4_0.gguf".to_string()
        } else {
            "models/fastflux-q4.gguf".to_string()
        };

        let dit_path = CString::new(dit_path_str.clone()).unwrap();
        let t5_path = CString::new("models/t5xxl-q4.gguf").unwrap();
        let vae_path = CString::new("models/ae.safetensors").unwrap();

        let arquitectura = xyron_identificar_modelo(dit_path.as_ptr());
        let es_flux = arquitectura == 5 && !dit_path_str.contains("wan"); 
        let es_wan = dit_path_str.contains("wan");

        let mut final_prompt = pri_string.clone();
        let mut final_neg = format!("{}, deformed hands, mutated fingers, extra digits, distorted eyes, bad anatomy", sec_string);

        let n_lower = narra_string.to_lowercase();
        let p_lower = pri_string.to_lowercase();
        
        let tiene_zenin = n_lower.contains("zenin") || p_lower.contains("zenin") || n_lower.contains("zeni") || p_lower.contains("zeni");
        let tiene_enzo = n_lower.contains("enzo") || p_lower.contains("enzo"); 
        let tiene_helius = n_lower.contains("helius") || p_lower.contains("helius"); 

        unsafe { sd_set_regional_mask(std::ptr::null_mut()); }

        if !is_expert && (!narra_string.is_empty() || !pri_string.is_empty()) && !es_wan {
            let tag_zenin = "Zenin, a young woman with very long wavy blonde hair and blue eyes, standing in a full-body shot. She is looking forward with a serious expression. She is wearing a highly detailed fantasy outfit consisting of a black top with a plunging cleavage cutout, a flowing white overcoat with intricate gold floral embroidery, and a black skirt with high slits. She wears black thigh-high leather boots detailed with complex gold straps and crosses, black fingerless gloves, and a thigh strap with a dangling blue tassel. She is accessorized with blue gem earrings and a dark blue ribbon tied in her hair. Plain light grey background.";
            let tag_enzo = "Enzo, anime-style young adult male, messy layered crimson red hair, intense red eyes partially visible beneath his bangs, muscular heavily toned athletic build, broad shoulders, wearing a black tactical long-sleeved shirt with sleeves rolled up to the elbows, black tactical cargo pants with multiple utility pockets and straps, black tactical gloves, heavy black combat boots, black face mask covering his nose and mouth, black tactical shoulder harness with adjustable straps and metal buckles, silver chain necklace with a silver cross pendant, long dangling cross earrings, standing upright in a neutral symmetrical pose, arms relaxed at his sides, legs slightly apart, facing forward, full body front view, entire figure visible from head to boots, plain white studio background, clean anime line art, detailed black tactical fabric folds, polished cel shading, high-quality anime character reference illustration";
            let tag_helius = "Helius, anime-style young adult male, messy spiky black hair with layered strands, light blue eyes, handsome youthful face, defined jawline, muscular heavily toned athletic build, broad shoulders, wearing a fitted black long-sleeved shirt, slim black jeans, black belt with a silver buckle, black shoulder harness with metal buckles crossing diagonally over his chest, katana in a black sheath strapped diagonally across his back and extending behind his hip, black and white athletic sneakers, standing upright in a relaxed neutral pose, arms resting naturally at his sides, legs slightly apart, facing forward, calm confident expression, full body shot, straight-on camera angle, entire body visible from head to shoes, centered composition, plain gray studio background, soft even lighting, subtle floor shadow, detailed black fabric textures, crisp anime line art, refined cel shading, high-quality anime character reference illustration";

            let accion = if !pri_string.is_empty() { &pri_string } else { &narra_string };

            if es_flux {
                if tiene_zenin && tiene_enzo {
                    final_prompt = format!("A highly detailed wide shot of Zenin and Enzo. On the left: {}. On the right: {}. The action between them: {}.", tag_zenin, tag_enzo, accion);
                } else if tiene_zenin && tiene_helius {
                    final_prompt = format!("A highly detailed wide shot of Zenin and Helius interacting. On the left: {}. On the right: {}. The action between them: {}.", tag_zenin, tag_helius, accion);
                } else if tiene_helius && tiene_enzo {
                    final_prompt = format!("A highly detailed wide shot of Helius and Enzo. On the left: {}. On the right: {}. The action between them: {}.", tag_helius, tag_enzo, accion);
                } else if tiene_zenin {
                    final_prompt = format!("A cinematic dynamic portrait of {}. Action: {}.", tag_zenin, accion);
                } else if tiene_helius {
                    final_prompt = format!("A cinematic dynamic portrait of {}. Action: {}.", tag_helius, accion);
                } else if tiene_enzo {
                    final_prompt = format!("A cinematic dynamic portrait of {}. Action: {}.", tag_enzo, accion);
                } else {
                    final_prompt = format!("A cinematic, high quality dynamic shot of {}.", accion);
                }
                final_neg = "".to_string(); 
            }
        }

        let mut loras_cstrings: Vec<CString> = Vec::new();
        let mut array_loras: Vec<sd_lora_t> = Vec::new();

        if !es_wan {
            if tiene_zenin && tiene_enzo {
                if let Ok(c) = CString::new("models/loras/enzo_lora.safetensors") { 
                    if Path::new("models/loras/enzo_lora.safetensors").exists() { loras_cstrings.push(c); } 
                }
            } else if tiene_zenin && tiene_helius {
                if let Ok(c) = CString::new("models/loras/helius_lora.safetensors") { 
                    if Path::new("models/loras/helius_lora.safetensors").exists() { loras_cstrings.push(c); } 
                }
            } else if tiene_helius && tiene_enzo {
                if let Ok(c) = CString::new("models/loras/enzo_lora.safetensors") { 
                    if Path::new("models/loras/enzo_lora.safetensors").exists() { loras_cstrings.push(c); } 
                }
            } else {
                let prompt_lower = final_prompt.to_lowercase();
                if prompt_lower.contains("zenin") || prompt_lower.contains("zeni") { 
                    if let Ok(c) = CString::new("models/loras/zenin_lora.safetensors") { 
                        if Path::new("models/loras/zenin_lora.safetensors").exists() { loras_cstrings.push(c); } 
                    } 
                }
                if prompt_lower.contains("enzo") { 
                    if let Ok(c) = CString::new("models/loras/enzo_lora.safetensors") { 
                        if Path::new("models/loras/enzo_lora.safetensors").exists() { loras_cstrings.push(c); } 
                    } 
                }
                if prompt_lower.contains("helius") { 
                    if let Ok(c) = CString::new("models/loras/helius_lora.safetensors") { 
                        if Path::new("models/loras/helius_lora.safetensors").exists() { loras_cstrings.push(c); } 
                    } 
                }
            }

            let lora_dinamico = 0.85; 
            for cstr in &loras_cstrings { array_loras.push(sd_lora_t { is_high_noise: false, multiplier: lora_dinamico, path: cstr.as_ptr() }); }

            if !lora_string_ui.is_empty() && Path::new(lora_string_ui.trim()).exists() {
                if let Ok(c_ui) = CString::new(lora_string_ui.trim()) {
                    let leaked_cstr = Box::leak(Box::new(c_ui)); 
                    array_loras.push(sd_lora_t { is_high_noise: false, multiplier: lora_weight_f32, path: leaked_cstr.as_ptr() });
                }
            }
        }

        let prompt_c = CString::new(final_prompt).unwrap_or_else(|_| CString::new("masterpiece").unwrap());
        let negative_c = CString::new(final_neg).unwrap_or_else(|_| CString::new("").unwrap());
        
        // 🚀 PROCESAMIENTO DIRECTO EN GPU VULKAN (16 GB VRAM)
        let hibrido_backend = CString::new("Vulkan0").unwrap();

        unsafe {
            let mut ctx_params: sd_ctx_params_t = std::mem::zeroed();
            sd_ctx_params_init(&mut ctx_params);
            
            ctx_params.diffusion_model_path = dit_path.as_ptr(); 
            ctx_params.t5xxl_path = t5_path.as_ptr();            
            ctx_params.vae_path = vae_path.as_ptr();             
            
            ctx_params.n_threads = 4;
            ctx_params.backend = hibrido_backend.as_ptr(); 

            let sd_ctx = new_sd_ctx(&ctx_params);
            
            if sd_ctx.is_null() { 
                println!("[ERROR CRÍTICO] Falló la carga del modelo.");
                if !GLOBAL_HWND.is_null() { PostMessageA(GLOBAL_HWND, 0x0402, 0, 0); }
                return; 
            }

            let mut gen_params: sd_img_gen_params_t = std::mem::zeroed();
            sd_img_gen_params_init(&mut gen_params);
            sd_sample_params_init(&mut gen_params.sample_params);

            // 🎬 CARGA DE FOTOGRAMA BASE PARA WAN 2.1 I2V
            let mut init_pixel_data = Vec::new();
            let mut init_sd_img = sd_image_t { width: 0, height: 0, channel: 0, data: std::ptr::null_mut() };

            if !init_img_string.is_empty() && Path::new(&init_img_string).exists() {
                if let Some((w, h, c, pixels)) = cargar_bmp_entrada(&init_img_string) {
                    println!("[VULKAN] -> Cargada imagen inicial para animación: {} ({}x{})", init_img_string, w, h);
                    init_pixel_data = pixels;
                    init_sd_img = sd_image_t {
                        width: w,
                        height: h,
                        channel: c,
                        data: init_pixel_data.as_mut_ptr(),
                    };
                    gen_params.init_image = init_sd_img;
                }
            }

            if es_wan {
                gen_params.sample_params.sample_steps = 16; 
                gen_params.sample_params.guidance.txt_cfg = 6.0; 
                println!("[VULKAN] Modo Wan 2.1 (Video I2V) Activado: 16 Pasos | CFG 6.0");
            } else if es_flux {
                gen_params.sample_params.sample_steps = 15; 
                gen_params.sample_params.guidance.txt_cfg = 1.0; 
                println!("[VULKAN] Modo Flux (Imagen) Activado: 15 Pasos | CFG 1.0");
            } else {
                gen_params.sample_params.sample_steps = steps; 
                gen_params.sample_params.guidance.txt_cfg = cfg_f32;
            }

            gen_params.prompt = prompt_c.as_ptr();
            gen_params.negative_prompt = negative_c.as_ptr();
            gen_params.width = width;
            gen_params.height = height;
            gen_params.clip_skip = -1;

            if !array_loras.is_empty() {
                gen_params.loras = array_loras.as_ptr() as *const c_void;
                gen_params.lora_count = array_loras.len() as u32;
            }

            let mut images_out: *mut sd_image_t = std::ptr::null_mut();
            let mut num_images: c_int = 0;

            let success = generate_image(sd_ctx, &gen_params, &mut images_out, &mut num_images);
            
            if success && !images_out.is_null() && num_images > 0 {
                let img = *images_out;
                let buffer_size = (img.width * img.height * img.channel) as usize;
                let slice = std::slice::from_raw_parts(img.data, buffer_size);
                
                if es_wan {
                    let _ = std::fs::write("output.mp4", slice);
                } else {
                    let _ = guardar_bmp("output.bmp", slice, img.width, img.height, img.channel);
                }
            } 
            
            if !images_out.is_null() { free_sd_images(images_out, num_images); }
            free_sd_ctx(sd_ctx);

            if !GLOBAL_HWND.is_null() {
                PostMessageA(GLOBAL_HWND, 0x0401, 100 as usize, 0); 
                PostMessageA(GLOBAL_HWND, 0x0402, 0, 0);            
            }
        }
    });
    true
}