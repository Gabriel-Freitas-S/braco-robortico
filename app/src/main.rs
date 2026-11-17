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
//! | [`firmware`] | sketch embutido, versão e gravação para upload |
//! | [`update`] | checagem das versões publicadas (app e firmware) |
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
//! - STOP (`#H`, tecla Espaço) está sempre visível e devolve o controle ao
//!   joystick; o firmware cancela um retorno à Posição 0 em andamento.
//! - Tocar num pad ou usar uma tecla de movimento cancela replay e alvo manual:
//!   quem está com a mão (ou no teclado) manda.
//! - A trava da garra é um único estado no firmware (botão D4 e app), e com ela
//!   ativa a garra só aperta.
//! - Os limites usados na validação da UI vêm do diagnóstico; quem os aplica de
//!   verdade é o firmware, então um alvo fora da faixa não vira movimento.
//! - Queda de link ou reinício da placa encerra a gravação, salva o que foi
//!   capturado e derruba a autoridade do PC; o usuário precisa assumir de novo.
//!
//! # Teclado
//!
//! WASD controla o pad esquerdo (Base/Cotovelo) e as setas o direito
//! (Garra/Ombro), pelo mesmo stick virtual `#V` dos pads; Shift deixa fino.
//! `M`/`J`/`Espaço`/`G`/`R`/`P`/`L`/`1`/`2`/`3` são ações e `H`/`F1` abrem a
//! lista de atalhos. Movimento exige o PC no controle; campo de texto, modal ou
//! popup bloqueiam o teclado. O mapa completo fica em [`App::processa_teclado`].
//!
//! # Log
//!
//! O painel inferior guarda os últimos 400 eventos, com cor para erros/STOP.
//! Aberto, permite copiar tudo, limpar e selecionar linhas. Ver
//! [`App::log_panel`].
//!
//! # Firmware e atualizações
//!
//! O sketch do firmware vai embutido no binário ([`firmware`]); o modal
//! **Firmware e atualizações** mostra a versão embutida, a que a placa reporta
//! em `#B` e a publicada no repositório. A checagem roda uma vez no boot em
//! thread separada ([`update`]) e nunca bloqueia nem impede o uso offline.
//! Baixar o firmware só grava o `.ino` na pasta do usuário; gravar na placa
//! continua sendo do Arduino IDE/arduino-cli. O app não se auto-atualiza.
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
//!
//! # Licença
//!
//! Código sob MIT (ver `LICENSE`). Modelos 3D, imagens e documentação sob
//! CC BY 4.0 (ver `LICENSE-CC-BY-4.0.txt`). Projeto acadêmico da disciplina de
//! Sistemas Embarcados do curso de Engenharia da Computação.

mod firmware;
mod icons;
mod link;
mod motion;
mod protocol;
mod update;

use icons::{botao, botao_quadrado, Icone};
use link::{Event, Link};
use motion::{PlayState, Recording, VELOCIDADES_REPLAY};
use protocol::Msg;
use std::cmp::Ordering;
use std::path::PathBuf;
use std::sync::mpsc::TryRecvError;
use std::time::{Duration, Instant};

/// Diretório de dados do usuário (movimentos e firmware).
///
/// Linux/macOS: respeita `XDG_DATA_HOME`; sem ele, usa `$HOME/.local/share`.
/// Windows: `%APPDATA%\robocontrol`.
fn dir_dados() -> PathBuf {
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        return PathBuf::from(xdg).join("robocontrol");
    }
    #[cfg(target_os = "windows")]
    let base = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."));
    #[cfg(not(target_os = "windows"))]
    let base = PathBuf::from(std::env::var("HOME").unwrap_or_default()).join(".local/share");
    base.join("robocontrol")
}

/// Pasta dos movimentos gravados.
fn dir_movimentos() -> PathBuf {
    dir_dados().join("movimentos")
}

/// Período do laço de controle: 50 ms = 20 Hz, a mesma taxa do status do
/// firmware. Também é o intervalo mínimo entre dois comandos de alvo.
const TICK: Duration = Duration::from_millis(50);

/// Alvo mínimo de toque dos controles principais, em pontos lógicos.
///
/// 48 pt é o menor valor confortável em tela touch (e o que o tema aplica em
/// `interact_size.y`). O log usa botões compactos de 26 pt por ser secundário.
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

/// `true` só na primeira pressão da tecla, ignorando o auto-repeat do SO.
///
/// `InputState::key_pressed` conta repetições; usar a versão direta faria a
/// lista de atalhos piscar enquanto H fica segurado.
fn tecla_pressionada(ctx: &egui::Context, tecla: egui::Key) -> bool {
    ctx.input(|i| {
        i.events.iter().any(|e| {
            matches!(
                e,
                egui::Event::Key { key, pressed: true, repeat: false, .. } if *key == tecla
            )
        })
    })
}

/// Eixo digital de um par de teclas: -1, 0 ou +1; `fino` reduz para ±0.5.
///
/// As duas teclas juntas se anulam (0) para o braço parar em vez de escolher
/// um lado por acaso.
fn stick_digital(negativo: bool, positivo: bool, fino: bool) -> f32 {
    let v = match (negativo, positivo) {
        (true, false) => -1.0,
        (false, true) => 1.0,
        _ => 0.0,
    };
    if fino { v * 0.5 } else { v }
}

/// Abre a janela, instala o tema e dispara a checagem de versões.
///
/// O tamanho inicial é fixo (1180×560) e `persist_window` fica desligado de
/// propósito: se o egui lembrasse a última sessão, o app abriria maximizado
/// sozinho depois de um fechamento em tela cheia.
///
/// A checagem de versões começa aqui, e não em [`App::new`], para os testes
/// continuarem sem rede.
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
            let mut app = App::new();
            app.inicia_check();
            Ok(Box::new(app))
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
    /// Stick sintetizado pelo teclado (mesma convenção de `PadState::stick`:
    /// `-1..=1`, Y para baixo). O send usa o toque quando ativo; senão, este.
    tecla_l: egui::Vec2,
    tecla_r: egui::Vec2,
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
    /// Instante do último `Status` (ou do último `#M`). Em modo PC o firmware
    /// manda status a 20 Hz; 1 s sem nenhum significa placa travada ou sem
    /// resposta — o timeout de 500 ms sozinho é revivido pelo próximo `#P`.
    last_status: Option<Instant>,
    /// Instante do último `tick` para medir `dt_ms` real do replay.
    last_tick: Instant,
    /// Janela curta após `assume()` para não descartar `pc` por um `Status` antigo em trânsito.
    last_assume: Option<Instant>,

    /// Gravação em andamento + instante inicial (base dos carimbos em ms).
    recording: Option<(Recording, Instant)>,
    /// Movimento aguardando confirmação de exclusão na lista.
    excluir: Option<PathBuf>,
    /// Nome que será dado ao próximo movimento salvo.
    rec_name: String,
    /// `true` enquanto o campo de nome do movimento tem foco de texto.
    /// `wants_keyboard_input` do egui fica verdadeiro para qualquer widget
    /// focado (inclusive o combo de portas), então o teclado usa este campo.
    nome_em_foco: bool,
    /// Caminhos dos movimentos salvos, em ordem, atualizados por `refresh_files`.
    files: Vec<PathBuf>,
    /// Movimento carregado para replay (não é o mesmo que estar reproduzindo).
    loaded: Option<Recording>,
    /// Caminho do arquivo carregado. A lista usa o nome sanitizado do arquivo,
    /// que não bate com `Recording::name` quando há espaço ou pontuação; o
    /// caminho é a identidade estável para destaque e exclusão.
    loaded_path: Option<PathBuf>,
    /// Replay em andamento (com pré-posicionamento, pausa e posição na linha do tempo).
    play: Option<PlayState>,
    /// Fator de velocidade selecionado para o replay (`0.5`, `1.0`, `2.0`).
    vel_replay: f32,
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
    /// Popup com a lista de atalhos do teclado.
    atalhos_abertos: bool,
    /// Popup "Firmware e atualizações".
    firmware_aberto: bool,
    /// Último resultado da checagem de versões; `None` antes da primeira.
    novidades: Option<update::Novidades>,
    /// Checagem em voo (a thread entrega uma mensagem e termina).
    checando: Option<std::sync::mpsc::Receiver<update::Novidades>>,
    /// Caminho do último sketch salvo, mostrado no modal.
    sketch_salvo: Option<PathBuf>,
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
            tecla_l: egui::Vec2::ZERO,
            tecla_r: egui::Vec2::ZERO,
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
            last_status: None,
            last_tick: Instant::now(),
            last_assume: None,
            recording: None,
            excluir: None,
            rec_name: "movimento".into(),
            nome_em_foco: false,
            files: Vec::new(),
            loaded: None,
            loaded_path: None,
            play: None,
            vel_replay: 1.0,
            looping: false,
            alvo_manual: false,
            editando_junta: None,
            editando_texto: String::new(),
            focar_angulo: false,
            atalhos_abertos: false,
            firmware_aberto: false,
            novidades: None,
            checando: None,
            sketch_salvo: None,
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

    /// Dispara a checagem de versões; uma por vez, sem bloquear a UI.
    fn inicia_check(&mut self) {
        if self.checando.is_some() {
            return;
        }
        self.loga("verificando versões publicadas…");
        self.checando = Some(update::checar());
    }

    /// Guarda o resultado da checagem e registra no log.
    ///
    /// Nunca usa [`Self::falha`]: ficar sem rede é o caso normal, não pode
    /// virar banner vermelho nem atrapalhar quem está com o braço na mão.
    fn aplica_novidades(&mut self, n: update::Novidades) {
        if let Some(app) = n.app.as_deref() {
            if firmware::compara(app, update::versao_app()) == Some(Ordering::Greater) {
                self.loga(format!("app {app} disponível — abra Firmware"));
            } else {
                self.loga(format!("app {} é o mais recente", update::versao_app()));
            }
        }
        if let Some(fw) = n.firmware.as_deref() {
            if firmware::compara(fw, firmware::versao()) == Some(Ordering::Greater) {
                self.loga(format!(
                    "firmware {fw} disponível (embutido {})",
                    firmware::versao()
                ));
            } else {
                self.loga(format!(
                    "firmware embutido é o mais recente ({})",
                    firmware::versao()
                ));
            }
        }
        if let Some(e) = n.erro.as_deref() {
            self.loga(format!("verificação de versões: {e}"));
        }
        self.novidades = Some(n);
    }

    /// `true` quando há app ou firmware mais novo que o embutido.
    fn tem_novidade(&self) -> bool {
        let Some(n) = &self.novidades else { return false };
        let mais_novo = |disponivel: Option<&str>, atual: &str| {
            disponivel.and_then(|v| firmware::compara(v, atual)) == Some(Ordering::Greater)
        };
        mais_novo(n.app.as_deref(), update::versao_app())
            || mais_novo(n.firmware.as_deref(), firmware::versao())
    }

    /// Grava o sketch mais novo disponível (baixado ou embutido) na pasta do
    /// usuário, pronto para abrir no Arduino IDE.
    fn salvar_sketch(&mut self) {
        let baixado = self.novidades.as_ref().and_then(|n| {
            let versao = n.firmware.as_deref()?;
            let sketch = n.sketch.as_ref()?;
            (firmware::compara(versao, firmware::versao()) == Some(Ordering::Greater))
                .then(|| (sketch.clone(), versao.to_string()))
        });
        let (texto, versao) = match baixado {
            Some((texto, versao)) => (texto, versao),
            None => (firmware::SKETCH.to_string(), firmware::versao().to_string()),
        };
        //  O texto pode ter vindo da rede: valida de novo antes de gravar.
        if firmware::validar(&texto).is_none() {
            self.falha("sketch inválido — nada foi salvo");
            return;
        }
        match firmware::salvar(&dir_dados().join("firmware"), &texto) {
            Ok(p) => {
                self.loga(format!("firmware {versao} salvo em {}", p.display()));
                self.sketch_salvo = Some(p);
            }
            Err(e) => self.falha(format!("Não foi possível salvar o firmware: {e}")),
        }
    }

    /// Encerra a gravação em andamento, corta o silêncio das pontas e salva.
    ///
    /// Usado pelo botão "Parar gravação" e por qualquer queda de link ou
    /// reinício da placa: sem isso, a UI continuaria "gravando" sem receber
    /// amostras. `permitir_sobrescrever` mantém o comportamento do botão de
    /// regravar o mesmo nome; nos teardowns automáticos, um nome já ocupado
    /// ganha sufixo `_2`, `_3`… para não apagar outro movimento. A gravação é
    /// consumida mesmo se o save falhar (o erro vai para o banner).
    fn finalizar_gravacao(&mut self, motivo: &str, permitir_sobrescrever: bool) {
        let Some((mut rec, _)) = self.recording.take() else { return };
        rec.name = self.rec_name.clone();
        rec.trim_idle();
        if rec.samples.is_empty() {
            self.loga(format!("gravação vazia descartada ({motivo})"));
            return;
        }
        let dir = dir_movimentos();
        let mut path = Recording::path_for(&dir, &rec.name);
        if !permitir_sobrescrever && path.exists() && self.loaded_path.as_ref() != Some(&path) {
            let base = rec.name.clone();
            let mut n = 2;
            loop {
                rec.name = format!("{base}_{n}");
                path = Recording::path_for(&dir, &rec.name);
                if !path.exists() {
                    break;
                }
                n += 1;
            }
        }
        match rec.save(&dir) {
            Ok(p) => {
                self.loga(format!(
                    "salvo em {} ({:.1}s, {} amostras) — {motivo}",
                    p.display(),
                    rec.duration_ms() as f32 / 1000.0,
                    rec.samples.len()
                ));
                self.refresh_files();
                self.loaded = Some(rec);
                self.loaded_path = Some(p);
            }
            Err(e) => self.falha(format!("Não foi possível salvar o movimento: {e}")),
        }
    }

    /// Compara o contador de boots e trata um reinício como perda de autoridade.
    ///
    /// O banner de boot chega antes do `loop()` do firmware começar, então ele
    /// fecha a janela em que o app ainda mandaria `#P` para uma placa recém-
    /// reiniciada. O diagnóstico (a cada 2 s) é o fallback se o banner se perder.
    fn registra_boots(&mut self, boots: u32) {
        if let Some(antes) = self.boots {
            //  Qualquer mudança é reinício: um reset a frio perde o `.noinit`
            //  e o contador volta para 1, então `>` sozinho não pegaria.
            if boots != antes && antes > 0 {
                self.reboot = Some(format!(
                    "a placa reiniciou (boots {antes} → {boots}) — suspeite da alimentação"
                ));
                self.finalizar_gravacao("placa reiniciou", false);
                self.solta_controle();
                self.loga("placa reiniciou — assuma o controle de novo");
            }
        }
        self.boots = Some(boots);
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
                self.last_assume = None;
                self.last_status = None;
                self.last_send = Instant::now() - TICK;
                self.last_diag = Instant::now() - Duration::from_secs(10);
            }
            Err(e) => self.falha(format!("Não foi possível abrir {path}: {e}")),
        }
    }

    /// Fecha a porta devolvendo o controle ao joystick antes de soltar o handle.
    ///
    /// Tudo que depende do link é zerado; a pose mostrada continua a última
    /// leitura (não inventamos posição). Uma gravação em andamento é encerrada
    /// e salva, para a captura não se perder com a porta.
    fn disconnect(&mut self) {
        if let Some(mut l) = self.link.take() {
            l.send(protocol::CMD_RELEASE);
        }
        self.finalizar_gravacao("desconectado", false);
        self.solta_controle();
        self.boots = None;
        self.reboot = None;
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
        self.last_assume = Some(Instant::now());
        self.last_status = Some(Instant::now());
        self.last_send = Instant::now() - TICK;
    }

    /// Devolve o controle ao joystick e congela onde o braço está (`#J`).
    fn devolve_ao_joystick(&mut self) {
        self.send(protocol::CMD_RELEASE);
        self.solta_controle();
    }

    /// Alterna a trava de pressão da garra (a mesma do botão físico D4).
    fn alterna_trava_garra(&mut self) {
        let travar = !self.garra_travada;
        self.send(protocol::cmd_lock(travar));
        self.garra_travada = travar;
    }

    /// Começa uma nova gravação com o nome atual.
    fn inicia_gravacao(&mut self) {
        self.recording = Some((
            Recording { name: self.rec_name.clone(), samples: Vec::new() },
            Instant::now(),
        ));
        self.loga("gravação iniciada");
    }

    /// Assume o PC e começa o replay de `rec` (se houver placa).
    fn inicia_replay(&mut self, rec: Recording) {
        if self.link.is_none() {
            return;
        }
        self.assume();
        self.alvo_manual = false;
        self.play = Some(PlayState::new(rec));
        self.loga("reproduzindo…");
    }

    /// STOP: segura as juntas (`#H`), encerra replay/alvo manual e devolve o
    /// controle ao joystick.
    ///
    /// Funciona também desconectado: só registra que não havia placa, para o
    /// usuário não achar que parou algo que estava rodando.
    fn stop(&mut self) {
        let conectado = self.link.is_some();
        self.send(protocol::CMD_STOP);
        self.solta_controle();
        if conectado {
            self.loga("STOP — movimento parado, controle de volta no joystick");
        } else {
            self.loga("STOP local — nenhuma placa conectada");
        }
    }

    /// Traduz o teclado nos mesmos comandos dos botões e pads.
    ///
    /// Só age sem campo de texto (ou combo) focado: digitar um nome de movimento
    /// não pode assumir o controle nem mover o braço. Espaço (STOP) continua
    /// valendo com a lista de atalhos aberta; os demais atalhos ficam desligados
    /// enquanto ela, o modal de firmware ou o de ângulo estiverem na frente.
    fn processa_teclado(&mut self, ctx: &egui::Context) {
        //  `wants_keyboard_input` fica verdadeiro para qualquer widget focado
        //  (inclusive o combo de portas), então o bloqueio de texto é explícito:
        //  campo de nome ou modal de ângulo.
        let digitando = self.nome_em_foco || self.editando_junta.is_some();
        let popup = ctx.is_popup_open();

        //  H/F1 alterna a lista. Com campo de texto focado, H é uma letra; com
        //  o modal de firmware aberto, quem fecha é o Esc/× do próprio modal.
        if !digitando && !popup && !self.firmware_aberto
            && (tecla_pressionada(ctx, egui::Key::H)
                || tecla_pressionada(ctx, egui::Key::F1))
        {
            self.atalhos_abertos = !self.atalhos_abertos;
        }

        //  STOP é a exceção: vale mesmo com a ajuda aberta; só um campo de
        //  texto (onde espaço é uma letra) o bloqueia.
        if !digitando && tecla_pressionada(ctx, egui::Key::Space) {
            self.stop();
        }

        if self.atalhos_abertos || self.firmware_aberto || digitando || popup {
            self.tecla_l = egui::Vec2::ZERO;
            self.tecla_r = egui::Vec2::ZERO;
            return;
        }

        if self.link.is_some() && !self.pc && tecla_pressionada(ctx, egui::Key::M) {
            self.assume();
        }
        if self.pc && tecla_pressionada(ctx, egui::Key::J) {
            self.devolve_ao_joystick();
        }
        if self.link.is_some() && tecla_pressionada(ctx, egui::Key::G) {
            self.alterna_trava_garra();
        }
        if self.link.is_some() && tecla_pressionada(ctx, egui::Key::R) {
            if self.recording.is_some() {
                self.finalizar_gravacao("atalho R", true);
            } else {
                self.inicia_gravacao();
            }
        }
        if tecla_pressionada(ctx, egui::Key::P) {
            if self.play.is_some() {
                let pausado = self
                    .play
                    .as_mut()
                    .map(|p| {
                        p.pausado = !p.pausado;
                        p.pausado
                    })
                    .unwrap_or(false);
                self.loga(if pausado { "replay pausado" } else { "replay retomado" });
            } else if let Some(r) = self.loaded.clone() {
                self.inicia_replay(r);
            }
        }
        if self.loaded.is_some() && tecla_pressionada(ctx, egui::Key::L) {
            self.looping = !self.looping;
            self.loga(if self.looping { "repetir ligado" } else { "repetir desligado" });
        }
        for (tecla, vel) in [
            (egui::Key::Num1, 0.5),
            (egui::Key::Num2, 1.0),
            (egui::Key::Num3, 2.0),
        ] {
            if tecla_pressionada(ctx, tecla) {
                self.vel_replay = vel;
            }
        }

        let fino = ctx.input(|i| i.modifiers.shift);
        let (a, d, w, s, esq, dir, cima, baixo) = ctx.input(|i| {
            (
                i.key_down(egui::Key::A),
                i.key_down(egui::Key::D),
                i.key_down(egui::Key::W),
                i.key_down(egui::Key::S),
                i.key_down(egui::Key::ArrowLeft),
                i.key_down(egui::Key::ArrowRight),
                i.key_down(egui::Key::ArrowUp),
                i.key_down(egui::Key::ArrowDown),
            )
        });
        self.tecla_l = egui::vec2(stick_digital(a, d, fino), stick_digital(w, s, fino));
        self.tecla_r = egui::vec2(stick_digital(esq, dir, fino), stick_digital(cima, baixo, fino));
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
    /// 5. Toque num pad ou tecla de movimento cancela replay e alvo manual.
    /// 6. Se o PC está no controle, envia `#P` (replay/alvo manual) ou `#V`
    ///    (sticks) no máximo a cada [`TICK`].
    fn tick(&mut self, now: Instant) {
        let dt_ms = (now.saturating_duration_since(self.last_tick).as_secs_f32() * 1000.0).min(250.0);
        self.last_tick = now;

        //  Resultado da checagem de versões: uma mensagem e a thread termina.
        if let Some(rx) = self.checando.take() {
            match rx.try_recv() {
                Ok(n) => self.aplica_novidades(n),
                Err(TryRecvError::Empty) => self.checando = Some(rx),
                Err(TryRecvError::Disconnected) => {
                    self.loga("verificação de versões não concluiu")
                }
            }
        }

        let mut events = Vec::new();
        if let Some(l) = self.link.as_ref() {
            if !l.drain(&mut events) {
                self.falha("A conexão serial foi perdida. Reconecte a placa.");
                self.link = None;
                self.finalizar_gravacao("conexão perdida", false);
                self.solta_controle();
            }
        }

        for ev in events {
            match ev {
                //  O banner amigável já foi posto acima; aqui só fica o detalhe
                //  no log (antes este `falha` sobrescrevia a mensagem melhor).
                Event::Error(e) => self.loga(format!("Erro na serial: {e}")),
                Event::Msg(Msg::Text(t)) => {
                    if let Some(rest) = t.strip_prefix("firmware v") {
                        let mut campos = rest.split_whitespace();
                        self.version = campos.next().unwrap_or("?").into();
                        if let Some(boots) = campos
                            .find_map(|w| w.strip_prefix("boots="))
                            .and_then(|v| v.parse().ok())
                        {
                            self.registra_boots(boots);
                        }
                    }
                    if !t.starts_with("[JOYSTICKS]") {
                        self.loga(t);
                    }
                }
                Event::Msg(Msg::Diag(d)) => {
                    self.version = d.version.clone();
                    self.limits = d.limits;
                    self.registra_boots(d.boots);
                }
                Event::Msg(Msg::Status(s)) => {
                    self.last_status = Some(now);
                    self.live = s.pose;
                    self.garra_travada = s.trava;
                    if s.pc_mode {
                        self.last_assume = None;
                    }
                    let em_transicao = self
                        .last_assume
                        .is_some_and(|t| now.saturating_duration_since(t) < Duration::from_millis(250));
                    // O firmware é dono da verdade sobre quem controla.
                    if self.pc && !s.pc_mode && !em_transicao {
                        self.solta_controle();
                        self.loga("joystick assumiu o controle");
                    }
                    if let Some((rec, t0)) = self.recording.as_mut() {
                        rec.samples.push((t0.elapsed().as_millis() as u32, s.pose));
                    }
                }
            }
        }

        //  Em modo PC o firmware manda status a 20 Hz. Um segundo de silêncio
        //  significa placa travada ou sem resposta; devolver o controle evita
        //  continuar mandando #P para uma placa que não está ouvindo.
        if self.pc
            && self.link.is_some()
            && self
                .last_status
                .is_some_and(|t| now.saturating_duration_since(t) > Duration::from_secs(1))
        {
            self.solta_controle();
            self.loga("placa parou de responder — controle devolvido ao joystick");
        }

        if self.link.is_some() && now.saturating_duration_since(self.last_diag) >= Duration::from_secs(2) {
            self.last_diag = now;
            self.send(protocol::CMD_DIAG);
        }

        // Replay: aguarda pré-posicionamento e avança conforme velocidade e pausa.
        if let Some(mut play) = self.play.take() {
            match play.step(
                dt_ms,
                self.vel_replay,
                self.looping,
                self.live,
                self.limits,
                self.garra_travada,
            ) {
                Some(p) => {
                    self.pose = p;
                    self.play = Some(play);
                }
                None => {
                    if let Some(last) = play.rec.sample_at(play.rec.duration_ms() as f32) {
                        self.pose = last;
                    }
                    self.loga("fim do movimento");
                }
            }
        }

        // Toque ou tecla cancela o replay e o alvo manual: quem está com a mão
        // (ou no teclado) manda.
        let teclado_ativo = self.tecla_l != egui::Vec2::ZERO || self.tecla_r != egui::Vec2::ZERO;
        if self.pad_l.stick != egui::Vec2::ZERO
            || self.pad_r.stick != egui::Vec2::ZERO
            || teclado_ativo
        {
            if self.play.is_some() {
                self.play = None;
                self.loga("replay cancelado (pad ou teclado)");
            }
            if self.alvo_manual {
                self.alvo_manual = false;
                self.loga("alvo manual cancelado (pad ou teclado)");
            }
        }

        if self.link.is_some() && now.saturating_duration_since(self.last_send) >= TICK {
            self.last_send = now;
            if self.pc {
                let cmd = if self.play.is_some() || self.alvo_manual {
                    protocol::cmd_targets(self.pose)
                } else {
                    //  Toque tem prioridade; sem toque, o teclado sintetiza o
                    //  mesmo stick e passa pela mesma rampa/limites do firmware.
                    let stick_l = if self.pad_l.stick != egui::Vec2::ZERO {
                        self.pad_l.stick
                    } else {
                        self.tecla_l
                    };
                    let stick_r = if self.pad_r.stick != egui::Vec2::ZERO {
                        self.pad_r.stick
                    } else {
                        self.tecla_r
                    };
                    protocol::cmd_sticks([
                        (stick_l.x * 100.0) as i32,
                        (-stick_l.y * 100.0) as i32, // tela: y para baixo; stick: y para cima
                        (stick_r.x * 100.0) as i32,
                        (-stick_r.y * 100.0) as i32,
                    ])
                };
                self.send(&cmd);
            } else {
                self.send(protocol::CMD_STATUS);
            }
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
            .on_hover_text("Atualiza a lista de portas seriais")
            .clicked()
        {
            self.refresh_ports();
        }
        let porta = self
            .ports
            .get(self.port_idx)
            .map(|p| texto_curto(p, 18))
            .unwrap_or_else(|| "nenhuma porta".into());
        let combo = egui::ComboBox::from_id_salt("porta")
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
        combo
            .response
            .on_hover_text("Porta serial da placa (seleção pelo mouse)");

        if self.link.is_none() {
            let tem_porta = self.ports.get(self.port_idx).is_some();
            let resposta = ui
                .add_enabled_ui(tem_porta, |ui| botao(ui, Icone::Conectar, "Conectar", 124.0))
                .inner
                .on_hover_text(if tem_porta {
                    "Abre a porta serial selecionada (conexão pelo mouse)"
                } else {
                    "Conecte a placa e atualize a lista de portas"
                });
            if resposta.clicked() {
                self.connect();
            }
        } else {
            let resposta = botao(ui, Icone::Desconectar, "Desconectar", 138.0).on_hover_text(
                "Fecha a porta, salva a gravação em andamento e devolve ao joystick",
            );
            if resposta.clicked() {
                self.disconnect();
            }
        }
    }

    /// Grupo de controle: assumir/devolver o comando e travar/liberar a garra.
    ///
    /// O estado exibido vem do firmware (`pc`/`garra_travada`), não do último
    /// clique; se a placa discordar, o botão acompanha a placa no próximo status.
    fn controle_bar(&mut self, ui: &mut egui::Ui) {
        if self.pc {
            let resposta = botao(ui, Icone::Joystick, "Devolver ao joystick", 200.0).on_hover_text(
                "Congela no ângulo atual e devolve o controle ao joystick\nTecla: J",
            );
            if resposta.clicked() {
                self.devolve_ao_joystick();
            }
        } else {
            let resposta = ui
                .add_enabled_ui(self.link.is_some(), |ui| {
                    botao(ui, Icone::Monitor, "Assumir controle", 184.0)
                })
                .inner
                .on_hover_text(
                    "Assume o controle do PC a partir do ângulo atual\nTecla: M",
                );
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
            .on_hover_text(
                "Mantém pressão contínua no objeto e bloqueia a abertura\nTecla: G",
            )
            .clicked()
        {
            self.alterna_trava_garra();
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
             (economia), então o braço pode ceder com o próprio peso.\n\
             Tecla: Espaço",
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

    /// Solta a autoridade do PC e zera tudo que depende dela.
    ///
    /// É o teardown único de "assumir/devolver/STOP/quedas": antes cada caminho
    /// repetia os campos e já divergia (`alvo_manual` ficava fantasma e o app
    /// continuava mandando `#P` em vez de `#V` até o primeiro toque). A
    /// gravação não é tocada aqui — quem decide encerrar é
    /// [`Self::finalizar_gravacao`].
    fn solta_controle(&mut self) {
        self.pc = false;
        self.last_assume = None;
        self.play = None;
        self.alvo_manual = false;
        self.reset_targets();
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
    /// O teclado sintetiza o mesmo stick: A/D e W/S no pad esquerdo, setas no
    /// direito (Shift = fino). O botão **Atalhos** no cabeçalho abre a lista.
    /// `centralizar` (dock lateral) divide a sobra vertical; empilhado, o
    /// conteúdo flui de cima. Clicar num chip de ângulo devolve a junta em
    /// `Option<usize>` para abrir o modal.
    fn sticks(&mut self, ui: &mut egui::Ui, altura_viewport: f32, centralizar: bool) {
        let disponivel = egui::vec2(ui.available_width(), altura_viewport);
        let geo = geometria_pads(disponivel);

        ui.horizontal(|ui| {
            titulo_secao(ui, "Controle");
            if geo.compacto && disponivel.x >= 720.0 {
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("·  Arraste para mover, solte para segurar")
                        .small()
                        .color(COR_MUTED),
                );
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if icons::botao_texto(ui, "Atalhos", self.atalhos_abertos, 96.0)
                    .on_hover_text("lista de atalhos do teclado (H)")
                    .clicked()
                {
                    self.atalhos_abertos = !self.atalhos_abertos;
                }
                let rotulo = if self.tem_novidade() { "Firmware •" } else { "Firmware" };
                if icons::botao_texto(ui, rotulo, self.firmware_aberto, 104.0)
                    .on_hover_text("firmware embutido, versão da placa e atualizações")
                    .clicked()
                {
                    self.firmware_aberto = !self.firmware_aberto;
                }
                if self.pc && (self.tecla_l != egui::Vec2::ZERO || self.tecla_r != egui::Vec2::ZERO)
                {
                    pill(ui, "teclado ativo", COR_ACENTO, cor(0x1b, 0x2a, 0x3a));
                }
                if self.link.is_none() {
                    pill(ui, "conecte e assuma o controle", COR_MUTED, COR_FUNDO);
                } else if self.pc {
                    pill(ui, "pads + teclado", COR_OK, COR_OK_FUNDO);
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
        //  Com a lista de atalhos, o modal de firmware ou o de ângulo na frente,
        //  os pads ficam desligados: a tecla/modal é a única entrada de movimento.
        let habilitado = conectado
            && self.pc
            && !self.atalhos_abertos
            && !self.firmware_aberto
            && self.editando_junta.is_none();
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
                    "Arraste para mover.\nTeclado: A/D = Base, W/S = Cotovelo\nShift = movimento fino",
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
                    "Arraste para mover.\nTeclado: ←/→ = Garra, ↑/↓ = Ombro\nShift = movimento fino",
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
                    "Arraste para mover.\nTeclado: A/D = Base, W/S = Cotovelo\nShift = movimento fino",
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
                    "Arraste para mover.\nTeclado: ←/→ = Garra, ↑/↓ = Ombro\nShift = movimento fino",
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
        //  O campo de nome só existe quando não está gravando; o foco é
        //  recalculado a cada frame para o teclado saber quando não deve agir.
        self.nome_em_foco = false;
        let gap_secao = if compacta { 12.0 } else { 20.0 };
        ui.add_space(if compacta { 2.0 } else { 6.0 });
        titulo_secao(ui, "Gravação");
        ui.add_space(6.0);

        if let Some((rec, _)) = &self.recording {
            //  Tempo capturado (última amostra), não tempo de parede: se a
            //  placa parar de responder, o contador não continua crescendo.
            let dur_s = rec.samples.last().map(|s| s.0 as f32 / 1000.0).unwrap_or(0.0);
            pill(
                ui,
                &format!("Gravando · {dur_s:.1}s · {} amostras", rec.samples.len()),
                COR_REC,
                COR_ERRO_FUNDO,
            );
            ui.add_space(8.0);
            let resposta = botao(ui, Icone::Stop, "Parar gravação e salvar", ui.available_width())
                .on_hover_text(
                    "Encerra a gravação, corta as pausas das pontas e salva\nTecla: R",
                );
            if resposta.clicked() {
                self.finalizar_gravacao("parada manual", true);
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
                        let resp = ui.add_sized(
                            [ui.available_width(), 24.0],
                            egui::TextEdit::singleline(&mut self.rec_name)
                                .frame(false)
                                .vertical_align(egui::Align::Center),
                        );
                        self.nome_em_foco = resp.has_focus();
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
                    "Captura a posição a 20 Hz, inclusive pelo joystick\nTecla: R (também para e salva)"
                } else {
                    "Conecte a placa para gravar"
                });
            if resposta.clicked() {
                self.inicia_gravacao();
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
            let resposta = icons::toggle(
                ui,
                self.looping,
                Icone::Repetir,
                "Repetir continuamente",
                ui.available_width(),
            )
            .on_hover_text("Repete o replay continuamente\nTecla: L");
            if resposta.clicked() {
                self.looping = !self.looping;
            }
            ui.add_space(6.0);
            ui.label(egui::RichText::new("Velocidade").small().color(COR_MUTED));
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                let larg_vel = (ui.available_width() - 20.0) / 3.0;
                for v in VELOCIDADES_REPLAY {
                    let ativo = (self.vel_replay - v).abs() < 0.01;
                    let rotulo = format!("{v}×");
                    let tecla = match v {
                        0.5 => "1",
                        1.0 => "2",
                        _ => "3",
                    };
                    let resposta = icons::botao_texto(ui, &rotulo, ativo, larg_vel)
                        .on_hover_text(format!("Velocidade {rotulo}\nTecla: {tecla}"));
                    if resposta.clicked() {
                        self.vel_replay = v;
                    }
                }
            });
            ui.add_space(6.0);
            if self.play.is_none() {
                let pode = self.link.is_some();
                let resposta = ui
                    .add_enabled_ui(pode, |ui| {
                        botao(ui, Icone::Play, "Reproduzir movimento", ui.available_width())
                    })
                    .inner
                    .on_hover_text(if pode {
                        "Assume o controle do PC e inicia o replay\nTecla: P"
                    } else {
                        "Conecte a placa para iniciar o replay"
                    });
                if resposta.clicked() {
                    self.inicia_replay(r.clone());
                }
            } else if let Some(play) = &self.play {
                let dur_ms = play.rec.duration_ms() as f32;
                let frac = if play.preparando || dur_ms <= 0.0 {
                    0.0
                } else {
                    (play.pos_ms / dur_ms).clamp(0.0, 1.0)
                };
                let status_txt = if play.preparando {
                    if play.pausado {
                        "Pausado no pré-posicionamento".to_string()
                    } else {
                        "Indo para o início…".to_string()
                    }
                } else if play.pausado {
                    format!(
                        "Pausado · {:.1}s / {:.1}s",
                        play.pos_ms / 1000.0,
                        dur_ms / 1000.0
                    )
                } else {
                    format!(
                        "Reproduzindo ({}×) · {:.1}s / {:.1}s",
                        self.vel_replay,
                        play.pos_ms / 1000.0,
                        dur_ms / 1000.0
                    )
                };
                ui.label(
                    egui::RichText::new(status_txt)
                        .small()
                        .color(if play.pausado { COR_WARN } else { COR_OK }),
                );
                ui.add_space(4.0);
                let (bar_rect, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 10.0),
                    egui::Sense::hover(),
                );
                let painter = ui.painter();
                painter.rect_filled(bar_rect, 5.0, cor(0x0d, 0x11, 0x15));
                painter.rect_stroke(
                    bar_rect,
                    5.0,
                    egui::Stroke::new(1.0_f32, cor(0x2c, 0x36, 0x43)),
                    egui::StrokeKind::Inside,
                );
                if frac > 0.0 {
                    let larg_fill = (bar_rect.width() * frac).max(8.0).min(bar_rect.width());
                    let fill_rect = egui::Rect::from_min_size(
                        bar_rect.min,
                        egui::vec2(larg_fill, bar_rect.height()),
                    );
                    let fill_cor = if play.pausado { COR_WARN } else { COR_ACENTO };
                    painter.rect_filled(fill_rect, 5.0, fill_cor);
                }
                ui.add_space(6.0);

                let pausado = play.pausado;
                let mut toggle_pausa = false;
                let mut parar = false;
                ui.horizontal(|ui| {
                    let larg = (ui.available_width() - 10.0) / 2.0;
                    let (ico, rot) = if pausado {
                        (Icone::Play, "Continuar")
                    } else {
                        (Icone::Pause, "Pausar")
                    };
                    let resposta = botao(ui, ico, rot, larg).on_hover_text(if pausado {
                        "Continua o replay de onde parou\nTecla: P"
                    } else {
                        "Pausa o replay\nTecla: P"
                    });
                    if resposta.clicked() {
                        toggle_pausa = true;
                    }
                    let resposta = botao(ui, Icone::Stop, "Parar", ui.available_width())
                        .on_hover_text(
                            "Para o replay e devolve o controle ao joystick\nTecla: Espaço",
                        );
                    if resposta.clicked() {
                        parar = true;
                    }
                });
                if toggle_pausa {
                    let mut agora_pausado = None;
                    if let Some(play) = self.play.as_mut() {
                        play.pausado = !play.pausado;
                        agora_pausado = Some(play.pausado);
                    }
                    if let Some(pausado) = agora_pausado {
                        self.loga(if pausado {
                            "replay pausado"
                        } else {
                            "replay retomado"
                        });
                    }
                } else if parar {
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
                                if self.loaded_path.as_ref() == Some(&f) {
                                    self.loaded = None;
                                    self.loaded_path = None;
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
                let selecionado = self.loaded_path.as_ref() == Some(&f);
                let largura_nome = (ui.available_width() - ALVO - 10.0).max(60.0);
                let nome_visivel = texto_curto(&nome, 30);
                let resposta = icons::botao_texto(ui, &nome_visivel, selecionado, largura_nome);
                if resposta.on_hover_text("carregar este movimento").clicked() {
                    match Recording::load(&f) {
                        Ok(r) => {
                            self.loaded = Some(r);
                            self.loaded_path = Some(f.clone());
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

    /// Log recolhível: última linha fechada, painel completo aberto.
    ///
    /// Fechado, mostra a última linha colorida por [`cor_log`] e a contagem.
    /// Aberto, tem contagem, **Copiar**/**Limpar**, moldura escura e um scroll
    /// fixo de 160 pt que acompanha o fim; as linhas são selecionáveis.
    fn log_panel(&mut self, ui: &mut egui::Ui) {
        let icone = if self.log_aberta {
            Icone::SetaBaixo
        } else {
            Icone::SetaCima
        };
        let (rect, resp) =
            ui.allocate_exact_size(egui::vec2(ui.available_width(), 36.0), egui::Sense::click());
        let resp = resp.on_hover_text("Mostra/oculta o histórico de eventos");
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
            let ultima = self.log.last().map(|s| s.as_str());
            let texto = ultima.unwrap_or("sem eventos");
            let limite = ((rect.width() - 130.0) / 7.5).max(16.0) as usize;
            p.text(
                egui::pos2(rect.left() + 72.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                texto_curto(texto, limite),
                egui::FontId::monospace(12.0),
                ultima.map(cor_log).unwrap_or(COR_MUTED),
            );
            if !self.log.is_empty() {
                p.text(
                    egui::pos2(rect.right() - 12.0, rect.center().y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{}", self.log.len()),
                    egui::FontId::proportional(11.5),
                    COR_MUTED,
                );
            }
        }
        if self.log_aberta {
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("{} eventos", self.log.len()))
                        .small()
                        .color(COR_MUTED),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let limpar = ui
                        .add_enabled_ui(!self.log.is_empty(), |ui| {
                            ui.add_sized(
                                [64.0, 26.0],
                                egui::Button::new(egui::RichText::new("Limpar").small()),
                            )
                        })
                        .inner
                        .on_hover_text("Apaga o histórico");
                    if limpar.clicked() {
                        self.log.clear();
                    }
                    let copiar = ui
                        .add_enabled_ui(!self.log.is_empty(), |ui| {
                            ui.add_sized(
                                [64.0, 26.0],
                                egui::Button::new(egui::RichText::new("Copiar").small()),
                            )
                        })
                        .inner
                        .on_hover_text("Copia todas as linhas para a área de transferência");
                    if copiar.clicked() {
                        ui.ctx().copy_text(self.log.join("\n"));
                    }
                });
            });
            ui.add_space(6.0);
            egui::Frame::NONE
                .fill(cor(0x0d, 0x11, 0x15))
                .stroke(egui::Stroke::new(1.0_f32, cor(0x2c, 0x36, 0x43)))
                .corner_radius(8)
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    egui::ScrollArea::vertical()
                        .stick_to_bottom(true)
                        .auto_shrink([false, false])
                        .max_height(160.0)
                        .show(ui, |ui| {
                            for l in &self.log {
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(l)
                                            .monospace()
                                            .size(12.5)
                                            .color(cor_log(l)),
                                    )
                                    .selectable(true),
                                );
                            }
                        });
                });
        }
    }

    /// Popup com a lista de atalhos; abre pelo botão "Atalhos" ou H/F1.
    ///
    /// O tamanho é fixo por frame, derivado da janela do app: a lista usa duas
    /// colunas e o miolo rola em tela baixa, com o botão Fechar fixo. A altura
    /// do scroll nunca vem de `available_height`, senão a janela cresce junto.
    /// Fecha no X, no botão, em Esc ou em H/F1.
    fn atalhos_modal(&mut self, ctx: &egui::Context) {
        if !self.atalhos_abertos {
            return;
        }
        let tela = ctx.content_rect();
        let largura = (tela.width() - 80.0).clamp(560.0, 760.0);
        let altura = (tela.height() - 80.0).clamp(300.0, 560.0);
        //  Altura do scroll derivada do tamanho fixo do popup, nunca de
        //  `ui.available_height()`: o egui recalcula a janela a partir do
        //  conteúdo e o scroll cresceria junto (popup "vivo").
        let altura_scroll = (altura - 150.0).max(120.0);
        let mut aberto = true;
        let mut fechar = false;
        egui::Window::new("Atalhos do teclado")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(largura, altura))
            .open(&mut aberto)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new(
                        "Movimento exige o PC no controle (M). Com um campo de texto \
                         focado, as teclas não controlam o braço.",
                    )
                    .small()
                    .color(COR_MUTED),
                );
                ui.add_space(8.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(altura_scroll)
                    .show(ui, |ui| {
                        ui.columns(2, |cols| {
                            let esq = &mut cols[0];
                            titulo_secao(esq, "Movimento");
                            egui::Grid::new("atalhos_movimento")
                                .num_columns(2)
                                .spacing([18.0, 6.0])
                                .show(esq, |ui| {
                                    linha_atalho(ui, "A / D", "Base: girar à esquerda / direita");
                                    linha_atalho(ui, "W / S", "Cotovelo: subir / descer");
                                    linha_atalho(ui, "← / →", "Garra: fechar / abrir");
                                    linha_atalho(ui, "↑ / ↓", "Ombro: subir / descer");
                                    linha_atalho(
                                        ui,
                                        "Shift",
                                        "Movimento fino (metade da velocidade)",
                                    );
                                });
                            esq.add_space(12.0);
                            titulo_secao(esq, "Controle");
                            egui::Grid::new("atalhos_controle")
                                .num_columns(2)
                                .spacing([18.0, 6.0])
                                .show(esq, |ui| {
                                    linha_atalho(ui, "M", "Assumir o controle (PC)");
                                    linha_atalho(ui, "J", "Devolver ao joystick");
                                    linha_atalho(
                                        ui,
                                        "Espaço",
                                        "STOP: parar e devolver ao joystick",
                                    );
                                    linha_atalho(ui, "G", "Travar / liberar a garra");
                                });

                            let dir = &mut cols[1];
                            titulo_secao(dir, "Gravação e replay");
                            egui::Grid::new("atalhos_gravacao")
                                .num_columns(2)
                                .spacing([18.0, 6.0])
                                .show(dir, |ui| {
                                    linha_atalho(
                                        ui,
                                        "R",
                                        "Iniciar / parar e salvar a gravação",
                                    );
                                    linha_atalho(
                                        ui,
                                        "P",
                                        "Reproduzir / pausar / continuar",
                                    );
                                    linha_atalho(ui, "1 / 2 / 3", "Velocidade 0,5× / 1× / 2×");
                                    linha_atalho(ui, "L", "Repetir o movimento (loop)");
                                });
                            dir.add_space(12.0);
                            titulo_secao(dir, "Interface");
                            egui::Grid::new("atalhos_interface")
                                .num_columns(2)
                                .spacing([18.0, 6.0])
                                .show(dir, |ui| {
                                    linha_atalho(ui, "H / F1", "Abrir / fechar esta lista");
                                    linha_atalho(
                                        ui,
                                        "Esc",
                                        "Fechar a lista ou o modal de ângulo",
                                    );
                                    linha_atalho(
                                        ui,
                                        "Enter",
                                        "Confirmar o ângulo no modal",
                                    );
                                });
                            dir.add_space(12.0);
                            dir.label(
                                egui::RichText::new(
                                    "Conectar, escolher a porta, carregar/excluir movimentos \
                                     e digitar o nome continuam no mouse/toque.",
                                )
                                .small()
                                .color(COR_MUTED),
                            );
                        });
                    });
                ui.add_space(10.0);
                if icons::botao_texto(ui, "Fechar", false, ui.available_width()).clicked() {
                    fechar = true;
                }
            });
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            fechar = true;
        }
        if !aberto {
            fechar = true;
        }
        if fechar {
            self.atalhos_abertos = false;
        }
    }

    /// Popup "Firmware e atualizações".
    ///
    /// Reúne as três versões (embutida, a que a placa reporta em `#B` e a
    /// publicada no repositório), o botão que grava o sketch na pasta do
    /// usuário e o link da release. O app não grava firmware na placa: o
    /// upload continua sendo do Arduino IDE/arduino-cli.
    ///
    /// Tudo que vem de `self` é copiado antes da janela para não manter
    /// empréstimo enquanto os botões chamam `&mut self`.
    fn firmware_modal(&mut self, ctx: &egui::Context) {
        if !self.firmware_aberto {
            return;
        }
        let embutida = firmware::versao().to_string();
        let app_atual = update::versao_app().to_string();
        let placa = self.version.clone();
        let conectado = self.link.is_some();
        let checando = self.checando.is_some();
        let disponivel = self.novidades.as_ref().and_then(|n| n.firmware.clone());
        let app_novo = self.novidades.as_ref().and_then(|n| n.app.clone());
        let erro = self.novidades.as_ref().and_then(|n| n.erro.clone());
        let salvo = self.sketch_salvo.clone();

        let tela = ctx.content_rect();
        //  Cabe na janela mínima (760×320): 20 de margem e corpo rolável.
        let largura = (tela.width() - 40.0).clamp(420.0, 660.0);
        let altura = (tela.height() - 40.0).clamp(280.0, 560.0);
        let mut aberto = true;
        let mut fechar = false;

        egui::Window::new("Firmware e atualizações")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .fixed_size(egui::vec2(largura, altura))
            .open(&mut aberto)
            .show(ctx, |ui| {
                let novo_fw = disponivel.as_deref().and_then(|v| firmware::compara(v, &embutida))
                    == Some(Ordering::Greater);
                let novo_app = app_novo.as_deref().and_then(|v| firmware::compara(v, &app_atual))
                    == Some(Ordering::Greater);

                //  Todo o corpo rola; só o Fechar fica fixo. Sem isso o
                //  conteúdo empurra a janela para fora em telas baixas.
                let altura_scroll = (altura - 96.0).max(120.0);
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(altura_scroll)
                    .show(ui, |ui| {
                        egui::Grid::new("fw_versoes")
                            .num_columns(2)
                            .spacing([18.0, 6.0])
                            .show(ui, |ui| {
                                ui.label("Embutido no app");
                                ui.label(egui::RichText::new(&embutida).strong());
                                ui.end_row();

                                ui.label("Na placa");
                                ui.label(if conectado {
                                    placa.clone()
                                } else {
                                    "— (desconectado)".to_string()
                                });
                                ui.end_row();

                                ui.label("No repositório");
                                ui.label(match &disponivel {
                                    Some(v) if novo_fw => format!("{v} (mais novo)"),
                                    Some(v) => v.clone(),
                                    None => "—".to_string(),
                                });
                                ui.end_row();

                                ui.label("App");
                                ui.label(match &app_novo {
                                    Some(v) if novo_app => format!("{app_atual} → {v} disponível"),
                                    _ => app_atual.clone(),
                                });
                                ui.end_row();
                            });

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if checando {
                                ui.label(
                                    egui::RichText::new("verificando…").small().color(COR_MUTED),
                                );
                            } else if icons::botao_texto(ui, "Verificar agora", false, 148.0)
                                .clicked()
                            {
                                self.inicia_check();
                            }
                            if icons::botao_texto(ui, "Salvar sketch (.ino)", false, 184.0)
                                .on_hover_text("grava o firmware na pasta do usuário para upload")
                                .clicked()
                            {
                                self.salvar_sketch();
                            }
                            if novo_app {
                                if let Some(url) = app_novo.as_deref().and_then(update::url_release) {
                                    if icons::botao_texto(ui, "Abrir release", false, 132.0)
                                        .clicked()
                                    {
                                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                                    }
                                }
                            }
                        });

                        if let Some(p) = &salvo {
                            ui.add_space(6.0);
                            ui.add(
                                egui::Label::new(
                                    egui::RichText::new(format!("salvo em {}", p.display()))
                                        .small()
                                        .color(COR_OK),
                                )
                                .selectable(true),
                            );
                        }
                        if let Some(e) = &erro {
                            ui.add_space(4.0);
                            ui.label(
                                egui::RichText::new(format!("não foi possível verificar: {e}"))
                                    .small()
                                    .color(COR_WARN),
                            );
                        }

                        ui.add_space(8.0);
                        ui.separator();
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Para gravar na placa, abra o arquivo salvo no Arduino IDE \
                                 (Arduino Uno + a porta) e faça Upload; ou use o arduino-cli \
                                 com o mesmo sketch.",
                            )
                            .small(),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Gravar reinicia a placa: os servos ficam sem torque durante o \
                                 upload, então escore o braço antes. A Posição 0 e os limites \
                                 ficam na EEPROM e sobrevivem à regravação.\n\
                                 Clones com CH340 podem precisar do driver do fabricante no \
                                 Windows (e em algumas versões do macOS); o Uno R3 original não.",
                            )
                            .small()
                            .color(COR_MUTED),
                        );
                        ui.add_space(6.0);
                        let pasta = dir_dados().join("firmware");
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(format!("pasta: {}", pasta.display()))
                                    .small()
                                    .color(COR_MUTED),
                            )
                            .selectable(true),
                        );
                    });

                ui.add_space(8.0);
                if icons::botao_texto(ui, "Fechar", false, ui.available_width()).clicked() {
                    fechar = true;
                }
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            fechar = true;
        }
        if !aberto {
            fechar = true;
        }
        if fechar {
            self.firmware_aberto = false;
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
                    let resposta = botao(ui, Icone::Stop, "Cancelar", larg)
                        .on_hover_text("Fecha o modal sem mover\nTecla: Esc");
                    if resposta.clicked() {
                        fechar = true;
                    }
                    let pode = avaliacao.is_ok();
                    let resp = ui
                        .add_enabled_ui(pode, |ui| {
                            botao(ui, Icone::Play, "Mover", ui.available_width())
                        })
                        .inner
                        .on_hover_text(if pode {
                            "Move a junta para o ângulo digitado\nTecla: Enter"
                        } else {
                            "Corrija o valor para mover"
                        });
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
                if !self.pc {
                    self.assume();
                }
                self.pose = self.live;
                self.pose[junta] = alvo as f32;
                self.play = None;
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
        //  Teclado antes do tick: o send do tick já leva o stick sintetizado e
        //  as ações de controle (M/J/STOP/gravar) entram no mesmo frame.
        self.processa_teclado(ctx);
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
        self.atalhos_modal(ctx);
        self.firmware_modal(ctx);
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

/// Cor de uma linha do log: erros/STOP em vermelho, eventos bons em verde.
fn cor_log(linha: &str) -> egui::Color32 {
    let l = linha.to_lowercase();
    let erro = l.contains("erro")
        || l.contains("falha")
        || l.contains("não foi possível")
        || l.contains("perdida")
        || l.contains("reiniciou")
        || l.starts_with("stop");
    if erro {
        COR_ERRO
    } else if l.contains("salvo")
        || l.contains("conectado")
        || l.contains("carregado")
        || l.contains("iniciada")
        || l.contains("reproduzindo")
    {
        COR_OK
    } else {
        cor(0xb8, 0xc4, 0xd0)
    }
}

/// Uma linha da tabela de atalhos: tecla em monoespaçado + descrição.
fn linha_atalho(ui: &mut egui::Ui, teclas: &str, descricao: &str) {
    ui.label(
        egui::RichText::new(teclas)
            .monospace()
            .size(14.5)
            .strong()
            .color(COR_ACENTO),
    );
    ui.label(egui::RichText::new(descricao).size(14.5));
    ui.end_row();
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
/// leituras (`lado + 12 + 116`); `compacto` só aperta os espaçamentos. `dica`
/// vira o tooltip do título (teclas equivalentes do teclado).
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
    dica: &str,
) -> Option<usize> {
    const LARG_LEITURA: f32 = 116.0;
    const ESPACO: f32 = 12.0;
    const ALTURA_LEITURAS: f32 = 144.0;
    let mut clicado = None;

    ui.push_id(id, |ui| {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::Vec2::ZERO;
            ui.set_width(lado + ESPACO + LARG_LEITURA);
            ui.label(egui::RichText::new(titulo).small().strong().color(COR_MUTED))
                .on_hover_text(dica);
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
    fn teclas_digitais_respeitam_sinal_e_modo_fino() {
        assert_eq!(stick_digital(true, false, false), -1.0);
        assert_eq!(stick_digital(false, true, false), 1.0);
        assert_eq!(stick_digital(true, true, false), 0.0);
        assert_eq!(stick_digital(false, false, false), 0.0);
        assert_eq!(stick_digital(false, true, true), 0.5);
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

    #[test]
    fn reinicio_derruba_autoridade_e_zera_alvos() {
        let mut app = App::new();
        app.pc = true;
        app.alvo_manual = true;
        app.boots = Some(3);
        app.pad_l.stick = egui::vec2(1.0, 0.0);
        app.play = Some(PlayState::new(Recording {
            name: "t".into(),
            samples: vec![(0, [90.0; 4])],
        }));

        app.registra_boots(4);

        assert!(!app.pc);
        assert!(!app.alvo_manual);
        assert!(app.play.is_none());
        assert_eq!(app.pad_l.stick, egui::Vec2::ZERO);
        assert_eq!(app.boots, Some(4));
        assert!(app.reboot.is_some());

        // Reset a frio: o contador volta para 1 e ainda é reinício.
        app.pc = true;
        app.registra_boots(1);
        assert!(!app.pc);
        assert_eq!(app.boots, Some(1));
    }
}
