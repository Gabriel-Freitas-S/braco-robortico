//! RoboControl — controle e gravação do braço 4-DOF por USB.
//!
//! O firmware continua dono do movimento (rampa, limites, hold). Aqui só
//! propomos alvos a 20 Hz e mostramos o que a placa responde.

mod link;
mod motion;
mod protocol;

use link::{Event, Link};
use motion::Recording;
use protocol::Msg;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const DIR_MOVIMENTOS: &str = "movimentos";
const TICK: Duration = Duration::from_millis(50);

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 680.0])
            .with_title("RoboControl — braço 4-DOF"),
        ..Default::default()
    };
    eframe::run_native(
        "RoboControl",
        options,
        Box::new(|_cc| Ok(Box::new(App::new()))),
    )
}

struct App {
    ports: Vec<String>,
    port_idx: usize,
    link: Option<Link>,

    pose: [f32; 4],
    live: [f32; 4],
    limits: [[f32; 2]; 4],
    pc: bool,

    version: String,
    boots: Option<u32>,
    reboot: Option<String>,

    log: Vec<String>,
    last_send: Instant,
    last_diag: Instant,

    recording: Option<(Recording, Instant)>,
    rec_name: String,
    files: Vec<PathBuf>,
    loaded: Option<Recording>,
    play: Option<(Recording, Instant)>,
    looping: bool,
}

impl App {
    fn new() -> Self {
        let mut app = Self {
            ports: Vec::new(),
            port_idx: 0,
            link: None,
            pose: [90.0; 4],
            live: [90.0; 4],
            limits: [[0.0, 180.0], [0.0, 180.0], [0.0, 180.0], [0.0, 90.0]],
            pc: false,
            version: "?".into(),
            boots: None,
            reboot: None,
            log: Vec::new(),
            last_send: Instant::now(),
            last_diag: Instant::now(),
            recording: None,
            rec_name: "movimento".into(),
            files: Vec::new(),
            loaded: None,
            play: None,
            looping: false,
        };
        app.refresh_ports();
        app.refresh_files();
        app
    }

    fn refresh_ports(&mut self) {
        self.ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .filter(|n| n.contains("ACM") || n.contains("USB"))
            .collect();
        self.ports.sort();
        if self.port_idx >= self.ports.len() {
            self.port_idx = 0;
        }
    }

    fn refresh_files(&mut self) {
        self.files = Recording::list(std::path::Path::new(DIR_MOVIMENTOS));
    }

    fn loga(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 400 {
            self.log.remove(0);
        }
    }

    fn connect(&mut self) {
        let Some(path) = self.ports.get(self.port_idx).cloned() else { return };
        match Link::open(&path) {
            Ok(l) => {
                self.loga(format!("conectado em {path}"));
                self.link = Some(l);
                self.boots = None;
                self.reboot = None;
                self.last_diag = Instant::now() - Duration::from_secs(10);
            }
            Err(e) => self.loga(format!("erro ao abrir: {e}")),
        }
    }

    fn disconnect(&mut self) {
        if let Some(mut l) = self.link.take() {
            l.send(protocol::CMD_RELEASE);
        }
        self.pc = false;
        self.play = None;
        self.recording = None;
    }

    fn send(&mut self, cmd: &str) {
        if let Some(l) = self.link.as_mut() {
            l.send(cmd);
        }
    }

    fn assume(&mut self) {
        self.pose = self.live;
        self.send(protocol::CMD_ASSUME);
        self.pc = true;
    }

    fn stop(&mut self) {
        self.send(protocol::CMD_STOP);
        self.pc = false;
        self.play = None;
        self.pose = self.live;
        self.loga("STOP — juntas seguradas");
    }

    /// Leitura da serial, heartbeat de 20 Hz, gravação e replay.
    fn tick(&mut self, now: Instant) {
        let mut events = Vec::new();
        if let Some(l) = self.link.as_ref() {
            if !l.drain(&mut events) {
                self.loga("porta caiu");
                self.link = None;
                self.pc = false;
            }
        }

        for ev in events {
            match ev {
                Event::Error(e) => self.loga(format!("serial: {e}")),
                Event::Msg(Msg::Text(t)) => {
                    if t.starts_with("firmware v") {
                        if let Some(v) = t.strip_prefix("firmware v") {
                            self.version = v.split_whitespace().next().unwrap_or("?").into();
                        }
                    }
                    if !t.starts_with("[JOYSTICKS]") {
                        self.loga(t);
                    }
                }
                Event::Msg(Msg::Diag(d)) => {
                    self.version = d.version.clone();
                    self.limits = d.limits;
                    if let Some(antes) = self.boots {
                        if d.boots != antes {
                            self.reboot = Some(format!(
                                "a placa reiniciou (boots {antes} → {}) — suspeite da alimentação",
                                d.boots
                            ));
                        }
                    }
                    self.boots = Some(d.boots);
                }
                Event::Msg(Msg::Status(s)) => {
                    self.live = s.pose;
                    // O firmware é dono da verdade sobre quem controla.
                    if self.pc && !s.pc_mode {
                        self.pc = false;
                        self.play = None;
                        self.loga("joystick assumiu o controle");
                    }
                    if let Some((rec, t0)) = self.recording.as_mut() {
                        rec.samples.push((t0.elapsed().as_millis() as u32, s.pose));
                    }
                }
            }
        }

        if self.link.is_some() {
            if now - self.last_diag >= Duration::from_secs(2) {
                self.last_diag = now;
                self.send(protocol::CMD_DIAG);
            }
        }

        // Replay: a posição vem da gravação, o firmware interpola o resto.
        if let Some((rec, t0)) = self.play.clone() {
            let dur = rec.duration_ms() as f32;
            let t = t0.elapsed().as_millis() as f32;
            if dur > 0.0 && t > dur {
                if self.looping {
                    self.play = Some((rec.clone(), Instant::now()));
                } else {
                    self.play = None;
                    self.loga("fim do movimento");
                }
            } else if let Some(p) = rec.sample_at(t) {
                self.pose = p;
            }
        }

        if self.link.is_some() && self.pc && now - self.last_send >= TICK {
            self.last_send = now;
            self.send(&protocol::cmd_targets(self.pose));
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.button("Atualizar portas").clicked() {
                self.refresh_ports();
            }
            egui::ComboBox::from_id_salt("porta")
                .selected_text(
                    self.ports
                        .get(self.port_idx)
                        .cloned()
                        .unwrap_or_else(|| "(nenhuma)".into()),
                )
                .show_ui(ui, |ui| {
                    for (i, p) in self.ports.iter().enumerate() {
                        ui.selectable_value(&mut self.port_idx, i, p);
                    }
                });

            if self.link.is_none() {
                if ui.button("Conectar").clicked() {
                    self.connect();
                }
            } else if ui.button("Desconectar").clicked() {
                self.disconnect();
            }

            ui.separator();

            if self.pc {
                if ui.button("Devolver ao joystick").clicked() {
                    self.send(protocol::CMD_RELEASE);
                    self.pc = false;
                    self.reset_targets();
                }
            } else if ui
                .add_enabled(self.link.is_some(), egui::Button::new("Assumir controle (PC)"))
                .clicked()
            {
                self.assume();
            }

            if ui
                .add_sized([120.0, 32.0], egui::Button::new(egui::RichText::new("STOP").strong()))
                .clicked()
            {
                self.stop();
            }

            ui.separator();
            let (txt, cor) = if self.link.is_none() {
                ("desconectado", egui::Color32::GRAY)
            } else if self.pc {
                ("PC no controle", egui::Color32::from_rgb(90, 200, 120))
            } else {
                ("joystick no controle", egui::Color32::from_rgb(220, 190, 90))
            };
            ui.colored_label(cor, txt);
            ui.label(format!("fw {}", self.version));
            if let Some(b) = self.boots {
                ui.label(format!("boots={b}"));
            }
        });
        if let Some(w) = &self.reboot {
            ui.colored_label(egui::Color32::from_rgb(230, 120, 120), w);
        }
    }

    fn reset_targets(&mut self) {
        self.pose = self.live;
    }

    fn joints(&mut self, ui: &mut egui::Ui) {
        for i in 0..4 {
            let (lo, hi) = (self.limits[i][0], self.limits[i][1]);
            ui.horizontal(|ui| {
                ui.add_sized([80.0, 20.0], egui::Label::new(protocol::JOINTS[i]));
                ui.add(
                    egui::Slider::new(&mut self.pose[i], lo..=hi)
                        .fixed_decimals(0)
                        .suffix("°"),
                );
                ui.label(format!("atual {:.0}°", self.live[i]));
            });
            ui.add_space(4.0);
        }
    }

    fn recordings_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("Gravação");

        if self.recording.is_some() {
            if ui.button("Parar gravação").clicked() {
                if let Some((mut rec, _)) = self.recording.take() {
                    rec.name = self.rec_name.clone();
                    match rec.save(std::path::Path::new(DIR_MOVIMENTOS)) {
                        Ok(p) => {
                            self.loga(format!("salvo em {}", p.display()));
                            self.refresh_files();
                        }
                        Err(e) => self.loga(format!("erro ao salvar: {e}")),
                    }
                }
            }
            if let Some((rec, t0)) = &self.recording {
                ui.label(format!(
                    "gravando… {:.1}s · {} amostras",
                    t0.elapsed().as_secs_f32(),
                    rec.samples.len()
                ));
            }
        } else {
            ui.text_edit_singleline(&mut self.rec_name);
            if ui
                .add_enabled(self.link.is_some(), egui::Button::new("Gravar"))
                .clicked()
            {
                self.recording = Some((
                    Recording { name: self.rec_name.clone(), samples: Vec::new() },
                    Instant::now(),
                ));
                self.loga("gravação iniciada");
            }
        }

        ui.separator();
        ui.heading("Movimentos salvos");
        if ui.button("Atualizar lista").clicked() {
            self.refresh_files();
        }
        let files = self.files.clone();
        for f in files {
            let nome = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            ui.horizontal(|ui| {
                if ui.small_button("▶").clicked() {
                    match Recording::load(&f) {
                        Ok(r) => {
                            self.loaded = Some(r);
                            self.loga(format!("carregado {nome}"));
                        }
                        Err(e) => self.loga(format!("erro ao ler {nome}: {e}")),
                    }
                }
                ui.label(nome);
            });
        }

        ui.separator();
        if let Some(r) = self.loaded.clone() {
            ui.label(format!(
                "{} — {:.1}s, {} amostras",
                r.name,
                r.duration_ms() as f32 / 1000.0,
                r.samples.len()
            ));
            ui.checkbox(&mut self.looping, "repetir");
            let pode = self.link.is_some() && self.play.is_none();
            if ui.add_enabled(pode, egui::Button::new("Reproduzir")).clicked() {
                self.assume();
                self.play = Some((r.clone(), Instant::now()));
                self.loga("reproduzindo…");
            }
            if self.play.is_some() && ui.button("Parar replay").clicked() {
                self.stop();
            }
        }
    }

    fn log_panel(&self, ui: &mut egui::Ui) {
        egui::ScrollArea::vertical()
            .stick_to_bottom(true)
            .max_height(140.0)
            .show(ui, |ui| {
                for l in &self.log {
                    ui.monospace(l);
                }
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(Instant::now());

        egui::TopBottomPanel::top("topo").show(ctx, |ui| self.top_bar(ui));
        egui::TopBottomPanel::bottom("log").show(ctx, |ui| self.log_panel(ui));
        egui::SidePanel::right("gravacao")
            .resizable(true)
            .default_width(300.0)
            .show(ctx, |ui| self.recordings_panel(ui));
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Juntas");
            ui.add_space(6.0);
            self.joints(ui);
            ui.add_space(10.0);
            ui.label(format!(
                "alvo: {:.0} ; {:.0} ; {:.0} ; {:.0}",
                self.pose[0], self.pose[1], self.pose[2], self.pose[3]
            ));
        });

        ctx.request_repaint_after(Duration::from_millis(20));
    }
}
