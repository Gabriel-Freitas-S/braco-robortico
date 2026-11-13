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
/// Altura mínima de qualquer botão (tela de toque).
const ALVO: f32 = 36.0;

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
    /// Deflexão dos pads de toque, -1..1 (x = direita, y = baixo na tela).
    stick_l: egui::Vec2,
    stick_r: egui::Vec2,
    limits: [[f32; 2]; 4],
    pc: bool,
    garra_travada: bool,

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
            stick_l: egui::Vec2::ZERO,
            stick_r: egui::Vec2::ZERO,
            limits: [[0.0, 180.0], [0.0, 180.0], [0.0, 180.0], [0.0, 90.0]],
            pc: false,
            garra_travada: false,
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
        self.boots = None;
        self.reboot = None;
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
        self.stick_l = egui::Vec2::ZERO;
        self.stick_r = egui::Vec2::ZERO;
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
                        if d.boots > antes && antes > 0 {
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
                    self.garra_travada = s.trava;
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

        // Toque num pad cancela o replay: quem está com a mão manda.
        if self.play.is_some() && (self.stick_l != egui::Vec2::ZERO || self.stick_r != egui::Vec2::ZERO) {
            self.play = None;
            self.loga("replay cancelado (toque no pad)");
        }

        if self.link.is_some() && self.pc && now - self.last_send >= TICK {
            self.last_send = now;
            let cmd = if self.play.is_some() {
                protocol::cmd_targets(self.pose)
            } else {
                protocol::cmd_sticks([
                    (self.stick_l.x * 100.0) as i32,
                    (-self.stick_l.y * 100.0) as i32,   // tela: y para baixo; stick: y para cima
                    (self.stick_r.x * 100.0) as i32,
                    (-self.stick_r.y * 100.0) as i32,
                ])
            };
            self.send(&cmd);
        }
    }

    fn top_bar(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.add_sized([110.0, ALVO], egui::Button::new("Atualizar")).clicked() {
                self.refresh_ports();
            }
            egui::ComboBox::from_id_salt("porta")
                .width(150.0)
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
                if ui.add_sized([110.0, ALVO], egui::Button::new("Conectar")).clicked() {
                    self.connect();
                }
            } else if ui.add_sized([130.0, ALVO], egui::Button::new("Desconectar")).clicked() {
                self.disconnect();
            }

            ui.separator();

            if self.pc {
                if ui.add_sized([190.0, ALVO], egui::Button::new("Devolver ao joystick")).clicked() {
                    self.send(protocol::CMD_RELEASE);
                    self.pc = false;
                    self.reset_targets();
                }
            } else {
                if ui
                    .add_enabled(
                        self.link.is_some(),
                        egui::Button::new("Assumir controle (PC)").min_size(egui::vec2(200.0, ALVO)),
                    )
                    .clicked()
                {
                    self.assume();
                }
            }

            let (rotulo, cor_fundo, cor_txt) = if self.garra_travada {
                ("Liberar garra", egui::Color32::from_rgb(34, 62, 46), egui::Color32::from_rgb(130, 210, 150))
            } else {
                ("Travar garra", egui::Color32::from_rgb(30, 42, 62), egui::Color32::from_rgb(120, 165, 225))
            };
            if ui
                .add_sized(
                    [150.0, ALVO],
                    egui::Button::new(egui::RichText::new(rotulo).color(cor_txt)).fill(cor_fundo),
                )
                .on_hover_text("mantém pressão contínua no objeto e bloqueia a abertura")
                .clicked()
            {
                let travar = !self.garra_travada;
                self.send(protocol::cmd_lock(travar));
                self.garra_travada = travar;
            }

            if ui
                .add_sized(
                    [130.0, ALVO + 6.0],
                    egui::Button::new(egui::RichText::new("STOP").strong().size(16.0))
                        .fill(egui::Color32::from_rgb(70, 26, 26)),
                )
                .clicked()
            {
                self.stop();
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(b) = self.boots {
                    ui.label(format!("boots={b}"));
                }
                ui.label(format!("fw {}", self.version));
                let (txt, cor) = if self.link.is_none() {
                    ("desconectado", egui::Color32::GRAY)
                } else if self.pc {
                    ("PC no controle", egui::Color32::from_rgb(90, 200, 120))
                } else {
                    ("joystick no controle", egui::Color32::from_rgb(220, 190, 90))
                };
                ui.colored_label(cor, egui::RichText::new(txt).strong());
            });
        });
        if let Some(w) = &self.reboot {
            ui.colored_label(egui::Color32::from_rgb(230, 120, 120), w);
        }
    }

    fn reset_targets(&mut self) {
        self.pose = self.live;
    }

    /// Dois pads de toque ocupando a altura disponível, no layout da bancada:
    /// esquerdo = Base/Cotovelo, direito = Garra/Ombro.
    fn sticks(&mut self, ui: &mut egui::Ui) {
        const GAP: f32 = 36.0;
        let altura_util = ui.available_height() - 110.0; // rótulo + leituras
        let lado = altura_util
            .min((ui.available_width() - GAP) * 0.5)
            .clamp(150.0, 460.0);

        //  O espaço de centralização tem que ser somado DENTRO do layout
        //  horizontal: no vertical ele empurraria os pads para fora da tela.
        let sobra = (ui.available_width() - (lado * 2.0 + GAP)).max(0.0);
        let live = self.live;
        ui.with_layout(egui::Layout::left_to_right(egui::Align::Min), |ui| {
            ui.add_space(sobra / 2.0);
            pad(ui, "pad_esq", "BASE · COTOVELO", &mut self.stick_l, lado,
                ("base", live[0]), ("cotovelo", live[2]));
            ui.add_space(GAP);
            pad(ui, "pad_dir", "GARRA · OMBRO", &mut self.stick_r, lado,
                ("garra", live[3]), ("ombro", live[1]));
        });
    }

    fn recordings_panel(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        ui.heading("Gravação");
        ui.add_space(2.0);

        if self.recording.is_some() {
            if let Some((rec, t0)) = &self.recording {
                ui.label(format!(
                    "gravando… {:.1}s · {} amostras",
                    t0.elapsed().as_secs_f32(),
                    rec.samples.len()
                ));
            }
            ui.add_space(4.0);
            if ui
                .add_sized([ui.available_width(), ALVO], egui::Button::new("Parar e salvar"))
                .clicked()
            {
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
        } else {
            ui.label("nome");
            ui.add_sized([ui.available_width(), ALVO], egui::TextEdit::singleline(&mut self.rec_name));
            ui.add_space(4.0);
            if ui
                .add_enabled(
                    self.link.is_some(),
                    egui::Button::new("Gravar").min_size(egui::vec2(ui.available_width(), ALVO)),
                )
                .clicked()
            {
                self.recording = Some((
                    Recording { name: self.rec_name.clone(), samples: Vec::new() },
                    Instant::now(),
                ));
                self.loga("gravação iniciada");
            }
        }

        ui.add_space(10.0);
        ui.separator();
        ui.heading("Movimentos");
        if ui.add_sized([ui.available_width(), ALVO], egui::Button::new("Atualizar lista")).clicked() {
            self.refresh_files();
        }
        ui.add_space(4.0);

        egui::ScrollArea::vertical()
            .max_height(180.0)
            .show(ui, |ui| {
                let files = self.files.clone();
                for f in files {
                    let nome = f
                        .file_name()
                        .map(|n| n.to_string_lossy().trim_end_matches(".json").to_string())
                        .unwrap_or_default();
                    ui.horizontal(|ui| {
                        if ui
                            .add_sized([40.0, ALVO], egui::Button::new("▶"))
                            .on_hover_text("carregar")
                            .clicked()
                        {
                            match Recording::load(&f) {
                                Ok(r) => {
                                    self.loaded = Some(r);
                                    self.loga(format!("carregado {nome}"));
                                }
                                Err(e) => self.loga(format!("erro ao ler {nome}: {e}")),
                            }
                        }
                        let selecionado = self
                            .loaded
                            .as_ref()
                            .is_some_and(|l| l.name == nome);
                        if ui
                            .add_sized([ui.available_width(), ALVO], egui::Button::selectable(selecionado, nome))
                            .clicked()
                        {
                            if let Ok(r) = Recording::load(&f) {
                                self.loaded = Some(r);
                            }
                        }
                    });
                }
            });

        ui.add_space(6.0);
        if let Some(r) = self.loaded.clone() {
            ui.separator();
            ui.label(egui::RichText::new(&r.name).strong());
            ui.label(format!("{:.1}s · {} amostras", r.duration_ms() as f32 / 1000.0, r.samples.len()));
            ui.checkbox(&mut self.looping, "repetir");
            let pode = self.link.is_some() && self.play.is_none();
            if ui
                .add_enabled(
                    pode,
                    egui::Button::new("Reproduzir").min_size(egui::vec2(ui.available_width(), ALVO)),
                )
                .clicked()
            {
                self.assume();
                self.play = Some((r.clone(), Instant::now()));
                self.loga("reproduzindo…");
            }
            if self.play.is_some()
                && ui
                    .add_sized([ui.available_width(), ALVO], egui::Button::new("Parar replay"))
                    .clicked()
            {
                self.stop();
            }
        }
    }

    /// Log recolhível: fechado, os pads ficam com a altura toda da tela.
    fn log_panel(&mut self, ui: &mut egui::Ui) {
        egui::CollapsingHeader::new(egui::RichText::new("log").small())
            .default_open(false)
            .show(ui, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(110.0)
                    .show(ui, |ui| {
                        for l in &self.log {
                            ui.monospace(egui::RichText::new(l).small());
                        }
                    });
            });
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Alvos de toque: a janela vive no ScreenPad, onde o dedo é mais gordo
        // que o mouse.
        let mut estilo = (*ctx.style()).clone();
        estilo.spacing.button_padding = egui::vec2(14.0, 9.0);
        estilo.spacing.item_spacing = egui::vec2(10.0, 8.0);
        estilo.spacing.interact_size.y = ALVO;
        ctx.set_style(estilo);

        self.tick(Instant::now());

        egui::TopBottomPanel::top("topo").show(ctx, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui);
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("log").show(ctx, |ui| self.log_panel(ui));
        egui::SidePanel::right("gravacao")
            .resizable(true)
            .default_width(330.0)
            .min_width(280.0)
            .show(ctx, |ui| self.recordings_panel(ui));
        egui::CentralPanel::default().show(ctx, |ui| self.sticks(ui));

        ctx.request_repaint_after(Duration::from_millis(20));
    }
}

/// Pad de toque estilo alavanca: arrastar move o knob, soltar volta ao centro
/// (centro = segurar a posição, igual ao joystick físico solto).
fn pad(
    ui: &mut egui::Ui,
    id: &str,
    titulo: &str,
    v: &mut egui::Vec2,
    lado: f32,
    eixo_a: (&str, f32),
    eixo_b: (&str, f32),
) {
    ui.push_id(id, |ui| {
        ui.vertical(|ui| {
            ui.label(egui::RichText::new(titulo).strong().size(15.0));
            let (rect, resp) =
                ui.allocate_exact_size(egui::vec2(lado, lado), egui::Sense::click_and_drag());
            let centro = rect.center();
            let raio = lado * 0.5 - 6.0;

            if resp.is_pointer_button_down_on() {
                if let Some(pos) = resp.interact_pointer_pos() {
                    let mut d = (pos - centro) / raio;
                    if d.length() > 1.0 {
                        d = d.normalized();
                    }
                    *v = d;
                }
            } else {
                *v = egui::Vec2::ZERO;
            }

            let p = ui.painter();
            p.circle_filled(centro, raio, egui::Color32::from_gray(34));
            p.circle_stroke(centro, raio, egui::Stroke::new(2.0_f32, egui::Color32::from_gray(80)));
            let linha = egui::Stroke::new(1.0_f32, egui::Color32::from_gray(58));
            p.line_segment([centro - egui::vec2(raio, 0.0), centro + egui::vec2(raio, 0.0)], linha);
            p.line_segment([centro - egui::vec2(0.0, raio), centro + egui::vec2(0.0, raio)], linha);

            let knob = centro + *v * raio;
            let ativo = *v != egui::Vec2::ZERO;
            p.circle_filled(
                knob,
                raio * 0.24,
                if ativo {
                    egui::Color32::from_rgb(120, 190, 255)
                } else {
                    egui::Color32::from_rgb(95, 170, 235)
                },
            );

            ui.add_space(2.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} {:.0}°", eixo_a.0, eixo_a.1))
                        .monospace()
                        .size(13.0),
                );
                ui.label("·");
                ui.label(
                    egui::RichText::new(format!("{} {:.0}°", eixo_b.0, eixo_b.1))
                        .monospace()
                        .size(13.0),
                );
            });
        });
    });
}
