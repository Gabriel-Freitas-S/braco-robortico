//! RoboControl — controle e gravação do braço 4-DOF por USB.
//!
//! O firmware continua dono do movimento (rampa, limites, hold). Aqui só
//! propomos alvos a 20 Hz e mostramos o que a placa responde.
//!
//! # Arquitetura
//!
//! Este crate é só a interface (egui/eframe). Cada responsabilidade tem um
//! módulo e um dono único:
//!
//! | Módulo | Responsabilidade |
//! |---|---|
//! | [`protocol`] | gramática serial, unidades e parsing |
//! | [`link`] | porta serial + thread de leitura |
//! | [`motion`] | gravação, interpolação e arquivos JSON |
//! | [`icons`] | desenho vetorial dos ícones e botões |
//!
//! # Máquina de estados
//!
//! `desconectado → joystick → PC` e volta. O campo `App::pc` é só um espelho do
//! `pc_mode` que o firmware manda no status: **o firmware é a fonte da verdade**.
//! Se o joystick físico assumir, o app larga o replay e os alvos sozinho.
//!
//! # Invariantes de segurança
//!
//! - STOP (`#H`) está sempre visível e devolve o controle ao joystick.
//! - Tocar num pad cancela replay e alvo manual: quem está com a mão manda.
//! - A trava da garra é um único estado no firmware (botão D4 e app), e com ela
//!   ativa a garra só aperta.
//! - Os limites usados na validação da UI vêm do diagnóstico; quem os aplica de
//!   verdade é o firmware, então um alvo fora da faixa não vira movimento.
//!
//! # Ritmo
//!
//! [`TICK`] = 50 ms: a cada tick drenamos a serial, gravamos uma amostra e, se o
//! PC está no controle, enviamos novos alvos/sticks. O diagnóstico é pedido a
//! cada 2 s e a UI repinta a cada 20 ms.
//!
//! # Layout
//!
//! Responsivo por largura/altura disponível: dock lateral a partir de 1024 pt,
//! topbar em uma linha a partir de 1024, pads lado a lado a partir de 760,
//! variante compacta quando a altura fica abaixo de 380. A janela mínima é
//! 760×320, que ainda comporta os pads empilhados.
//!
//! Movimentos gravados ficam no diretório de dados do usuário, nunca no cwd —
//! ver [`dir_movimentos`].

mod icons;
mod link;
mod motion;
mod protocol;

use icons::{botao, botao_quadrado, Icone};
use link::{Event, Link};
use motion::Recording;
use protocol::Msg;
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Movimentos ficam no diretório de dados do usuário (não no diretório atual):
/// pelo menu do sistema o app abre com cwd = $HOME.
///
/// Linux/macOS: respeita `XDG_DATA_HOME`; sem ele, usa `$HOME/.local/share`.
/// Windows: `%APPDATA%\robocontrol\movimentos`.
fn dir_movimentos() -> std::path::PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return std::path::PathBuf::from(xdg).join("robocontrol/movimentos");
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var("APPDATA")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| std::path::PathBuf::from("."));
    #[cfg(not(target_os = "windows"))]
    let base = std::path::PathBuf::from(std::env::var("HOME").unwrap_or_default())
        .join(".local/share");
    base.join("robocontrol/movimentos")
}

/// Período do laço de controle: 50 ms = 20 Hz, a mesma taxa do status do
/// firmware. Também é o intervalo mínimo entre dois comandos de alvo.
const TICK: Duration = Duration::from_millis(50);

/// Alvo mínimo de toque dos controles, em pontos lógicos.
///
/// 48 pt é o menor valor confortável em tela touch (e o que o tema aplica em
/// `interact_size.y`); nenhum botão do app fica abaixo disso.
const ALVO: f32 = 48.0;

/// Largura mínima para o painel de gravação virar dock lateral. Abaixo disso
/// ele desce para o corpo com scroll, senão os pads ficariam espremidos.
const DOCK_MIN_WIDTH: f32 = 1024.0;

/// Largura mínima para a topbar caber em uma linha só.
const TOPBAR_MIN_WIDTH: f32 = 1024.0;

/// Espaço horizontal além do círculo do pad: 12 de respiro + 116 da coluna de
/// leituras de ângulo. Usado para calcular quantos pads cabem na largura.
const PAD_MODULO_EXTRA: f32 = 128.0;

/// Vão entre os dois pads na disposição lado a lado.
const PAD_GAP: f32 = 24.0;

/// Altura fora do círculo do pad (título, legendas, leituras) no modo normal.
const PAD_CHROME_NORMAL: f32 = 144.0;

/// Altura fora do círculo do pad no modo compacto (tela baixa).
const PAD_CHROME_COMPACTO: f32 = 102.0;

/// Abaixo desta altura disponível os pads trocam para o chrome compacto.
const ALTURA_PADS_COMPACTA: f32 = 380.0;

/// Diâmetro máximo do pad: evita um círculo gigante em tela ultrawide.
const PAD_MAX: f32 = 450.0;

/// Largura mínima para os dois pads ficarem lado a lado. Abaixo disso eles
/// empilham e o scroll vertical resolve a altura.
const PAD_LADO_A_LADO_MIN: f32 = 760.0;

/// Resultado do cálculo responsivo dos pads.
#[derive(Clone, Copy, Debug, PartialEq)]
struct GeometriaPads {
    /// Diâmetro do círculo de cada pad, já limitado a 96..=[`PAD_MAX`].
    lado: f32,
    /// `true` quando os dois pads cabem na mesma linha.
    lado_a_lado: bool,
    /// `true` quando a altura disponível pede chrome reduzido.
    compacto: bool,
}

/// Estado de um pad entre frames.
#[derive(Clone, Copy, Default)]
struct PadState {
    /// Posição normalizada do knob, `-1..=1` por eixo. `(0,0)` = centro =
    /// segurar a posição atual.
    stick: egui::Vec2,
    /// Toque ativo (id + posição) para permitir os dois pads ao mesmo tempo.
    touch: Option<(egui::TouchId, egui::Pos2)>,
}

/// Decide se o painel de gravação fica à direita (dock) ou abaixo dos pads.
fn usa_dock_lateral(largura: f32) -> bool {
    largura >= DOCK_MIN_WIDTH
}

/// Decide se a topbar cabe em uma linha só.
fn topbar_em_linha_unica(largura: f32) -> bool {
    largura >= TOPBAR_MIN_WIDTH
}

/// Altura reservada fora do círculo do pad, conforme o modo.
fn pad_chrome(compacto: bool) -> f32 {
    if compacto {
        PAD_CHROME_COMPACTO
    } else {
        PAD_CHROME_NORMAL
    }
}

/// Calcula tamanho e disposição dos pads para o espaço disponível.
///
/// Lado a lado, o lado do pad é o menor entre o que cabe na largura (dois pads +
/// vãos) e o que cabe na altura (menos o chrome); empilhado, a largura manda e
/// a altura fica com o scroll. O resultado fica em `96..=PAD_MAX` para nunca
/// desaparecer nem virar um alvo absurdo.
fn geometria_pads(disponivel: egui::Vec2) -> GeometriaPads {
    let lado_a_lado = disponivel.x >= PAD_LADO_A_LADO_MIN;
    let compacto = lado_a_lado && disponivel.y < ALTURA_PADS_COMPACTA;
    let por_largura = if lado_a_lado {
        (disponivel.x - PAD_GAP - 2.0 * PAD_MODULO_EXTRA) / 2.0
    } else {
        disponivel.x - PAD_MODULO_EXTRA
    };
    // Na disposição empilhada o scroll resolve a altura; na lado a lado os dois
    // pads precisam permanecer visíveis juntos sem corte na base.
    let por_altura = if lado_a_lado {
        (disponivel.y - pad_chrome(compacto)).max(96.0)
    } else {
        PAD_MAX
    };
    GeometriaPads {
        lado: por_largura.min(por_altura).clamp(96.0, PAD_MAX),
        lado_a_lado,
        compacto,
    }
}

/// Encurta texto com reticências sem quebrar caractere Unicode.
///
/// O resultado tem no máximo `limite` caracteres (a reticência conta); limite 0
/// devolve string vazia. Conta `char`, não bytes, porque nomes de movimento
/// aceitam acentos e ideogramas.
fn texto_curto(texto: &str, limite: usize) -> String {
    if texto.chars().count() <= limite {
        return texto.into();
    }
    if limite == 0 {
        return String::new();
    }
    let mut curto: String = texto.chars().take(limite - 1).collect();
    curto.push('…');
    curto
}

/// Abre a janela e instala o tema.
///
/// O tamanho inicial é fixo (1180×560) e `persist_window` fica desligado de
/// propósito: se o egui lembrasse a última sessão, o app abriria maximizado
/// sozinho depois de um fechamento em tela cheia.
fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1180.0, 560.0])
            .with_min_inner_size([760.0, 320.0])
            .with_maximized(false)
            .with_app_id("robocontrol")
            .with_title("RoboControl — braço 4-DOF"),
        centered: true,
        //  Não herda tamanho/posição da sessão anterior: o app sempre abre no
        //  tamanho definido acima (senão ele volta maximizado sozinho).
        persist_window: false,
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

/// Todo o estado da aplicação. Vive na thread da UI; a única concorrência é a
/// thread leitora da serial dentro de [`Link`].
struct App {
    /// Nomes de porta encontrados na última varredura (só ACM/USB).
    ports: Vec<String>,
    /// Índice selecionado em `ports`; sempre válido enquanto a lista não esvazia.
    port_idx: usize,
    /// `None` = desconectado. Presente = porta aberta e thread leitora viva.
    link: Option<Link>,

    /// Alvo proposto pelo PC, em graus, enviado em `#P`. É o que o replay e o
    /// alvo manual manipulam.
    pose: [f32; 4],
    /// Última pose medida pelo firmware, em graus (`Status::pose`). A UI mostra
    /// este valor, nunca `pose`.
    live: [f32; 4],
    /// Pad esquerdo: X = Base, Y = Cotovelo.
    pad_l: PadState,
    /// Pad direito: X = Garra, Y = Ombro.
    pad_r: PadState,
    /// Limites mecânicos por junta vindos do diagnóstico; padrão otimista
    /// `[0,180]` (garra `[0,90]`) até a primeira resposta.
    limits: [[f32; 2]; 4],
    /// Espelho de `Status::pc_mode`. Desligado = joystick no controle.
    pc: bool,
    /// Espelho de `Status::trava` (mesma trava do botão D4).
    garra_travada: bool,

    /// Versão do firmware reportada no banner/diagnóstico.
    version: String,
    /// Contador de boots da última leitura; `None` antes do primeiro diagnóstico.
    boots: Option<u32>,
    /// Aviso de reinício inesperado (salto de `boots`), mostrado na topbar.
    reboot: Option<String>,
    /// Último erro para o banner vermelho; o log guarda o histórico.
    erro: Option<String>,

    /// Histórico de eventos, limitado a 400 linhas (as mais antigas saem).
    log: Vec<String>,
    /// Estado do painel de log (recolhido mostra só a última linha).
    log_aberta: bool,
    /// Instante do último envio de alvos/sticks; controla o ritmo de [`TICK`].
    last_send: Instant,
    /// Instante do último `#B`; o diagnóstico é pedido a cada 2 s.
    last_diag: Instant,

    /// Gravação em andamento + instante inicial (base dos carimbos em ms).
    recording: Option<(Recording, Instant)>,
    /// Movimento aguardando confirmação de exclusão na lista.
    excluir: Option<PathBuf>,
    /// Nome que será dado ao próximo movimento salvo.
    rec_name: String,
    /// Caminhos dos movimentos salvos, em ordem, atualizados por `refresh_files`.
    files: Vec<PathBuf>,
    /// Movimento carregado para replay (não é o mesmo que estar reproduzindo).
    loaded: Option<Recording>,
    /// Replay em andamento + instante inicial do replay.
    play: Option<(Recording, Instant)>,
    /// Ao terminar, recomeça do início em vez de parar.
    looping: bool,
    /// Alvo pontual definido pelo modal de ângulo; enviado via `#P` até ser
    /// cancelado (toque no pad, STOP ou novo replay).
    alvo_manual: bool,
    /// Junta aberta no modal de ângulo, se houver.
    editando_junta: Option<usize>,
    /// Texto do campo de ângulo; validado por `avaliar_alvo` a cada frame.
    editando_texto: String,
    /// Pede foco no campo de ângulo no próximo frame (só uma vez).
    focar_angulo: bool,
}

impl App {
    /// Estado inicial: pose neutra de 90° em todas as juntas, limites padrão e
    /// listas de portas/movimentos já carregadas (nada de IO no primeiro frame).
    fn new() -> Self {
        let mut app = Self {
            ports: Vec::new(),
            port_idx: 0,
            link: None,
            pose: [90.0; 4],
            live: [90.0; 4],
            pad_l: PadState::default(),
            pad_r: PadState::default(),
            limits: [[0.0, 180.0], [0.0, 180.0], [0.0, 180.0], [0.0, 90.0]],
            pc: false,
            garra_travada: false,
            version: "?".into(),
            boots: None,
            reboot: None,
            erro: None,
            log: Vec::new(),
            log_aberta: false,
            last_send: Instant::now(),
            last_diag: Instant::now(),
            recording: None,
            excluir: None,
            rec_name: "movimento".into(),
            files: Vec::new(),
            loaded: None,
            play: None,
            looping: false,
            alvo_manual: false,
            editando_junta: None,
            editando_texto: String::new(),
            focar_angulo: false,
        };
        app.refresh_ports();
        app.refresh_files();
        app
    }

    /// Porta que parece placa: ACM/USB no Linux, usbmodem/wchusbserial no
    /// macOS e COM no Windows (Bluetooth também aparece como COM; o usuário
    /// escolhe a placa).
    fn porta_de_placa(nome: &str) -> bool {
        if cfg!(target_os = "windows") {
            nome.starts_with("COM")
        } else if cfg!(target_os = "macos") {
            nome.contains("usbmodem")
                || nome.contains("usbserial")
                || nome.contains("wchusbserial")
                || nome.contains("USB")
        } else {
            nome.contains("ACM") || nome.contains("USB")
        }
    }

    /// Varre as portas seriais e mantém só as que parecem placa: o filtro
    /// esconde Bluetooth e portas virtuais que só poluem o combo.
    /// Se o índice selecionado deixar de existir, volta para a primeira.
    fn refresh_ports(&mut self) {
        self.ports = serialport::available_ports()
            .unwrap_or_default()
            .into_iter()
            .map(|p| p.port_name)
            .filter(|n| Self::porta_de_placa(n))
            .collect();
        self.ports.sort();
        if self.port_idx >= self.ports.len() {
            self.port_idx = 0;
        }
    }

    /// Relê a pasta de movimentos. Chamado ao abrir, ao salvar, ao excluir e no
    /// botão de atualizar da lista.
    fn refresh_files(&mut self) {
        self.files = Recording::list(&dir_movimentos());
    }

    /// Acrescenta uma linha ao log, descartando a mais antiga acima de 400.
    fn loga(&mut self, s: impl Into<String>) {
        self.log.push(s.into());
        if self.log.len() > 400 {
            self.log.remove(0);
        }
    }

    /// Registra a falha no log **e** no banner de erro (que fica até ser
    /// fechado). Use para o que o usuário precisa ver sem abrir o log.
    fn falha(&mut self, s: impl Into<String>) {
        let s = s.into();
        self.loga(s.clone());
        self.erro = Some(s);
    }

    /// Abre a porta selecionada. Não assume o controle: isso é ação explícita
    /// do usuário em [`Self::assume`].
    ///
    /// O `last_diag` recuado força um `#B` já no primeiro tick, para a UI não
    /// ficar 2 s com versão e limites "?".
    fn connect(&mut self) {
        let Some(path) = self.ports.get(self.port_idx).cloned() else { return };
        match Link::open(&path) {
            Ok(l) => {
                self.loga(format!("conectado em {path}"));
                self.erro = None;
                self.link = Some(l);
                self.boots = None;
                self.reboot = None;
                self.last_diag = Instant::now() - Duration::from_secs(10);
            }
            Err(e) => self.falha(format!("Não foi possível abrir {path}: {e}")),
        }
    }

    /// Fecha a porta devolvendo o controle ao joystick antes de soltar o handle.
    ///
    /// Tudo que depende do link é zerado; a pose mostrada continua a última
    /// leitura (não inventamos posição).
    fn disconnect(&mut self) {
        if let Some(mut l) = self.link.take() {
            l.send(protocol::CMD_RELEASE);
        }
        self.pc = false;
        self.play = None;
        self.alvo_manual = false;
        self.recording = None;
        self.boots = None;
        self.reboot = None;
        self.reset_targets();
    }

    /// Envia um comando se houver link; sem conexão é no-op silencioso.
    fn send(&mut self, cmd: &str) {
        if let Some(l) = self.link.as_mut() {
            l.send(cmd);
        }
    }

    /// Toma o controle do PC: `#M` + alvos iguais à pose atual, para o firmware
    /// adotar os alvos sem tranco.
    ///
    /// `pc` é otimista aqui, mas o próximo [`protocol::Status`] confirma ou
    /// desfaz — o firmware continua sendo a fonte da verdade.
    fn assume(&mut self) {
        self.reset_targets();
        self.send(protocol::CMD_ASSUME);
        self.pc = true;
    }

    /// STOP: segura as juntas (`#H`), encerra replay/alvo manual e devolve o
    /// controle ao joystick.
    ///
    /// Funciona também desconectado: só registra que não havia placa, para o
    /// usuário não achar que parou algo que estava rodando.
    fn stop(&mut self) {
        let conectado = self.link.is_some();
        self.send(protocol::CMD_STOP);
        self.pc = false;
        self.play = None;
        self.alvo_manual = false;
        self.reset_targets();
        if conectado {
            self.loga("STOP — movimento parado, controle de volta no joystick");
        } else {
            self.loga("STOP local — nenhuma placa conectada");
        }
    }

    /// Um passo do laço: leitura da serial, heartbeat de diagnóstico, gravação,
    /// replay e envio de alvos.
    ///
    /// A ordem importa:
    /// 1. Drena eventos. Erro de serial derruba o link e zera o estado de
    ///    controle — nunca tentamos escrever num handle morto.
    /// 2. Aplica mensagens: `Text` (banner/versão), `Diag` (limites/boots) e
    ///    `Status` (pose, trava e quem está no controle — o firmware vence).
    /// 3. Pede `#B` a cada 2 s, para detectar reinício da placa.
    /// 4. Avança o replay ou o encerra (com loop opcional).
    /// 5. Toque em qualquer pad cancela replay e alvo manual.
    /// 6. Se o PC está no controle, envia `#P` (replay/alvo manual) ou `#V`
    ///    (sticks) no máximo a cada [`TICK`].
    fn tick(&mut self, now: Instant) {
        let mut events = Vec::new();
        if let Some(l) = self.link.as_ref() {
            if !l.drain(&mut events) {
                self.falha("A conexão serial foi perdida. Reconecte a placa.");
                self.link = None;
                self.pc = false;
                self.reset_targets();
            }
        }

        for ev in events {
            match ev {
                Event::Error(e) => self.falha(format!("Erro na serial: {e}")),
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
                        self.reset_targets();
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

        // Toque num pad cancela o replay e o alvo manual: quem está com a mão manda.
        if self.pad_l.stick != egui::Vec2::ZERO || self.pad_r.stick != egui::Vec2::ZERO {
            if self.play.is_some() {
                self.play = None;
                self.loga("replay cancelado (toque no pad)");
            }
            if self.alvo_manual {
                self.alvo_manual = false;
                self.loga("alvo manual cancelado (toque no pad)");
            }
        }

        if self.link.is_some() && self.pc && now - self.last_send >= TICK {
            self.last_send = now;
            let cmd = if self.play.is_some() || self.alvo_manual {
                protocol::cmd_targets(self.pose)
            } else {
                protocol::cmd_sticks([
                    (self.pad_l.stick.x * 100.0) as i32,
                    (-self.pad_l.stick.y * 100.0) as i32, // tela: y para baixo; stick: y para cima
                    (self.pad_r.stick.x * 100.0) as i32,
                    (-self.pad_r.stick.y * 100.0) as i32,
                ])
            };
            self.send(&cmd);
        }
    }

    /// Monta a topbar. Em uma linha (telas largas) os três grupos ficam lado a
    /// lado; estreito, vira duas linhas com o STOP sempre visível à direita.
    ///
    /// Os avisos de erro e de reinício são desenhados logo abaixo, fora dos
    /// grupos, porque precisam de largura total.
    fn top_bar(&mut self, ui: &mut egui::Ui, compacta: bool) {
        if topbar_em_linha_unica(ui.available_width()) {
            ui.horizontal(|ui| {
                self.conexao_bar(ui);
                ui.separator();
                self.controle_bar(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.seguranca_bar(ui, !compacta, self.recording.is_some());
                });
            });
        } else {
            ui.horizontal(|ui| {
                self.conexao_bar(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    self.seguranca_bar(ui, false, false);
                });
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                self.controle_bar(ui);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if self.recording.is_some() {
                        pill(ui, "Gravando", COR_REC, COR_ERRO_FUNDO);
                    }
                    if !compacta {
                        self.diagnostico_bar(ui);
                    }
                });
            });
        }

        if let Some(erro) = self.erro.clone() {
            ui.add_space(4.0);
            egui::Frame::NONE
                .fill(COR_ERRO_FUNDO)
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(12, 6))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if botao_quadrado(ui, Icone::Excluir, ALVO)
                                .on_hover_text("fechar aviso")
                                .clicked()
                            {
                                self.erro = None;
                            }
                        });
                        ui.label(egui::RichText::new(erro).color(COR_ERRO));
                    });
                });
        }
        if let Some(w) = &self.reboot {
            ui.add_space(4.0);
            ui.colored_label(COR_ERRO, format!("Alimentação: {w}"));
        }
    }

    /// Grupo de conexão: atualizar portas, escolher a porta e conectar/desconectar.
    ///
    /// Sem porta selecionada o botão fica desabilitado com explicação, em vez de
    /// falhar ao clicar.
    fn conexao_bar(&mut self, ui: &mut egui::Ui) {
        if botao_quadrado(ui, Icone::Atualizar, ALVO)
            .on_hover_text("atualizar portas seriais")
            .clicked()
        {
            self.refresh_ports();
        }
        let porta = self
            .ports
            .get(self.port_idx)
            .map(|p| texto_curto(p, 18))
            .unwrap_or_else(|| "nenhuma porta".into());
        egui::ComboBox::from_id_salt("porta")
            .width(140.0)
            .selected_text(porta)
            .show_ui(ui, |ui| {
                if self.ports.is_empty() {
                    ui.label("Conecte a placa e atualize a lista");
                }
                for (i, p) in self.ports.iter().enumerate() {
                    ui.selectable_value(&mut self.port_idx, i, p);
                }
            });

        if self.link.is_none() {
            let tem_porta = self.ports.get(self.port_idx).is_some();
            let resposta = ui
                .add_enabled_ui(tem_porta, |ui| botao(ui, Icone::Conectar, "Conectar", 124.0))
                .inner
                .on_hover_text(if tem_porta {
                    "abrir a porta serial selecionada"
                } else {
                    "conecte a placa e atualize a lista de portas"
                });
            if resposta.clicked() {
                self.connect();
            }
        } else if botao(ui, Icone::Desconectar, "Desconectar", 138.0).clicked() {
            self.disconnect();
        }
    }

    /// Grupo de controle: assumir/devolver o comando e travar/liberar a garra.
    ///
    /// O estado exibido vem do firmware (`pc`/`garra_travada`), não do último
    /// clique; se a placa discordar, o botão acompanha a placa no próximo status.
    fn controle_bar(&mut self, ui: &mut egui::Ui) {
        if self.pc {
            if botao(ui, Icone::Joystick, "Devolver ao joystick", 200.0).clicked() {
                self.send(protocol::CMD_RELEASE);
                self.pc = false;
                self.alvo_manual = false;
                self.reset_targets();
            }
        } else {
            let resposta = ui
                .add_enabled_ui(self.link.is_some(), |ui| {
                    botao(ui, Icone::Monitor, "Assumir controle", 184.0)
                })
                .inner;
            if resposta.clicked() {
                self.assume();
            }
        }

        let (icone, rotulo, fundo, frente) = if self.garra_travada {
            (Icone::Liberar, "Liberar garra", COR_OK_FUNDO, COR_OK)
        } else {
            (
                Icone::Travar,
                "Travar garra",
                cor(0x1e, 0x2a, 0x3d),
                cor(0x9e, 0xcb, 0xf5),
            )
        };
        let resposta = ui
            .add_enabled_ui(self.link.is_some(), |ui| {
                icons::botao_cor(ui, icone, rotulo, 152.0, fundo, frente)
            })
            .inner;
        if resposta
            .on_hover_text("mantém pressão contínua no objeto e bloqueia a abertura")
            .clicked()
        {
            let travar = !self.garra_travada;
            self.send(protocol::cmd_lock(travar));
            self.garra_travada = travar;
        }
    }

    /// Grupo de segurança: STOP (sempre habilitado), indicador de quem controla
    /// e, em telas largas, o diagnóstico.
    ///
    /// `diagnostico` só é `true` quando há espaço; com dois "Gravando" na tela
    /// (topbar e painel), a duplicata some no modo estreito.
    fn seguranca_bar(&mut self, ui: &mut egui::Ui, diagnostico: bool, gravando: bool) {
        if icons::botao_cor(
            ui,
            Icone::Stop,
            "STOP",
            124.0,
            cor(0x62, 0x20, 0x27),
            cor(0xff, 0xc2, 0xc2),
        )
        .on_hover_text(
            "Para o braço onde está e devolve o controle ao joystick.\n\
             Cancela o replay. Depois de ~0,6 s os servos desenergizam\n\
             (economia), então o braço pode ceder com o próprio peso.",
        )
        .clicked()
        {
            self.stop();
        }
        if gravando {
            pill(ui, "Gravando", COR_REC, COR_ERRO_FUNDO);
        }
        let (txt, ct, cf) = if self.link.is_none() {
            ("desconectado", COR_MUTED, COR_FUNDO)
        } else if self.pc {
            ("PC no controle", COR_OK, COR_OK_FUNDO)
        } else {
            ("joystick no controle", COR_WARN, COR_WARN_FUNDO)
        };
        pill(ui, txt, ct, cf);
        if diagnostico {
            self.diagnostico_bar(ui);
        }
    }

    /// Versão do firmware e contador de boots. Silencioso quando desconectado.
    fn diagnostico_bar(&self, ui: &mut egui::Ui) {
        if self.link.is_none() {
            return;
        }
        if let Some(b) = self.boots {
            ui.label(
                egui::RichText::new(format!("boots={b}"))
                    .small()
                    .color(COR_MUTED),
            );
        }
        ui.label(
            egui::RichText::new(format!("fw {}", self.version))
                .small()
                .color(COR_MUTED),
        );
    }

    /// Realinha os alvos com a última pose medida e centraliza os pads.
    ///
    /// Chamado em toda troca de autoridade (assumir, devolver, STOP, queda da
    /// serial) para o app nunca continuar mirando um alvo antigo.
    fn reset_targets(&mut self) {
        self.pose = self.live;
        self.pad_l = PadState::default();
        self.pad_r = PadState::default();
    }

    /// Cabeçalho e pads. `altura_viewport` é medida antes do ScrollArea, para
    /// que a rolagem não confunda o cálculo do modo baixo com espaço infinito.
    ///
    /// Os quatro controles são distribuídos como na bancada:
    ///
    /// | Pad | Eixo X | Eixo Y |
    /// |---|---|---|
    /// | esquerdo | Base | Cotovelo |
    /// | direito | Garra | Ombro |
    ///
    /// `centralizar` (dock lateral) divide a sobra vertical; empilhado, o
    /// conteúdo flui de cima. Clicar num chip de ângulo devolve a junta em
    /// `Option<usize>` para abrir o modal.
    fn sticks(&mut self, ui: &mut egui::Ui, altura_viewport: f32, centralizar: bool) {
        let disponivel = egui::vec2(ui.available_width(), altura_viewport);
        let geo = geometria_pads(disponivel);

        ui.horizontal(|ui| {
            titulo_secao(ui, "Controle por toque");
            if geo.compacto && disponivel.x >= 720.0 {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("·  Arraste para mover, solte para segurar")
                        .small()
                        .color(COR_MUTED),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if self.link.is_none() {
                    pill(ui, "conecte e assuma o controle", COR_MUTED, COR_FUNDO);
                } else if self.pc {
                    pill(ui, "pads habilitados", COR_OK, COR_OK_FUNDO);
                } else {
                    pill(ui, "joystick no controle", COR_WARN, COR_WARN_FUNDO);
                }
            });
        });
        if geo.compacto {
            ui.add_space(6.0);
        } else {
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("Arraste para mover · solte para segurar a posição")
                    .small()
                    .color(COR_MUTED),
            );
            ui.add_space(12.0);
        }

        let conectado = self.link.is_some();
        let habilitado = conectado && self.pc;
        let live = self.live;
        let esquerdo = [
            ("Base", 0, conectado.then_some(live[0])),
            ("Cotovelo", 2, conectado.then_some(live[2])),
        ];
        let direito = [
            ("Garra", 3, conectado.then_some(live[3])),
            ("Ombro", 1, conectado.then_some(live[1])),
        ];

        let mut clicado = None;
        if geo.lado_a_lado {
            if centralizar {
                let sobra_vertical =
                    (altura_viewport - pad_chrome(geo.compacto) - geo.lado).max(0.0) / 2.0;
                ui.add_space(sobra_vertical);
            }
            let largura_total = 2.0 * (geo.lado + PAD_MODULO_EXTRA) + PAD_GAP;
            let sobra = (ui.available_width() - largura_total).max(0.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(sobra / 2.0);
                if let Some(j) = pad(
                    ui,
                    "pad_esq",
                    "BASE · COTOVELO",
                    &mut self.pad_l,
                    geo.lado,
                    geo.compacto,
                    habilitado,
                    esquerdo,
                ) {
                    clicado = Some(j);
                }
                ui.add_space(PAD_GAP);
                if let Some(j) = pad(
                    ui,
                    "pad_dir",
                    "GARRA · OMBRO",
                    &mut self.pad_r,
                    geo.lado,
                    geo.compacto,
                    habilitado,
                    direito,
                ) {
                    clicado = Some(j);
                }
            });
        } else {
            let sobra = (ui.available_width() - geo.lado - PAD_MODULO_EXTRA).max(0.0) / 2.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(sobra);
                if let Some(j) = pad(
                    ui,
                    "pad_esq",
                    "BASE · COTOVELO",
                    &mut self.pad_l,
                    geo.lado,
                    geo.compacto,
                    habilitado,
                    esquerdo,
                ) {
                    clicado = Some(j);
                }
            });
            ui.add_space(PAD_GAP);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(sobra);
                if let Some(j) = pad(
                    ui,
                    "pad_dir",
                    "GARRA · OMBRO",
                    &mut self.pad_r,
                    geo.lado,
                    geo.compacto,
                    habilitado,
                    direito,
                ) {
                    clicado = Some(j);
                }
            });
        }
        if let Some(junta) = clicado {
            self.editando_junta = Some(junta);
            self.editando_texto = if conectado {
                format!("{:.0}", self.live[junta])
            } else {
                String::new()
            };
            self.focar_angulo = true;
        }
    }

    /// Painel lateral de gravação (só no modo dock).
    ///
    /// Um scroll só para o painel inteiro: qualquer estimativa de altura fixa
    /// cortava o bloco de replay quando havia movimento carregado.
    fn recordings_panel(&mut self, ui: &mut egui::Ui, compacta: bool) {
        //  Um scroll só para o painel inteiro: qualquer estimativa de altura
        //  fixa cortava o bloco de replay quando havia movimento carregado.
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| self.recordings_body(ui, compacta));
    }

    /// Conteúdo do painel: gravar, replay e lista de movimentos salvos.
    ///
    /// Gravar exige conexão (não há pose para capturar sem placa) e captura
    /// também o movimento feito pelo joystick — é "ensinar fazendo". Reproduzir
    /// assume o controle do PC antes de iniciar. Excluir pede confirmação na
    /// própria linha e cancela o replay se o arquivo era o carregado.
    fn recordings_body(&mut self, ui: &mut egui::Ui, compacta: bool) {
        let gap_secao = if compacta { 12.0 } else { 20.0 };
        ui.add_space(if compacta { 2.0 } else { 6.0 });
        titulo_secao(ui, "Gravação");
        ui.add_space(6.0);

        if let Some((rec, t0)) = &self.recording {
            pill(
                ui,
                &format!("Gravando · {:.1}s · {} amostras", t0.elapsed().as_secs_f32(), rec.samples.len()),
                COR_REC,
                COR_ERRO_FUNDO,
            );
            ui.add_space(8.0);
            if botao(ui, Icone::Stop, "Parar gravação e salvar", ui.available_width()).clicked() {
                if let Some((mut rec, _)) = self.recording.take() {
                    rec.name = self.rec_name.clone();
                    match rec.save(&dir_movimentos()) {
                        Ok(p) => {
                            self.loga(format!("salvo em {}", p.display()));
                            self.refresh_files();
                        }
                        Err(e) => self.falha(format!("Não foi possível salvar o movimento: {e}")),
                    }
                }
            }
        } else {
            ui.label(egui::RichText::new("Nome do movimento").small().color(COR_MUTED));
            ui.add_space(4.0);
            egui::Frame::NONE
                .fill(cor(0x0d, 0x11, 0x15))
                .stroke(egui::Stroke::new(1.0_f32, cor(0x2c, 0x36, 0x43)))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(12, 0))
                .show(ui, |ui| {
                    ui.set_height(ALVO);
                    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                        ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::TextEdit::singleline(&mut self.rec_name)
                                .frame(false)
                                .vertical_align(egui::Align::Center),
                        );
                    });
                });
            ui.add_space(8.0);
            let resposta = ui
                .add_enabled_ui(self.link.is_some(), |ui| {
                    icons::botao_cor(
                        ui,
                        Icone::Gravar,
                        "Iniciar gravação",
                        ui.available_width(),
                        COR_ERRO_FUNDO,
                        COR_REC,
                    )
                })
                .inner
                .on_hover_text(if self.link.is_some() {
                    "captura a posição a 20 Hz, inclusive pelo joystick"
                } else {
                    "conecte a placa para gravar"
                });
            if resposta.clicked() {
                self.recording = Some((
                    Recording { name: self.rec_name.clone(), samples: Vec::new() },
                    Instant::now(),
                ));
                self.loga("gravação iniciada");
            }
        }

        ui.add_space(gap_secao);
        titulo_secao(ui, "Replay");
        ui.add_space(6.0);
        if let Some(r) = self.loaded.clone() {
            ui.horizontal(|ui| {
                ui.add(egui::Label::new(egui::RichText::new(&r.name).strong()).truncate());
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
            ui.add_space(6.0);
            if icons::toggle(
                ui,
                self.looping,
                Icone::Repetir,
                "Repetir continuamente",
                ui.available_width(),
            )
            .clicked()
            {
                self.looping = !self.looping;
            }
            ui.add_space(6.0);
            let pode = self.link.is_some() && self.play.is_none();
            let resposta = ui
                .add_enabled_ui(pode, |ui| {
                    botao(ui, Icone::Play, "Reproduzir movimento", ui.available_width())
                })
                .inner
                .on_hover_text(if pode {
                    "assume o controle do PC e inicia o replay"
                } else {
                    "conecte a placa e aguarde o replay atual terminar"
                });
            if resposta.clicked() {
                self.assume();
                self.alvo_manual = false;
                self.play = Some((r.clone(), Instant::now()));
                self.loga("reproduzindo…");
            }
            if self.play.is_some() {
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Reproduzindo — toque num pad ou use STOP para interromper")
                        .small()
                        .color(COR_OK),
                );
                ui.add_space(6.0);
                if botao(ui, Icone::Stop, "Parar replay", ui.available_width()).clicked() {
                    self.stop();
                }
            }
        } else {
            ui.label(
                egui::RichText::new("Selecione um movimento abaixo para preparar o replay.")
                    .small()
                    .color(COR_MUTED),
            );
        }

        ui.add_space(gap_secao);
        ui.horizontal(|ui| {
            titulo_secao(ui, "Movimentos salvos");
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if botao_quadrado(ui, Icone::Atualizar, ALVO)
                    .on_hover_text("atualizar lista")
                    .clicked()
                {
                    self.refresh_files();
                }
            });
        });
        ui.add_space(8.0);

        if self.files.is_empty() {
            ui.label(egui::RichText::new("Nenhum movimento salvo").strong().color(COR_MUTED));
            ui.add_space(2.0);
            ui.label(
                egui::RichText::new("Conecte a placa, dê um nome e inicie a gravação.")
                    .small()
                    .color(COR_MUTED),
            );
        }
        let files = self.files.clone();
        for f in files {
            let nome = f
                .file_name()
                .map(|n| n.to_string_lossy().trim_end_matches(".json").to_string())
                .unwrap_or_default();

            if self.excluir.as_ref() == Some(&f) {
                ui.horizontal(|ui| {
                    let largura = (ui.available_width() - 10.0) / 2.0;
                    if icons::botao_cor(
                        ui,
                        Icone::Excluir,
                        "Excluir",
                        largura,
                        cor(0x62, 0x20, 0x27),
                        cor(0xff, 0xc2, 0xc2),
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
                            Err(e) => self.falha(format!("Não foi possível excluir {nome}: {e}")),
                        }
                        self.excluir = None;
                    }
                    if botao(ui, Icone::Stop, "Cancelar", ui.available_width()).clicked() {
                        self.excluir = None;
                    }
                });
                continue;
            }

            ui.horizontal(|ui| {
                let selecionado = self.loaded.as_ref().is_some_and(|l| l.name == nome);
                let largura_nome = (ui.available_width() - ALVO - 10.0).max(60.0);
                let nome_visivel = texto_curto(&nome, 30);
                let resposta = icons::botao_texto(ui, &nome_visivel, selecionado, largura_nome);
                if resposta.on_hover_text("carregar este movimento").clicked() {
                    match Recording::load(&f) {
                        Ok(r) => {
                            self.loaded = Some(r);
                            self.loga(format!("carregado {nome}"));
                        }
                        Err(e) => self.falha(format!("Não foi possível ler {nome}: {e}")),
                    }
                }
                if botao_quadrado(ui, Icone::Excluir, ALVO)
                    .on_hover_text("excluir")
                    .clicked()
                {
                    self.excluir = Some(f.clone());
                }
            });
        }

        ui.add_space(12.0);
        ui.add(
            egui::Label::new(
                egui::RichText::new(dir_movimentos().display().to_string())
                    .small()
                    .color(COR_MUTED),
            )
            .truncate(),
        );
    }

    /// Log recolhível com área de toque confortável.
    ///
    /// Recolhido mostra só a última linha (com o texto encurtado pelo espaço);
    /// aberto vira um scroll de 110 pt que acompanha o fim.
    fn log_panel(&mut self, ui: &mut egui::Ui) {
        let icone = if self.log_aberta {
            Icone::SetaBaixo
        } else {
            Icone::SetaCima
        };
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());
        if resp.clicked() {
            self.log_aberta = !self.log_aberta;
        }
        let p = ui.painter();
        if resp.hovered() {
            p.rect_filled(rect, 6.0, cor(0x1a, 0x20, 0x28));
        }
        icons::desenha(
            p,
            egui::pos2(rect.left() + 14.0, rect.center().y),
            6.5,
            icone,
            COR_MUTED,
        );
        p.text(
            egui::pos2(rect.left() + 30.0, rect.center().y),
            egui::Align2::LEFT_CENTER,
            "LOG",
            egui::FontId::proportional(12.5),
            COR_MUTED,
        );
        if !self.log_aberta {
            if let Some(ultima) = self.log.last() {
                let limite = ((rect.width() - 90.0) / 7.5).max(16.0) as usize;
                p.text(
                    egui::pos2(rect.left() + 72.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    texto_curto(ultima, limite),
                    egui::FontId::monospace(12.0),
                    COR_MUTED,
                );
            }
        }
        if self.log_aberta {
            ui.add_space(4.0);
            egui::ScrollArea::vertical()
                .stick_to_bottom(true)
                .max_height(110.0)
                .show(ui, |ui| {
                    for l in &self.log {
                        ui.monospace(egui::RichText::new(l).small());
                    }
                });
        }
    }

    /// Modal para digitar o ângulo de uma junta.
    ///
    /// Valida a cada frame com [`avaliar_alvo`] para mostrar o motivo do erro
    /// antes de confirmar. Enter ou "Mover" aplica; Esc ou "Cancelar" fecha.
    /// Confirmar assume o controle do PC se preciso e liga o alvo manual — o
    /// `#P` correspondente sai pelo [`Self::tick`], no ritmo global.
    fn editor_angulo_modal(&mut self, ctx: &egui::Context) {
        let Some(junta) = self.editando_junta else { return };
        let nomes = ["Base", "Ombro", "Cotovelo", "Garra"];
        let nome = nomes[junta];
        let conectado = self.link.is_some();
        let lim = self.limits[junta];
        let avaliacao = avaliar_alvo(
            junta,
            &self.editando_texto,
            self.limits,
            self.live,
            conectado,
            self.garra_travada,
        );

        let mut fechar = false;
        let mut confirmar = false;
        egui::Window::new(format!("Definir ângulo · {nome}"))
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(320.0, 0.0))
            .show(ctx, |ui| {
                let atual = if conectado {
                    format!("{:.0}°", self.live[junta])
                } else {
                    "—".into()
                };
                ui.label(
                    egui::RichText::new(format!(
                        "Atual: {atual}   ·   Limite: {:.0}° a {:.0}°",
                        lim[0], lim[1]
                    ))
                    .small()
                    .color(COR_MUTED),
                );
                ui.add_space(8.0);

                egui::Frame::NONE
                    .fill(cor(0x0d, 0x11, 0x15))
                    .stroke(egui::Stroke::new(1.0_f32, COR_ACENTO))
                    .corner_radius(8)
                    .inner_margin(egui::Margin::symmetric(12, 0))
                    .show(ui, |ui| {
                        ui.set_height(ALVO);
                        ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
                            let resp = ui.add_sized(
                                [ui.available_width(), 28.0],
                                egui::TextEdit::singleline(&mut self.editando_texto)
                                    .frame(false)
                                    .font(egui::FontId::proportional(20.0))
                                    .vertical_align(egui::Align::Center),
                            );
                            if self.focar_angulo {
                                resp.request_focus();
                                self.focar_angulo = false;
                            }
                            if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                                confirmar = true;
                            }
                        });
                    });
                ui.add_space(8.0);

                match &avaliacao {
                    Ok(v) => {
                        ui.label(
                            egui::RichText::new(format!("Pronto para mover {nome} para {v}°"))
                                .small()
                                .color(COR_OK),
                        );
                    }
                    Err(msg) => {
                        ui.label(egui::RichText::new(msg).small().color(COR_ERRO));
                    }
                }
                ui.add_space(12.0);

                ui.horizontal(|ui| {
                    let larg = (ui.available_width() - 10.0) / 2.0;
                    if botao(ui, Icone::Stop, "Cancelar", larg).clicked() {
                        fechar = true;
                    }
                    let pode = avaliacao.is_ok();
                    let resp = ui
                        .add_enabled_ui(pode, |ui| botao(ui, Icone::Play, "Mover", ui.available_width()))
                        .inner;
                    if resp.clicked() {
                        confirmar = true;
                    }
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            fechar = true;
        }
        if confirmar {
            if let Ok(alvo) = avaliacao {
                self.pose = self.live;
                self.pose[junta] = alvo as f32;
                self.play = None;
                if !self.pc {
                    self.send(protocol::CMD_ASSUME);
                    self.pc = true;
                }
                self.alvo_manual = true;
                self.loga(format!("alvo manual: {nome} → {alvo}°"));
                fechar = true;
            }
        }
        if fechar {
            self.editando_junta = None;
        }
    }
}

/// Valida o texto do modal para uma junta e devolve o alvo inteiro em graus.
///
/// Aceita vírgula decimal (teclado pt-BR) e exige valor finito. Rejeita
/// desconexão, texto inválido, alvo fora dos limites informados pelo firmware e,
/// com a garra travada, qualquer tentativa de abrir (`alvo > live[3]`). Não fala
/// com a placa: os limites vêm do último diagnóstico, e o firmware continua
/// aplicando a trava de verdade.
fn avaliar_alvo(
    junta: usize,
    texto: &str,
    limits: [[f32; 2]; 4],
    live: [f32; 4],
    conectado: bool,
    garra_travada: bool,
) -> Result<i32, String> {
    if !conectado {
        return Err("Conecte a placa primeiro".into());
    }
    let limpo = texto.trim().replace(',', ".");
    let valor: f32 = limpo
        .parse()
        .ok()
        .filter(|v: &f32| v.is_finite())
        .ok_or_else(|| "Digite um número válido em graus".to_string())?;
    let alvo = valor.round() as i32;
    let [min, max] = limits[junta];
    if (alvo as f32) < min || (alvo as f32) > max {
        return Err(format!("Fora do limite ({min:.0}° a {max:.0}°)"));
    }
    if junta == 3 && garra_travada && (alvo as f32) > live[3] {
        return Err(format!(
            "Garra travada: só pode apertar (≤ {:.0}°)",
            live[3]
        ));
    }
    Ok(alvo)
}

impl eframe::App for App {
    /// Frame da UI: um tick, topbar, log, dock condicional, área central com
    /// scroll e o modal por cima.
    ///
    /// `compacta` usa a altura disponível da janela, não a do monitor, para o
    /// modo baixo funcionar também com a janela redimensionada. O repaint a
    /// cada 20 ms mantém replay e pads suaves mesmo sem evento de entrada.
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.tick(Instant::now());
        let compacta = ctx.available_rect().height() < 560.0;

        egui::TopBottomPanel::top("topo").show(ctx, |ui| {
            ui.add_space(4.0);
            self.top_bar(ui, compacta);
            ui.add_space(4.0);
        });
        egui::TopBottomPanel::bottom("log").show(ctx, |ui| self.log_panel(ui));

        let dock = usa_dock_lateral(ctx.available_rect().width());
        if dock {
            // Largura limitada: um nome longo não pode engolir a área dos pads.
            egui::SidePanel::right("gravacao")
                .resizable(true)
                .default_width(320.0)
                .width_range(300.0..=480.0)
                .show(ctx, |ui| self.recordings_panel(ui, compacta));
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            let altura_viewport = ui.available_height();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    self.sticks(ui, altura_viewport, dock);
                    if !dock {
                        ui.add_space(20.0);
                        ui.separator();
                        ui.add_space(12.0);
                        self.recordings_body(ui, compacta);
                    }
                });
        });

        self.editor_angulo_modal(ctx);
        ctx.request_repaint_after(Duration::from_millis(20));
    }
}

// ---------------------------------------------------------------- paleta --

/// Atalho `const` para `Color32::from_rgb`, para a paleta caber em uma linha.
const fn cor(r: u8, g: u8, b: u8) -> egui::Color32 {
    egui::Color32::from_rgb(r, g, b)
}

/// Fundo padrão dos painéis (o mais escuro da hierarquia de superfícies).
const COR_FUNDO: egui::Color32 = cor(0x18, 0x1d, 0x23);
/// Texto secundário: legendas, unidades, caminhos.
const COR_MUTED: egui::Color32 = cor(0x84, 0x93, 0xa1);
/// Acento: foco de teclado, seleção, borda do pad ativo.
const COR_ACENTO: egui::Color32 = cor(0x7f, 0xb2, 0xe5);
/// Estado bom: PC no controle, garra liberada.
const COR_OK: egui::Color32 = cor(0x8f, 0xd6, 0xa2);
/// Fundo do estado bom (pill verde).
const COR_OK_FUNDO: egui::Color32 = cor(0x1b, 0x2e, 0x23);
/// Estado de atenção: joystick no controle.
const COR_WARN: egui::Color32 = cor(0xd9, 0xc3, 0x76);
/// Fundo do estado de atenção (pill amarela).
const COR_WARN_FUNDO: egui::Color32 = cor(0x2c, 0x28, 0x18);
/// Erro e ações destrutivas (STOP, excluir).
const COR_ERRO: egui::Color32 = cor(0xe2, 0x86, 0x86);
/// Fundo de erro/gravação (pill e banner).
const COR_ERRO_FUNDO: egui::Color32 = cor(0x35, 0x1d, 0x24);
/// Vermelho de gravação, um tom acima do erro para não competir com ele.
const COR_REC: egui::Color32 = cor(0xe0, 0x7b, 0x7b);

/// Tema único do app: superfícies escuras azuladas, cantos de 8 px e alvos de
/// toque de 48 px. Aplicado uma vez, no arranque.
///
/// Mexe em `Visuals` (fills, seleção, estados dos widgets), no espaçamento
/// (`interact_size.y = ALVO`) e nos tamanhos de texto. Não há troca de tema em
/// tempo de execução: a paleta acima é a única.
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
    v.widgets.noninteractive.weak_bg_fill = cor(0x1a, 0x20, 0x28);
    v.widgets.noninteractive.bg_fill = cor(0x1a, 0x20, 0x28);
    v.widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0_f32, cor(0x2a, 0x34, 0x41));

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

    st.text_styles.insert(TextStyle::Body, FontId::new(15.0, FontFamily::Proportional));
    st.text_styles.insert(TextStyle::Button, FontId::new(15.0, FontFamily::Proportional));
    st.text_styles.insert(TextStyle::Heading, FontId::new(20.0, FontFamily::Proportional));
    st.text_styles.insert(TextStyle::Small, FontId::new(12.5, FontFamily::Proportional));
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

/// Leitura clicável de uma junta, com largura fixa para alinhar com o pad.
///
/// Mostra `—` quando desconectado. O anel de hover e o tooltip existem porque o
/// alvo é abrir o modal de ângulo, não só ler o valor.
fn chip(ui: &mut egui::Ui, nome: &str, valor: Option<f32>, largura: f32) -> egui::Response {
    let texto = valor.map(|v| format!("{v:.0}°")).unwrap_or_else(|| "—".into());
    let resp = egui::Frame::NONE
        .fill(cor(0x1c, 0x23, 0x2b))
        .stroke(egui::Stroke::new(1.0_f32, cor(0x2d, 0x38, 0x46)))
        .corner_radius(10)
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.set_width((largura - 24.0).max(24.0));
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(egui::RichText::new(nome.to_uppercase()).small().color(COR_MUTED));
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new(texto)
                        .size(23.0)
                        .strong()
                        .color(cor(0xe8, 0xef, 0xf6)),
                );
            });
        })
        .response
        .interact(egui::Sense::click());
    if resp.hovered() {
        ui.painter().rect_stroke(
            resp.rect,
            10.0,
            egui::Stroke::new(1.5_f32, COR_ACENTO),
            egui::StrokeKind::Inside,
        );
    }
    resp.on_hover_text("toque para digitar o ângulo")
}

/// Rastreia toque dedicado por pad (multi-touch simultâneo) com fallback para mouse/ponteiro.
///
/// O `Sense::click_and_drag` do egui trata todos os dedos como um ponteiro só;
/// para os dois pads funcionarem ao mesmo tempo, cada pad registra o `TouchId`
/// que começou dentro do seu retângulo e ignora os demais. Sem toque ativo, cai
/// para o ponteiro (mouse/teste). Devolve a posição em coordenadas de tela ou
/// `None` quando não há interação.
fn pos_pad(
    ui: &egui::Ui,
    rect: egui::Rect,
    resp: &egui::Response,
    touch: &mut Option<(egui::TouchId, egui::Pos2)>,
) -> Option<egui::Pos2> {
    ui.ctx().input(|i| {
        for ev in &i.events {
            if let egui::Event::Touch { id, phase, pos, .. } = ev {
                match phase {
                    egui::TouchPhase::Start if touch.is_none() && rect.contains(*pos) => {
                        *touch = Some((*id, *pos));
                    }
                    egui::TouchPhase::Move if touch.is_some_and(|(tid, _)| tid == *id) => {
                        *touch = Some((*id, *pos));
                    }
                    egui::TouchPhase::End | egui::TouchPhase::Cancel
                        if touch.is_some_and(|(tid, _)| tid == *id) =>
                    {
                        *touch = None;
                    }
                    _ => {}
                }
            }
        }
    });
    if let Some((_, pos)) = *touch {
        return Some(pos);
    }
    resp.interact_pointer_pos().or_else(|| {
        if resp.dragged() {
            ui.ctx().input(|i| i.pointer.latest_pos())
        } else {
            None
        }
    })
}

/// Pad de toque estilo alavanca: arrastar move o knob, soltar volta ao centro
/// (centro = segurar a posição, igual ao joystick físico solto).
///
/// O vetor do stick é normalizado pelo raio e preso ao círculo unitário, então
/// arrastar além da borda satura em 100% em vez de extrapolar. `habilitado`
/// falso ignora a entrada e centraliza. O layout é sempre círculo + coluna de
/// leituras (`lado + 12 + 116`); `compacto` só aperta os espaçamentos.
///
/// Retorna o índice da junta caso o usuário toque em um dos cards de ângulo.
#[allow(clippy::too_many_arguments)]
fn pad(
    ui: &mut egui::Ui,
    id: &str,
    titulo: &str,
    state: &mut PadState,
    lado: f32,
    compacto: bool,
    habilitado: bool,
    leituras: [(&str, usize, Option<f32>); 2],
) -> Option<usize> {
    const LARG_LEITURA: f32 = 116.0;
    const ESPACO: f32 = 12.0;
    const ALTURA_LEITURAS: f32 = 144.0;
    let mut clicado = None;

    ui.push_id(id, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.set_width(lado + ESPACO + LARG_LEITURA);
            ui.label(egui::RichText::new(titulo).small().strong().color(COR_MUTED));
            ui.add_space(if compacto { 2.0 } else { 4.0 });
            ui.label(
                egui::RichText::new(format!("X · {}   ·   Y · {}", leituras[0].0, leituras[1].0))
                    .small()
                    .color(COR_MUTED),
            );
            ui.add_space(if compacto { 6.0 } else { 8.0 });

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                let (rect, resp) =
                    ui.allocate_exact_size(egui::vec2(lado, lado), egui::Sense::click_and_drag());
                let centro = rect.center();
                let raio = lado * 0.5 - 4.0;

                let pos = if habilitado {
                    pos_pad(ui, rect, &resp, &mut state.touch)
                } else {
                    state.touch = None;
                    None
                };
                match pos {
                    Some(pos) => {
                        let mut d = (pos - centro) / raio;
                        if d.length() > 1.0 {
                            d = d.normalized();
                        }
                        state.stick = d;
                    }
                    None => state.stick = egui::Vec2::ZERO,
                }

                let ativo = habilitado && state.stick != egui::Vec2::ZERO;
                let fundo_pad = if habilitado { cor(0x1c, 0x22, 0x2a) } else { cor(0x17, 0x1c, 0x23) };
                let borda_pad = if ativo {
                    COR_ACENTO
                } else if habilitado {
                    cor(0x32, 0x3e, 0x4c)
                } else {
                    cor(0x27, 0x2f, 0x39)
                };
                let p = ui.painter();
                p.circle_filled(centro, raio, fundo_pad);
                if ativo {
                    p.circle_filled(
                        centro,
                        raio,
                        egui::Color32::from_rgba_unmultiplied(0x7f, 0xb2, 0xe5, 24),
                    );
                }
                p.circle_stroke(
                    centro,
                    raio,
                    egui::Stroke::new(if ativo { 2.5_f32 } else { 1.5_f32 }, borda_pad),
                );
                let linha = egui::Stroke::new(1.0_f32, cor(0x25, 0x2d, 0x37));
                p.line_segment([centro - egui::vec2(raio, 0.0), centro + egui::vec2(raio, 0.0)], linha);
                p.line_segment([centro - egui::vec2(0.0, raio), centro + egui::vec2(0.0, raio)], linha);
                p.circle_stroke(centro, raio * 0.65, egui::Stroke::new(1.0_f32, cor(0x20, 0x27, 0x30)));
                p.circle_stroke(centro, raio * 0.28, egui::Stroke::new(1.0_f32, cor(0x25, 0x2d, 0x37)));

                // Sinais sutis nas pontas dos eixos para orientação imediata ao toque.
                let cor_eixo = if habilitado { cor(0x64, 0x74, 0x84) } else { cor(0x3d, 0x47, 0x52) };
                let fonte_eixo = egui::FontId::proportional(11.0);
                let margem_eixo = (raio - 11.0).max(12.0);
                for (offset, sinal) in [
                    (egui::vec2(-margem_eixo, 0.0), "−"),
                    (egui::vec2(margem_eixo, 0.0), "+"),
                    (egui::vec2(0.0, -margem_eixo), "+"),
                    (egui::vec2(0.0, margem_eixo), "−"),
                ] {
                    p.text(centro + offset, egui::Align2::CENTER_CENTER, sinal, fonte_eixo.clone(), cor_eixo);
                }

                let knob = centro + state.stick * raio;
                if ativo {
                    p.line_segment(
                        [centro, knob],
                        egui::Stroke::new(2.5_f32, egui::Color32::from_rgba_unmultiplied(0x7f, 0xb2, 0xe5, 140)),
                    );
                }
                let raio_knob = (raio * 0.22).clamp(18.0, 36.0);
                let cor_knob = if !habilitado {
                    cor(0x53, 0x61, 0x70)
                } else if ativo {
                    cor(0x9e, 0xcb, 0xf5)
                } else {
                    COR_ACENTO
                };
                if ativo {
                    p.circle_filled(
                        knob,
                        raio_knob * 1.32,
                        egui::Color32::from_rgba_unmultiplied(0x7f, 0xb2, 0xe5, 42),
                    );
                }
                p.circle_filled(knob, raio_knob, cor_knob);
                p.circle_stroke(knob, raio_knob, egui::Stroke::new(2.0_f32, cor(0x0f, 0x14, 0x19)));
                p.circle_filled(knob, raio_knob * 0.28, cor(0x16, 0x1d, 0x25));

                ui.add_space(ESPACO);
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.set_width(LARG_LEITURA);
                    ui.add_space(((lado - ALTURA_LEITURAS) * 0.5).max(0.0));
                    ui.label(egui::RichText::new("ÂNGULOS ATUAIS").small().color(COR_MUTED));
                    ui.add_space(6.0);
                    if chip(ui, leituras[0].0, leituras[0].2, LARG_LEITURA).clicked() {
                        clicado = Some(leituras[0].1);
                    }
                    ui.add_space(8.0);
                    if chip(ui, leituras[1].0, leituras[1].2, LARG_LEITURA).clicked() {
                        clicado = Some(leituras[1].1);
                    }
                });
            });
        });
    });
    clicado
}

#[cfg(test)]
mod ui_layout_tests {
    use super::*;

    #[test]
    fn dock_e_topbar_reagem_a_largura_do_viewport() {
        assert!(usa_dock_lateral(1920.0));
        assert!(usa_dock_lateral(1280.0));
        assert!(usa_dock_lateral(1024.0));
        assert!(!usa_dock_lateral(760.0));
        assert!(!topbar_em_linha_unica(760.0));
        assert!(topbar_em_linha_unica(1024.0));
        assert!(topbar_em_linha_unica(1280.0));
    }

    #[test]
    fn pads_cabem_nas_duas_telas_zenbook_duo_e_no_fallback_estreito() {
        for disponivel in [
            egui::vec2(1560.0, 900.0), // tela cheia 16:10
            egui::vec2(1920.0, 1080.0),
            egui::vec2(1560.0, 390.0), // Zenbook Duo faixa larga e baixa
            egui::vec2(1440.0, 350.0),
            egui::vec2(1280.0, 320.0),
            egui::vec2(940.0, 250.0),
            egui::vec2(740.0, 260.0),  // fallback estreito empilhado
        ] {
            let geo = geometria_pads(disponivel);
            if geo.lado_a_lado {
                let largura = 2.0 * (geo.lado + PAD_MODULO_EXTRA) + PAD_GAP;
                assert!(largura <= disponivel.x + 0.01, "{geo:?} em {disponivel:?}");
                assert!(
                    geo.lado + pad_chrome(geo.compacto) <= disponivel.y + 0.01,
                    "{geo:?} em {disponivel:?}"
                );
            } else {
                assert!(geo.lado + PAD_MODULO_EXTRA <= disponivel.x + 0.01);
            }
        }
    }

    #[test]
    fn texto_curto_respeita_limite_e_unicode() {
        assert_eq!(texto_curto("movimento", 18), "movimento");
        assert_eq!(texto_curto("açãobjörn東京", 8).chars().count(), 8);
        assert_eq!(texto_curto("abc", 3), "abc");
    }

    #[test]
    fn avaliar_alvo_valida_limites_conexao_e_trava_da_garra() {
        let lim = [[5.0, 175.0], [10.0, 170.0], [0.0, 180.0], [0.0, 90.0]];
        let live = [90.0, 90.0, 90.0, 30.0];

        assert_eq!(avaliar_alvo(0, "120", lim, live, true, false), Ok(120));
        assert_eq!(avaliar_alvo(0, " 45,6 ", lim, live, true, false), Ok(46));
        assert!(avaliar_alvo(0, "120", lim, live, false, false).is_err());
        assert!(avaliar_alvo(0, "abc", lim, live, true, false).is_err());
        assert!(avaliar_alvo(0, "180", lim, live, true, false).is_err());
        assert!(avaliar_alvo(1, "5", lim, live, true, false).is_err());

        // Garra travada: só pode apertar (<= live[3]), nunca abrir (> live[3])
        assert_eq!(avaliar_alvo(3, "20", lim, live, true, true), Ok(20));
        assert!(avaliar_alvo(3, "45", lim, live, true, true).is_err());
    }
}
