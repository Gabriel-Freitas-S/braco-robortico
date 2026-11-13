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

/// Movimentos ficam no diretório de dados do usuário (não no diretório atual):
/// pelo menu do sistema o app abre com cwd = $HOME.
fn dir_movimentos() -> std::path::PathBuf {
    let base = std::env::var("XDG_DATA_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| {
            std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
                .join(".local/share")
        });
    base.join("robocontrol/movimentos")
}
const TICK: Duration = Duration::from_millis(50);
/// Altura mínima de qualquer botão (tela de toque).
const ALVO: f32 = 36.0;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([980.0, 680.0])
            .with_min_inner_size([720.0, 480.0])
            .with_app_id("robocontrol")
            .with_title("RoboControl — braço 4-DOF"),
        ..Default::default()
    };
    eframe::run_native(
        "RoboControl",
        options,
        Box::new(|cc| {
            aplica_tema(&cc.egui_ctx);
            Ok(Box::new(App::new()))
        }),
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
    /** movimento aguardando confirmação de exclusão */
    excluir: Option<PathBuf>,
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
            excluir: None,
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
        self.files = Recording::list(&dir_movimentos());
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
                    ui.label(egui::RichText::new(format!("boots={b}")).small().color(COR_MUTED));
                }
                ui.label(egui::RichText::new(format!("fw {}", self.version)).small().color(COR_MUTED));
                let (txt, ct, cf) = if self.link.is_none() {
                    ("desconectado", COR_MUTED, COR_FUNDO)
                } else if self.pc {
                    ("PC no controle", COR_OK, COR_OK_FUNDO)
                } else {
                    ("joystick no controle", COR_WARN, COR_WARN_FUNDO)
                };
                pill(ui, txt, ct, cf);
            });
        });
        if let Some(w) = &self.reboot {
            ui.colored_label(COR_ERRO, w);
        }
    }

    fn reset_targets(&mut self) {
        self.pose = self.live;
    }

    /// Cabeçalho da seção + dois pads (cada um com as duas leituras embaixo).
    fn sticks(&mut self, ui: &mut egui::Ui) {
        const GAP: f32 = 44.0;

        ui.horizontal(|ui| {
            titulo_secao(ui, "Controle por toque");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.link.is_none() {
                    pill(ui, "conecte a placa para começar", COR_MUTED, COR_FUNDO);
                } else if self.pc {
                    pill(ui, "PC no controle · joystick travado", COR_OK, COR_OK_FUNDO);
                } else {
                    pill(ui, "joystick no controle", COR_WARN, COR_WARN_FUNDO);
                }
            });
        });
        ui.add_space(10.0);

        let avail = ui.available_size();
        //  Cada coluna = pad + leituras AO LADO (não embaixo): na tela baixa é o
        //  que libera diâmetro. 2 colunas + GAP precisam caber na largura.
        const COL_LADO: f32 = 116.0; // leituras empilhadas ao lado do pad
        let lado = ((avail.y - 34.0)
            .min((avail.x - GAP - 2.0 * COL_LADO) * 0.5))
            .clamp(120.0, 420.0);

        if Frames::a_cada(90) {
            eprintln!("LAYOUT avail={avail:?} lado={lado}");
        }

        //  Centralização: o espaço tem que ser somado DENTRO do layout horizontal.
        let sobra = (avail.x - (lado * 2.0 + GAP + 2.0 * COL_LADO)).max(0.0);
        let live = self.live;
        ui.horizontal(|ui| {
            ui.add_space(sobra / 2.0);
            pad(ui, "pad_esq", "BASE · COTOVELO", &mut self.stick_l, lado,
                [("base", live[0]), ("cotovelo", live[2])]);
            ui.add_space(GAP);
            pad(ui, "pad_dir", "GARRA · OMBRO", &mut self.stick_r, lado,
                [("garra", live[3]), ("ombro", live[1])]);
        });
    }

    fn recordings_panel(&mut self, ui: &mut egui::Ui) {
        //  O bloco de replay + rodapé é reservado PRIMEIRO, como painel inferior:
        //  assim a lista nunca empurra o "Reproduzir" para fora da tela.
        egui::TopBottomPanel::bottom("replay_painel").show_inside(ui, |ui| {
            if let Some(r) = self.loaded.clone() {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new(&r.name).strong());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "{:.1}s · {} amostras",
                                r.duration_ms() as f32 / 1000.0,
                                r.samples.len()
                            ))
                            .small()
                            .color(COR_MUTED),
                        );
                    });
                });
                ui.add_space(2.0);
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
                ui.add_space(4.0);
                ui.separator();
            }
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new(dir_movimentos().display().to_string())
                    .small()
                    .color(cor(0x5d, 0x6a, 0x78)),
            );
        });

        ui.add_space(6.0);
        titulo_secao(ui, "Gravação");
        ui.add_space(6.0);

        if let Some((rec, t0)) = &self.recording {
            ui.horizontal(|ui| {
                ui.label(egui::RichText::new("●").color(COR_REC).size(14.0));
                ui.label(
                    egui::RichText::new(format!(
                        "{:.1}s · {} amostras",
                        t0.elapsed().as_secs_f32(),
                        rec.samples.len()
                    ))
                    .color(COR_REC),
                );
            });
            ui.add_space(4.0);
            if ui
                .add_sized([ui.available_width(), ALVO], egui::Button::new("Parar e salvar"))
                .clicked()
            {
                if let Some((mut rec, _)) = self.recording.take() {
                    rec.name = self.rec_name.clone();
                    match rec.save(&dir_movimentos()) {
                        Ok(p) => {
                            self.loga(format!("salvo em {}", p.display()));
                            self.refresh_files();
                        }
                        Err(e) => self.loga(format!("erro ao salvar: {e}")),
                    }
                }
            }
        } else {
            ui.add_sized(
                [ui.available_width(), ALVO],
                egui::TextEdit::singleline(&mut self.rec_name).hint_text("nome do movimento"),
            );
            ui.add_space(4.0);
            if ui
                .add_enabled(
                    self.link.is_some(),
                    egui::Button::new("●  Gravar").min_size(egui::vec2(ui.available_width(), ALVO)),
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

        ui.add_space(14.0);
        ui.horizontal(|ui| {
            titulo_secao(ui, "Movimentos");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.small_button("atualizar").clicked() {
                    self.refresh_files();
                }
            });
        });
        ui.add_space(6.0);

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .max_height(ui.available_height())
            .show(ui, |ui| {
                if self.files.is_empty() {
                    ui.label(egui::RichText::new("nenhum movimento salvo").small().color(COR_MUTED));
                }
                let files = self.files.clone();
                for f in files {
                    let nome = f
                        .file_name()
                        .map(|n| n.to_string_lossy().trim_end_matches(".json").to_string())
                        .unwrap_or_default();

                    if self.excluir.as_ref() == Some(&f) {
                        //  Confirmação no lugar do nome: excluir um movimento
                        //  ensinado não tem desfazer.
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized(
                                    [ui.available_width() - ALVO - 10.0, ALVO],
                                    egui::Button::new("excluir este?").fill(cor(0x4a, 0x22, 0x22)),
                                )
                                .clicked()
                            {
                                match Recording::delete(&f) {
                                    Ok(()) => {
                                        self.loga(format!("excluído {nome}"));
                                        if self.loaded.as_ref().is_some_and(|l| l.name == nome) {
                                            self.loaded = None;
                                            self.play = None;
                                        }
                                        self.refresh_files();
                                    }
                                    Err(e) => self.loga(format!("erro ao excluir: {e}")),
                                }
                                self.excluir = None;
                            }
                            if ui.add_sized([ALVO, ALVO], egui::Button::new("✕")).clicked() {
                                self.excluir = None;
                            }
                        });
                        continue;
                    }

                    ui.horizontal(|ui| {
                        if ui
                            .add_sized([ALVO, ALVO], egui::Button::new("▶"))
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
                        let selecionado = self.loaded.as_ref().is_some_and(|l| l.name == nome);
                        let largura_nome = ui.available_width() - ALVO - 10.0;
                        if ui
                            .add_sized(
                                [largura_nome.max(60.0), ALVO],
                                egui::Button::selectable(selecionado, nome),
                            )
                            .clicked()
                        {
                            if let Ok(r) = Recording::load(&f) {
                                self.loaded = Some(r);
                            }
                        }
                        if ui
                            .add_sized([ALVO, ALVO], egui::Button::new("✕"))
                            .on_hover_text("excluir")
                            .clicked()
                        {
                            self.excluir = Some(f.clone());
                        }
                    });
                }
            });
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

// ---------------------------------------------------------------- paleta --

const fn cor(r: u8, g: u8, b: u8) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}

const COR_FUNDO: egui::Color32 = cor(0x18, 0x1d, 0x23);
const COR_MUTED: egui::Color32 = cor(0x84, 0x93, 0xa1);
const COR_ACENTO: egui::Color32 = cor(0x7f, 0xb2, 0xe5);
const COR_OK: egui::Color32 = cor(0x8f, 0xd6, 0xa2);
const COR_OK_FUNDO: egui::Color32 = cor(0x1b, 0x2e, 0x23);
const COR_WARN: egui::Color32 = cor(0xd9, 0xc3, 0x76);
const COR_WARN_FUNDO: egui::Color32 = cor(0x2c, 0x28, 0x18);
const COR_ERRO: egui::Color32 = cor(0xe2, 0x86, 0x86);
const COR_REC: egui::Color32 = cor(0xe0, 0x7b, 0x7b);

/// Tema único do app: superfícies escuras azuladas, cantos de 8 px e alvos de
/// toque de 36 px. Aplicado uma vez, no arranque.
fn aplica_tema(ctx: &egui::Context) {
    use egui::{FontFamily, FontId, TextStyle};
    let mut st = (*ctx.style()).clone();
    let mut v = egui::Visuals::dark();

    v.panel_fill = cor(0x14, 0x18, 0x1e);
    v.window_fill = cor(0x18, 0x1d, 0x23);
    v.extreme_bg_color = cor(0x0d, 0x11, 0x15);
    v.faint_bg_color = cor(0x1a, 0x20, 0x27);
    v.override_text_color = Some(cor(0xd6, 0xdf, 0xe9));
    v.selection.bg_fill = cor(0x2b, 0x5f, 0x8f);
    v.selection.stroke = egui::Stroke::new(1.5_f32, COR_ACENTO);
    v.hyperlink_color = COR_ACENTO;

    let raio = egui::CornerRadius::same(8);
    v.widgets.noninteractive.corner_radius = raio;
    v.widgets.noninteractive.bg_fill = cor(0x19, 0x1f, 0x26);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, cor(0x24, 0x2c, 0x35));

    for w in [
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.corner_radius = raio;
    }
    v.widgets.inactive.weak_bg_fill = cor(0x20, 0x28, 0x32);
    v.widgets.inactive.bg_fill = cor(0x20, 0x28, 0x32);
    v.widgets.inactive.bg_stroke = egui::Stroke::new(1.0_f32, cor(0x2c, 0x36, 0x43));
    v.widgets.hovered.weak_bg_fill = cor(0x27, 0x32, 0x40);
    v.widgets.hovered.bg_fill = cor(0x27, 0x32, 0x40);
    v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, cor(0x42, 0x57, 0x6e));
    v.widgets.active.weak_bg_fill = cor(0x30, 0x3e, 0x50);
    v.widgets.active.bg_fill = cor(0x30, 0x3e, 0x50);
    v.widgets.active.bg_stroke = egui::Stroke::new(1.5_f32, COR_ACENTO);

    st.visuals = v;
    st.spacing.button_padding = egui::vec2(14.0, 9.0);
    st.spacing.item_spacing = egui::vec2(10.0, 8.0);
    st.spacing.interact_size.y = ALVO;

    st.text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    st.text_styles.insert(TextStyle::Button, FontId::new(14.5, FontFamily::Proportional));
    st.text_styles.insert(TextStyle::Heading, FontId::new(19.0, FontFamily::Proportional));
    st.text_styles.insert(TextStyle::Small, FontId::new(11.5, FontFamily::Proportional));
    ctx.set_style(st);
}

/// Título de seção: caixa alta, pequeno e discreto.
fn titulo_secao(ui: &mut egui::Ui, texto: &str) {
    ui.label(
        egui::RichText::new(texto.to_uppercase())
            .small()
            .strong()
            .color(COR_MUTED),
    );
}

/// Etiqueta arredondada de estado.
fn pill(ui: &mut egui::Ui, texto: &str, cor_texto: egui::Color32, fundo: egui::Color32) {
    egui::Frame::NONE
        .fill(fundo)
        .corner_radius(20)
        .inner_margin(egui::Margin::symmetric(12, 5))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(texto).strong().color(cor_texto));
        });
}

/// Contador de frames só para não inundar o stderr com diagnóstico de layout.
struct Frames;
impl Frames {
    fn a_cada(n: u32) -> bool {
        use std::sync::atomic::{AtomicU32, Ordering};
        static CONT: AtomicU32 = AtomicU32::new(0);
        CONT.fetch_add(1, Ordering::Relaxed) % n == 0
    }
}

/// Leitura de uma junta, com largura fixa para alinhar com o pad.
fn chip(ui: &mut egui::Ui, nome: &str, valor: f32, largura: f32) {
    egui::Frame::NONE
        .fill(cor(0x1c, 0x23, 0x2b))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 7))
        .show(ui, |ui| {
            ui.set_width((largura - 24.0).max(24.0));
            ui.vertical(|ui| {
                ui.label(egui::RichText::new(nome.to_uppercase()).small().color(COR_MUTED));
                ui.label(
                    egui::RichText::new(format!("{valor:.0}°"))
                        .size(21.0)
                        .strong()
                        .color(cor(0xe8, 0xef, 0xf6)),
                );
            });
        });
}

/// Pad de toque estilo alavanca: arrastar move o knob, soltar volta ao centro
/// (centro = segurar a posição, igual ao joystick físico solto). As duas
/// leituras ficam empilhadas ao lado, para o pad poder usar toda a altura.
fn pad(
    ui: &mut egui::Ui,
    id: &str,
    titulo: &str,
    v: &mut egui::Vec2,
    lado: f32,
    leituras: [(&str, f32); 2],
) {
    const LARG_LEITURA: f32 = 116.0;
    const ESPACO: f32 = 12.0;

    ui.push_id(id, |ui| {
        ui.vertical(|ui| {
            ui.set_width(lado + ESPACO + LARG_LEITURA);
            ui.label(egui::RichText::new(titulo).small().strong().color(COR_MUTED));
            ui.add_space(2.0);

            ui.horizontal(|ui| {
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(lado, lado), egui::Sense::click_and_drag());
                let centro = rect.center();
                let raio = lado * 0.5 - 4.0;

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

                let ativo = *v != egui::Vec2::ZERO;
                let p = ui.painter();
                p.circle_filled(centro, raio, cor(0x1c, 0x22, 0x2a));
                if ativo {
                    p.circle_filled(
                        centro,
                        raio,
                        egui::Color32::from_rgba_unmultiplied(0x7f, 0xb2, 0xe5, 18),
                    );
                }
                p.circle_stroke(
                    centro,
                    raio,
                    egui::Stroke::new(
                        if ativo { 2.5_f32 } else { 1.5_f32 },
                        if ativo { COR_ACENTO } else { cor(0x2f, 0x3a, 0x47) },
                    ),
                );
                let linha = egui::Stroke::new(1.0_f32, cor(0x25, 0x2d, 0x37));
                p.line_segment([centro - egui::vec2(raio, 0.0), centro + egui::vec2(raio, 0.0)], linha);
                p.line_segment([centro - egui::vec2(0.0, raio), centro + egui::vec2(0.0, raio)], linha);
                p.circle_stroke(centro, raio * 0.30, egui::Stroke::new(1.0_f32, cor(0x22, 0x29, 0x32)));

                let knob = centro + *v * raio;
                p.circle_filled(
                    knob,
                    raio * 0.22,
                    if ativo { cor(0x9e, 0xcb, 0xf5) } else { COR_ACENTO },
                );
                p.circle_stroke(knob, raio * 0.22, egui::Stroke::new(2.0_f32, cor(0x0f, 0x14, 0x19)));

                ui.add_space(ESPACO);
                ui.vertical(|ui| {
                    ui.set_width(LARG_LEITURA);
                    //  Centraliza o par de leituras na altura do pad.
                    let altura_par = 118.0;
                    ui.add_space(((lado - altura_par) * 0.5).max(0.0));
                    chip(ui, leituras[0].0, leituras[0].1, LARG_LEITURA);
                    ui.add_space(8.0);
                    chip(ui, leituras[1].0, leituras[1].1, LARG_LEITURA);
                });
            });
        });
    });
}
