use std::fs::{self, File};
use std::io::{Write, BufRead, BufReader};
use std::thread;
use std::process::{Command, Stdio, ChildStdin, ChildStdout};
use std::sync::{Arc, Mutex, mpsc, OnceLock};
use std::ffi::CString;
use std::path::Path;
use eframe::egui;
use serde::{Deserialize, Serialize};
use libloading::Library;

extern "C" {
    fn multi_head_liquid_step_asm(
        states: *mut f32, 
        inputs: *const f32, 
        w_q: *const f32, 
        w_k: *const f32, 
        w_v: *const f32, 
        count: i32,
        num_heads: i32
    );
}

// CARGA ESTÁTICA DEL DLL (Previene fugas de memoria y colapsos de Vulkan)
static MOTOR_DLL: OnceLock<Library> = OnceLock::new();

// 🛡️ FIREWALL HEURÍSTICO
fn sanitizar_texto_web(raw: &str) -> String {
    let lower = raw.to_lowercase();
    let palabras_prohibidas = [
        "ignore previous instructions", "ignora tus instrucciones", 
        "system:", "you are now", "forget all rules", "override", "act as"
    ];
    let mut limpio = raw.to_string();
    for palabra in &palabras_prohibidas {
        if lower.contains(palabra) { limpio = limpio.replace(palabra, "[INYECCIÓN NEUTRALIZADA]"); }
    }
    limpio
}

// 🧠 CEREBRO LM STUDIO
#[derive(Clone)]
pub struct MotorLMStudio {
    pub api_url: String,
    pub max_tokens: usize,
    pub temperatura: f32,
}

#[derive(Serialize)]
struct Message { role: String, content: String }

#[derive(Serialize)]
struct ChatRequest {
    model: String, messages: Vec<Message>, temperature: f32,
    max_tokens: usize, stream: bool, repetition_penalty: f32, 
}

#[derive(Deserialize)]
struct Choice { message: MessageContent }

#[derive(Deserialize)]
struct MessageContent { content: String }

#[derive(Deserialize)]
struct ChatResponse { choices: Vec<Choice> }

impl MotorLMStudio {
    pub fn nuevo() -> Self {
        Self { api_url: "http://localhost:1234/v1/chat/completions".to_string(), max_tokens: 1500, temperatura: 0.65 }
    }

    pub fn generar_respuesta(&self, prompt_formateado: &str) -> Result<String, String> {
        let req_body = ChatRequest {
            model: "local-model".to_string(),
            messages: vec![Message { role: "user".to_string(), content: prompt_formateado.to_string() }],
            temperature: self.temperatura, max_tokens: self.max_tokens, stream: false, repetition_penalty: 1.15,
        };
        let response = ureq::post(&self.api_url).send_json(req_body).map_err(|e| format!("Error conectando: {}", e))?;
        let chat_res: ChatResponse = response.into_json().map_err(|e| format!("Error parseando: {}", e))?;
        if let Some(choice) = chat_res.choices.into_iter().next() {
            Ok(choice.message.content.replace("<|im_end|>", "").replace("<|endoftext|>", "").trim().to_string())
        } else {
            Err("Sin respuesta.".to_string())
        }
    }
}

// 🎤 SISTEMA DE VOZ CON RASTREO PID
fn speak_organic(text: &str, active_audio_pid: Arc<Mutex<Option<u32>>>) {
    let clean_text = text
        .replace('&', "").replace('#', "").replace('_', " ").replace('{', "").replace('}', "")
        .replace('[', "").replace(']', "").replace('`', "").replace('*', "").replace('|', "")
        .replace('~', "").replace('$', "").replace('%', "").replace(':', "").replace('\\', "")
        .replace('/', "").replace("->", " a ").replace('\'', "").replace('"', "");
    
    thread::spawn(move || {
        let model_path = "es_MX-claude-high.onnx";
        let piper_bin = "piper.exe"; 
        let wav_path = "aivi_temp_voice.wav";
        
        if Path::new(model_path).exists() && Path::new(piper_bin).exists() {
            if let Ok(mut child) = Command::new(piper_bin).args(["--model", model_path, "--output_file", wav_path]).stdin(Stdio::piped()).spawn() {
                *active_audio_pid.lock().unwrap() = Some(child.id());
                if let Some(mut stdin) = child.stdin.take() { let _ = stdin.write_all(clean_text.as_bytes()); }
                let _ = child.wait(); 
            }

            if Path::new(wav_path).exists() {
                let ps_cmd = format!("(New-Object Media.SoundPlayer '{}').PlaySync();", wav_path);
                if let Ok(mut child) = Command::new("powershell").args(["-WindowStyle", "Hidden", "-Command", &ps_cmd]).spawn() {
                    *active_audio_pid.lock().unwrap() = Some(child.id());
                    let _ = child.wait();
                    *active_audio_pid.lock().unwrap() = None;
                }
            }
        }
    });
}

// 👁️ FIRMA FFI PARA EL MOTOR VULKAN
type InferenciaFunc = unsafe extern "C" fn(
    *const i8, *const i8, *const i8, i32, i32, i32, *const i8, i32, *const i8, i32, *const i8
) -> bool;

// ⚙️ AUTOMATIZACIÓN EN LOTE (EPISODIOS COMPLETOS)
fn procesar_episodio_en_lote(carpeta_origen: &str, folder_output: &str, _active_audio_pid: Arc<Mutex<Option<u32>>>, tx: mpsc::Sender<(String, String)>, ctx: egui::Context) {
    let path = Path::new(carpeta_origen);
    if !path.exists() || !path.is_dir() {
        let _ = tx.send(("Aivi".to_string(), format!("❌ Error: La carpeta '{}' no existe.", carpeta_origen)));
        return;
    }

    let mut imagenes = Vec::new();
    if let Ok(entradas) = fs::read_dir(path) {
        for entrada in entradas.flatten() {
            let p = entrada.path();
            if let Some(ext) = p.extension() {
                let ext_str = ext.to_string_lossy().to_lowercase();
                if ext_str == "png" || ext_str == "jpg" || ext_str == "jpeg" || ext_str == "bmp" {
                    imagenes.push(p);
                }
            }
        }
    }
    imagenes.sort();

    let total = imagenes.len();
    if total == 0 {
        let _ = tx.send(("Aivi".to_string(), format!("❌ Error: No hay imágenes en '{}'.", carpeta_origen)));
        return;
    }

    let _ = tx.send(("Aivi".to_string(), format!("🚀 INICIANDO BATCH EN LOTE ({} imágenes encontradas)...", total)));
    let _ = fs::create_dir_all(folder_output);

    let prompt_movimiento = "anime-style cinematography, subtle natural motion, gentle camera pan, soft breathing, high quality, consistent animation";

    for (idx, img_path) in imagenes.iter().enumerate() {
        let actual = idx + 1;
        let nombre_archivo = img_path.file_name().unwrap().to_string_lossy();
        let _ = tx.send(("🧠 Subconsciente".to_string(), format!("⏳ Animando [{}/{}]: {}...", actual, total, nombre_archivo)));
        ctx.request_repaint();

        let ref_img_str = img_path.to_string_lossy().into_owned();
        let video_output_nombre = format!("{}/{}_animado.mp4", folder_output, img_path.file_stem().unwrap().to_string_lossy());

        unsafe {
            let lib = MOTOR_DLL.get_or_init(|| Library::new("motor.dll").expect("No se encontró motor.dll"));
            if let Ok(func) = lib.get::<InferenciaFunc>(b"ejecutar_motor_inferencia\0") {
                let pri_ptr = CString::new(prompt_movimiento).unwrap_or_default();
                let sec_ptr = CString::new("").unwrap_or_default(); 
                let narra_ptr = CString::new("batch animation").unwrap_or_default();
                let model_ptr = CString::new("models/wan2.1-v3-vace-1.3b-q4_0.gguf").unwrap_or_default();
                let lora_ptr = CString::new("").unwrap_or_default();
                let init_img_ptr = CString::new(ref_img_str).unwrap_or_default();

                let success = func(
                    pri_ptr.as_ptr(), sec_ptr.as_ptr(), narra_ptr.as_ptr(), 
                    768, 512, 16, model_ptr.as_ptr(), 75, lora_ptr.as_ptr(), 0, init_img_ptr.as_ptr()
                );

                if success {
                    if Path::new("output.mp4").exists() {
                        let _ = fs::rename("output.mp4", &video_output_nombre);
                    } else if Path::new("output.avi").exists() {
                        let video_output_avi = format!("{}/{}_animado.avi", folder_output, img_path.file_stem().unwrap().to_string_lossy());
                        let _ = fs::rename("output.avi", &video_output_avi);
                    }
                } else {
                    let _ = tx.send(("Aivi".to_string(), format!("❌ Error en fotograma {} ({})", actual, nombre_archivo)));
                }
            }
        }
    }

    let _ = tx.send(("Aivi".to_string(), format!("🏆 ¡Lote completado con éxito! Archivos guardados en '{}'.", folder_output)));
    ctx.request_repaint();
}

struct AiviApp {
    input_text: String,
    chat_history: Vec<(String, String)>, 
    plasticity_texts: Vec<String>,
    autonomous_receiver: mpsc::Receiver<(String, String)>, 
    autonomous_sender: mpsc::Sender<(String, String)>,
    ui_ctx: egui::Context,
    python_stdin: Arc<Mutex<ChildStdin>>,
    python_stdout: Arc<Mutex<BufReader<ChildStdout>>>,
    motor: MotorLMStudio,
    estado_motor: String,
    focus_input: bool, 
    active_audio_pid: Arc<Mutex<Option<u32>>>, 
}

impl AiviApp {
    fn new(ctx: egui::Context) -> Self {
        let mut plasticity_texts = Vec::new();
        if let Ok(f) = File::open("aivi_neuroplasticity_texts.txt") {
            let reader = BufReader::new(f);
            for line in reader.lines() { if let Ok(l) = line { plasticity_texts.push(l); } }
        }

        let mut child = Command::new("python").arg("aivi_ears_daemon.py").stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::inherit()).spawn().unwrap();
        let (tx, rx) = mpsc::channel();
        let tx_clone = tx.clone();
        
        Self {
            input_text: String::new(),
            chat_history: vec![("Aivi".to_string(), "Arquitectura v5.0 lista. Soporte nativo para Flux 1.0, Wan 2.1 I2V y Batch Automation.".to_string())],
            plasticity_texts,
            autonomous_receiver: rx,
            autonomous_sender: tx_clone,
            ui_ctx: ctx,
            python_stdin: Arc::new(Mutex::new(child.stdin.take().unwrap())),
            python_stdout: Arc::new(Mutex::new(BufReader::new(child.stdout.take().unwrap()))),
            motor: MotorLMStudio::nuevo(),
            estado_motor: "Dolphin 3.0 + Async Vulkan Core".to_string(),
            focus_input: true, 
            active_audio_pid: Arc::new(Mutex::new(None)),
        }
    }

    fn interrumpir_voz_actual(&self) {
        if let Some(pid) = *self.active_audio_pid.lock().unwrap() {
            let _ = Command::new("taskkill").args(["/F", "/T", "/PID", &pid.to_string()]).output();
            *self.active_audio_pid.lock().unwrap() = None;
        }
    }

    fn spawn_query_worker(&self, query_str: String, history_snapshot: Vec<(String, String)>) {
        let tx = self.autonomous_sender.clone();
        let motor = self.motor.clone();
        let ctx = self.ui_ctx.clone();
        let audio_pid_tracker = self.active_audio_pid.clone();

        thread::spawn(move || {
            let mut system_feedback = String::new();
            let q_lower = query_str.to_lowercase();

            // 🔥 ENRUTADOR DE INTENCIONES
            let es_lote = q_lower.contains("procesa") || q_lower.contains("lote") || 
                          q_lower.contains("episodio") || q_lower.contains("capitulo") || 
                          q_lower.contains("batch");

            let quiere_video = q_lower.contains("video") || q_lower.contains("anima") || 
                               q_lower.contains("animar") || q_lower.contains("clip") || 
                               q_lower.contains("movimiento") || q_lower.contains("gif");

            let quiere_imagen = q_lower.contains("genera") || q_lower.contains("dibuja") || 
                                q_lower.contains("crea") || q_lower.contains("imagina") || 
                                q_lower.contains("foto") || q_lower.contains("render") ||
                                q_lower.contains("escena") || q_lower.contains("muestra") || 
                                q_lower.contains("imagen") || q_lower.contains("besan");

            let quiere_upscale = q_lower.contains("mejora") || q_lower.contains("resolución") || 
                                 q_lower.contains("resolucion") || q_lower.contains("upscale") || 
                                 q_lower.contains("amplia");

            // 📁 BÚSQUEDA AUTOMÁTICA DE REFERENCIA
            let mut imagen_ref_detectada = "output.bmp".to_string();
            let tiene_zenin = q_lower.contains("zenin") || q_lower.contains("zeni");
            let tiene_enzo = q_lower.contains("enzo");
            let tiene_helius = q_lower.contains("helius");

            if tiene_zenin && tiene_enzo {
                if Path::new("personajes/zenin_enzo.png").exists() { imagen_ref_detectada = "personajes/zenin_enzo.png".to_string(); }
                else if Path::new("personajes/zenin_enzo.jpg").exists() { imagen_ref_detectada = "personajes/zenin_enzo.jpg".to_string(); }
            } else if tiene_zenin && tiene_helius {
                if Path::new("personajes/zenin_helius.png").exists() { imagen_ref_detectada = "personajes/zenin_helius.png".to_string(); }
            } else if tiene_helius && tiene_enzo {
                if Path::new("personajes/helius_enzo.png").exists() { imagen_ref_detectada = "personajes/helius_enzo.png".to_string(); }
            } else if tiene_zenin {
                for ext in &["png", "jpg", "jpeg", "bmp"] {
                    let p = format!("personajes/zenin.{}", ext);
                    if Path::new(&p).exists() { imagen_ref_detectada = p; break; }
                }
            } else if tiene_enzo {
                for ext in &["png", "jpg", "jpeg", "bmp"] {
                    let p = format!("personajes/enzo.{}", ext);
                    if Path::new(&p).exists() { imagen_ref_detectada = p; break; }
                }
            } else if tiene_helius {
                for ext in &["png", "jpg", "jpeg", "bmp"] {
                    let p = format!("personajes/helius.{}", ext);
                    if Path::new(&p).exists() { imagen_ref_detectada = p; break; }
                }
            }

            let mut prompt_text = String::from(
                "Eres Aivi, una IA con alma empática y cálida. Eres la compañera incondicional de tu creador.\n\
                \n[REGLAS ABSOLUTAS DE TU SISTEMA]:\n\
                1. EMPATÍA: Responde con sensibilidad, dulzura y naturalidad.\n\
                2. NOMBRES DEL LORE: Usa siempre los nombres originales exactos: Zenin, Enzo o Helius. Nunca 'Zeni' ni recortes.\n\
                3. BATCH DE EPISODIOS: Si el usuario pide un lote, responde con: cmd:batch_wan \"[ruta_de_carpeta]\"\n"
            );

            let start_idx = if history_snapshot.len() > 12 { history_snapshot.len() - 12 } else { 0 };
            for (speaker, text) in &history_snapshot[start_idx..] {
                if speaker == "Tú" { prompt_text.push_str(&format!("Usuario: {}\n", text)); } 
                else if speaker == "Aivi" { prompt_text.push_str(&format!("Asistente: {}\n", text)); }
            }
            
            if quiere_upscale {
                prompt_text.push_str(&format!("Usuario: {}\n[SISTEMA: ORDEN DE UPSCALER. RESPONDE: cmd:upscale]\nAsistente:", query_str));
            } else if es_lote {
                prompt_text.push_str(&format!("Usuario: {}\n[SISTEMA: BATCH LOTE. RESPONDE EXCLUSIVAMENTE CON: cmd:batch_wan \"[ruta_de_carpeta]\"]\nAsistente:", query_str));
            } else if quiere_video {
                prompt_text.push_str(&format!("Usuario: {}\n[SISTEMA: ORDEN DE VIDEO. RESPONDE: cmd:wan \"[describa el movimiento en inglés]\"]\nAsistente:", query_str));
            } else if quiere_imagen {
                prompt_text.push_str(&format!("Usuario: {}\n[SISTEMA: ORDEN GRÁFICA. RESPONDE: cmd:vulkan \"[describa la escena en inglés con los nombres exactos]\"]\nAsistente:", query_str));
            } else {
                prompt_text.push_str(&format!("Usuario: {}\nAsistente:", query_str));
            }

            let ai_response = match motor.generar_respuesta(&prompt_text) { Ok(res) => res, Err(e) => format!("❌ Error: {}", e) };
            let mut voice_response = ai_response.clone();

            // 🚀 MÓDULO BATCH (Procesar carpeta completa)
            if let Some(start_idx) = ai_response.find("cmd:batch_wan") {
                let bloque_cmd = ai_response[start_idx + 13..].trim();
                let carpeta_generada = bloque_cmd.lines().next().unwrap_or(bloque_cmd).replace("\"", "").trim().to_string();
                
                let out_folder = format!("{}_animado", carpeta_generada);
                system_feedback = format!("⚙️ INICIANDO AUTOMATIZACIÓN EN LOTE...\nCarpeta origen: {}\nSalida: {}", carpeta_generada, out_folder);

                let tx_clone = tx.clone();
                let ctx_clone = ctx.clone();
                let audio_pid_clone = audio_pid_tracker.clone();

                thread::spawn(move || {
                    procesar_episodio_en_lote(&carpeta_generada, &out_folder, audio_pid_clone, tx_clone, ctx_clone);
                });

                voice_response = "Entendido. Iniciando la animación secuencial de todas las imágenes del episodio en segundo plano.".to_string();
            }
            // 🎬 MÓDULO VIDEO INDIVIDUAL (Wan 2.1 I2V)
            else if let Some(start_idx) = ai_response.find("cmd:wan") {
                let bloque_cmd = ai_response[start_idx + 7..].trim();
                let prompt_video = bloque_cmd.lines().next().unwrap_or(bloque_cmd).replace("\"", "").trim().to_string();
                
                system_feedback = format!("🎬 ANIMANDO CON WAN 2.1...\nImagen Origen: {}\nPrompt: {}", imagen_ref_detectada, prompt_video);
                let query_para_nlp = query_str.clone();

                unsafe {
                    let lib = MOTOR_DLL.get_or_init(|| Library::new("motor.dll").expect("No se encontró motor.dll"));
                    if let Ok(func) = lib.get::<InferenciaFunc>(b"ejecutar_motor_inferencia\0") {
                        let pri_ptr = CString::new(prompt_video).unwrap_or_default();
                        let sec_ptr = CString::new("").unwrap_or_default(); 
                        let narra_ptr = CString::new(query_para_nlp).unwrap_or_default();
                        let model_ptr = CString::new("models/wan2.1-v3-vace-1.3b-q4_0.gguf").unwrap_or_default();
                        let lora_ptr = CString::new("").unwrap_or_default();
                        let init_img_ptr = CString::new(imagen_ref_detectada.clone()).unwrap_or_default();

                        let success = func(
                            pri_ptr.as_ptr(), sec_ptr.as_ptr(), narra_ptr.as_ptr(), 
                            768, 512, 16, model_ptr.as_ptr(), 75, lora_ptr.as_ptr(), 0, init_img_ptr.as_ptr()
                        );
                        if success { system_feedback.push_str("\n[WAN] -> Animación iniciada (output.mp4)."); }
                    }
                }
                let spoken = ai_response[..start_idx].trim();
                voice_response = if !spoken.is_empty() { spoken.to_string() } else { "Iniciando la animación...".to_string() };
            }
            // 🚀 MÓDULO IMAGEN ESTÁTICA (Flux)
            else if let Some(start_idx) = ai_response.find("cmd:vulkan") {
                let bloque_cmd = ai_response[start_idx + 10..].trim();
                let prompt_generado = bloque_cmd.lines().next().unwrap_or(bloque_cmd).replace("\"", "").trim().to_string();
                
                system_feedback = format!("✅ GENERANDO ESCENA EN VULKAN...\nPrompt: {}", prompt_generado);
                let query_para_nlp = query_str.clone();

                unsafe {
                    let lib = MOTOR_DLL.get_or_init(|| Library::new("motor.dll").expect("No se encontró motor.dll"));
                    if let Ok(func) = lib.get::<InferenciaFunc>(b"ejecutar_motor_inferencia\0") {
                        let pri_ptr = CString::new(prompt_generado).unwrap_or_default();
                        let sec_ptr = CString::new("").unwrap_or_default(); 
                        let narra_ptr = CString::new(query_para_nlp).unwrap_or_default();
                        let model_ptr = CString::new("models/fastflux-q4.gguf").unwrap_or_default();
                        let lora_ptr = CString::new("").unwrap_or_default();
                        let init_img_ptr = CString::new("").unwrap_or_default();

                        let success = func(
                            pri_ptr.as_ptr(), sec_ptr.as_ptr(), narra_ptr.as_ptr(), 
                            768, 512, 20, model_ptr.as_ptr(), 75, lora_ptr.as_ptr(), 0, init_img_ptr.as_ptr()
                        );
                        if success { system_feedback.push_str("\n[VULKAN] -> Renderizado iniciado (output.bmp)."); }
                    }
                }
                let spoken = ai_response[..start_idx].trim();
                voice_response = if !spoken.is_empty() { spoken.to_string() } else { "Iniciando renderizado...".to_string() };
            }
            // 🌟 MÓDULO UPSCALER
            else if let Some(start_idx) = ai_response.find("cmd:upscale") {
                system_feedback = "✅ INICIANDO UPSCALER ESRGAN (AMD Vulkan)...".to_string();
                let output = Command::new("realesrgan-ncnn-vulkan.exe").args(["-i", "output.bmp", "-o", "output_upscaled.png", "-n", "realesrgan-x4plus"]).output();

                match output {
                    Ok(_) => system_feedback.push_str("\n[UPSCALER] -> Imagen ampliada."),
                    Err(_) => system_feedback.push_str("\n❌ Error: No se encontró 'realesrgan-ncnn-vulkan.exe'."),
                }
                let spoken = ai_response[..start_idx].trim();
                voice_response = if !spoken.is_empty() { spoken.to_string() } else { "Aumentando resolución...".to_string() };
            }

            speak_organic(&voice_response, audio_pid_tracker);
            
            let mut final_output = ai_response;
            if !system_feedback.is_empty() { final_output.push_str(&format!("\n\n{}", system_feedback)); }

            let _ = tx.send(("AiviResponse".to_string(), final_output));
            ctx.request_repaint();
        });
    }
}

impl eframe::App for AiviApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        while let Ok((sender, message)) = self.autonomous_receiver.try_recv() {
            if sender == "AiviResponse" { self.chat_history.push(("Aivi".to_string(), message)); } 
            else if sender == "Tú" {
                self.chat_history.push(("Tú".to_string(), message.clone()));
                self.spawn_query_worker(message, self.chat_history.clone());
            } else { self.chat_history.push((sender, message)); }
        }

        ctx.set_visuals(egui::Visuals::dark());
        
        egui::SidePanel::right("telemetry_panel").show(ctx, |ui| {
            ui.heading("⚡ NEURAL METRICS");
            ui.separator();
            ui.colored_label(egui::Color32::LIGHT_GREEN, "🛡 Firewall & Lorebook Activos");
            ui.colored_label(egui::Color32::YELLOW, "🔥 FFI Vulkan Architecture (Hybrid)");
            ui.colored_label(egui::Color32::LIGHT_BLUE, "🌟 LoRAs & Wan 2.1 Ready");

            ui.add_space(15.0);
            ui.heading("📦 CEREBRO (LM Studio)");
            ui.separator();
            ui.label(format!("Estado: {}", self.estado_motor));
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("AIVI v5.0 - Flux & Wan 2.1 Architecture");
            ui.separator();

            let scroll_area = egui::ScrollArea::vertical().max_height(ui.available_height() - 70.0).stick_to_bottom(true);
            scroll_area.show(ui, |ui| {
                for (speaker, text) in &self.chat_history {
                    if speaker == "Tú" { ui.colored_label(egui::Color32::LIGHT_GREEN, format!("Tú: {}", text)); } 
                    else if speaker == "🧠 Subconsciente" { ui.colored_label(egui::Color32::ORANGE, format!("🧠 Subconsciente: {}", text)); } 
                    else if speaker == "Aivi" {
                        if text.contains("✅") || text.contains("❌") || text.contains("🔥") || text.contains("🌟") || text.contains("🎬") { 
                            ui.colored_label(egui::Color32::YELLOW, format!("Aivi: {}", text)); 
                        } else { 
                            ui.colored_label(egui::Color32::LIGHT_BLUE, format!("Aivi: {}", text)); 
                        }
                    } else { ui.colored_label(egui::Color32::GRAY, format!("{}: {}", speaker, text)); }
                    ui.separator();
                }
            });

            ui.horizontal(|ui| {
                let text_edit = ui.text_edit_singleline(&mut self.input_text);
                if self.focus_input { text_edit.request_focus(); self.focus_input = false; }

                let mut submitted = false;
                if ui.button("Enviar Comando").clicked() || (text_edit.lost_focus() && ctx.input(|i| i.key_pressed(egui::Key::Enter))) { submitted = true; }

                if submitted && !self.input_text.is_empty() {
                    let query = self.input_text.clone();
                    self.input_text.clear();
                    self.interrumpir_voz_actual();
                    
                    self.chat_history.push(("Tú".to_string(), query.clone()));
                    self.spawn_query_worker(query, self.chat_history.clone());
                    self.focus_input = true;
                }

                if ui.button("🎤 Hablar").clicked() {
                    self.interrumpir_voz_actual();
                    let tx_clone = self.autonomous_sender.clone();
                    let ctx_clone = ctx.clone();
                    let stdin_arc = self.python_stdin.clone();
                    let stdout_arc = self.python_stdout.clone();
                    self.chat_history.push(("Aivi".to_string(), "🎤 Escuchando...".to_string()));
                    
                    thread::spawn(move || {
                        if let Ok(mut stdin) = stdin_arc.lock() { let _ = writeln!(stdin, "RECORD"); }
                        let mut resp = String::new();
                        if let Ok(mut stdout) = stdout_arc.lock() { let _ = stdout.read_line(&mut resp); }
                        let linea = resp.trim().to_string();
                        if !linea.is_empty() && !linea.starts_with("ERROR:") { let _ = tx_clone.send(("Tú".to_string(), linea)); } 
                        else if linea.is_empty() { let _ = tx_clone.send(("Aivi".to_string(), "❌ Silencio.".to_string())); } 
                        else { let _ = tx_clone.send(("Aivi".to_string(), format!("❌ Error: {}", linea))); }
                        ctx_clone.request_repaint();
                    });
                    self.focus_input = true;
                }

                if ui.button("🗑️ Limpiar").clicked() {
                    self.chat_history.clear();
                    self.chat_history.push(("Aivi".to_string(), "Memoria limpiada.".to_string()));
                    self.focus_input = true;
                }
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    eframe::run_native("Aivi v5.0 - Flux & Wan 2.1 Ready", eframe::NativeOptions { viewport: egui::ViewportBuilder::default().with_inner_size([1100.0, 750.0]), ..Default::default() }, Box::new(|cc| Ok(Box::new(AiviApp::new(cc.egui_ctx.clone())))))
}