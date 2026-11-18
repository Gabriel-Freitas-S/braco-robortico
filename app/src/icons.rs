//! Ícones do app: atlas rasterizado do conjunto "Ionicons Sharp Interface
//! Icons" (SVG Repo, Ionicons, MIT). O atlas é branco com alfa; [`desenha`]
//! aplica o tint do tema.
//!
//! São 18 conceitos em 5×4 células de 64 px: transporte (play/pause/stop),
//! gravação, cadeado aberto/fechado, refresh, lixeira, fechar, link/unlink,
//! desktop, gamepad, repeat, chevrons, teclado e CPU. Os SVGs fonte ficam em
//! `assets/svgrepo/`; o mapa conceito → id/slug, a coleção e a licença estão em
//! `assets/svgrepo-icons.json`.
//!
//! Todo ícone é desenhado por [`desenha`] centrado em `c` com raio `r`, então
//! o mesmo atlas serve ao botão grande, ao quadrado e ao log.

use egui::{Pos2, Response, Sense, Stroke, Ui, Vec2};

/// Atlas 5×4 de células 64×64 (320×256), RGBA8 branco sobre transparente.
const ATLAS: &[u8] = include_bytes!("../assets/icones_hud.rgba");
const ATLAS_LARGURA: usize = 320;
const ATLAS_ALTURA: usize = 256;
const CELULA: usize = 64;
const COLUNAS: usize = 5;

/// Catálogo de ícones. A ordem do atlas está em [`Icone::indice`].
#[derive(Clone, Copy)]
pub enum Icone {
    Play,
    Pause,
    Stop,
    Gravar,
    Travar,
    Liberar,
    Atualizar,
    Excluir,
    Fechar,
    Conectar,
    Desconectar,
    Monitor,
    Joystick,
    Repetir,
    SetaCima,
    SetaBaixo,
    Atalhos,
    Firmware,
}

impl Icone {
    /// Nome do ícone para leitores de tela e tooltip padrão.
    fn rotulo(self) -> &'static str {
        match self {
            Self::Play => "reproduzir",
            Self::Pause => "pausar",
            Self::Stop => "parar",
            Self::Gravar => "gravar",
            Self::Travar => "travar garra",
            Self::Liberar => "liberar garra",
            Self::Atualizar => "atualizar",
            Self::Excluir => "excluir",
            Self::Fechar => "fechar",
            Self::Conectar => "conectar",
            Self::Desconectar => "desconectar",
            Self::Monitor => "assumir controle",
            Self::Joystick => "devolver ao joystick",
            Self::Repetir => "repetir continuamente",
            Self::SetaCima => "expandir",
            Self::SetaBaixo => "recolher",
            Self::Atalhos => "atalhos do teclado",
            Self::Firmware => "firmware e atualizações",
        }
    }

    /// Posição no atlas, linha a linha.
    fn indice(self) -> usize {
        match self {
            Self::Play => 0,
            Self::Pause => 1,
            Self::Stop => 2,
            Self::Gravar => 3,
            Self::Travar => 4,
            Self::Liberar => 5,
            Self::Atualizar => 6,
            Self::Excluir => 7,
            Self::Fechar => 8,
            Self::Conectar => 9,
            Self::Desconectar => 10,
            Self::Monitor => 11,
            Self::Joystick => 12,
            Self::Repetir => 13,
            Self::SetaCima => 14,
            Self::SetaBaixo => 15,
            Self::Atalhos => 16,
            Self::Firmware => 17,
        }
    }
}

/// Textura do atlas, carregada uma vez por `Context`.
fn textura_icones(ctx: &egui::Context) -> egui::TextureId {
    let id = egui::Id::new("svgrepo_icons_atlas");
    if let Some(handle) = ctx.data(|d| d.get_temp::<egui::TextureHandle>(id)) {
        return handle.id();
    }
    let imagem = egui::ColorImage::from_rgba_unmultiplied([ATLAS_LARGURA, ATLAS_ALTURA], ATLAS);
    let handle = ctx.load_texture("svgrepo_icons", imagem, egui::TextureOptions::LINEAR);
    let tid = handle.id();
    ctx.data_mut(|d| d.insert_temp(id, handle));
    tid
}

/// Cores de botão respeitando o estado habilitado sem sumir no fundo escuro.
///
/// Desabilitado usa `noninteractive` + um cinza explícito, porque o padrão do
/// egui fica quase invisível no tema escuro.
fn cores_botao(ui: &Ui, resp: &Response) -> (egui::Color32, Stroke, egui::Color32) {
    let vis = ui.style().interact(resp);
    if ui.is_enabled() {
        (vis.weak_bg_fill, vis.bg_stroke, vis.text_color())
    } else {
        let w = ui.style().visuals.widgets.noninteractive;
        (w.weak_bg_fill, w.bg_stroke, crate::COR_DESABILITADO)
    }
}

/// Fonte do texto de botão, resolvida do estilo (não fixa em 15 pt).
fn fonte_botao(ui: &Ui) -> egui::FontId {
    egui::TextStyle::Button.resolve(ui.style())
}

/// Corte padrão dos botões: 45° no topo-direita e na base-esquerda.
const CORTE_BOTAO: f32 = 5.0;
/// Corte dos botões só de ícone, menor para não comer o desenho.
const CORTE_QUADRADO: f32 = 3.0;

/// Fundo + borda no retângulo cortado.
fn pinta_cortado(
    p: &egui::Painter,
    rect: egui::Rect,
    corte: f32,
    fundo: egui::Color32,
    borda: Stroke,
) {
    let pts = crate::pontos_cortados(rect, corte);
    p.add(egui::Shape::convex_polygon(
        pts.clone(),
        fundo,
        Stroke::NONE,
    ));
    p.add(egui::Shape::closed_line(pts, borda));
}

/// Anel de foco de teclado, por fora do controle e acompanhando o corte.
fn foco_anel(p: &egui::Painter, rect: egui::Rect, corte: f32, resp: &Response) {
    if resp.has_focus() {
        p.add(egui::Shape::closed_line(
            crate::pontos_cortados(rect.expand(2.0), corte + 1.0),
            Stroke::new(1.5_f32, crate::COR_ACENTO),
        ));
    }
}

/// Pinta o corpo comum dos botões: fundo, borda, foco, ícone e texto.
fn pinta_botao(
    ui: &Ui,
    rect: egui::Rect,
    resp: &Response,
    icone: Icone,
    texto: &str,
    fundo: egui::Color32,
    borda: Stroke,
    frente: egui::Color32,
) {
    let p = ui.painter();
    pinta_cortado(p, rect, CORTE_BOTAO, fundo, borda);
    foco_anel(p, rect, CORTE_BOTAO, resp);
    desenha(
        p,
        Pos2::new(rect.left() + 22.0, rect.center().y),
        10.5,
        icone,
        frente,
    );
    p.text(
        Pos2::new(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        fonte_botao(ui),
        frente,
    );
}

/// Botão com ícone à esquerda e texto, no tema do app.
///
/// Altura fixa em `crate::ALVO` (48 pt) e `widget_info` preenchido: o leitor de
/// tela anuncia o texto e o botão entra na navegação por teclado.
pub fn botao(ui: &mut Ui, icone: Icone, texto: &str, largura: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let habilitado = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (fundo, borda, frente) = cores_botao(ui, &resp);
    pinta_botao(ui, rect, &resp, icone, texto, fundo, borda, frente);
    resp
}

/// Ação principal da tela: fundo de seleção e borda de acento.
///
/// Só o próximo passo do fluxo (Conectar → Assumir → Reproduzir) usa esta
/// variante; o resto continua neutro para o STOP seguir sendo o mais forte.
pub fn botao_primario(ui: &mut Ui, icone: Icone, texto: &str, largura: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let habilitado = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (fundo, borda, frente) = if habilitado {
        let fundo = if resp.is_pointer_button_down_on() {
            crate::COR_ACENTO_FUNDO.gamma_multiply(1.28)
        } else if resp.hovered() {
            crate::COR_ACENTO_FUNDO.gamma_multiply(1.15)
        } else {
            crate::COR_ACENTO_FUNDO
        };
        (
            fundo,
            Stroke::new(1.0_f32, crate::COR_ACENTO),
            crate::COR_ACENTO_TEXTO,
        )
    } else {
        cores_botao(ui, &resp)
    };
    pinta_botao(ui, rect, &resp, icone, texto, fundo, borda, frente);
    resp
}

/// Botão de ferramenta do cabeçalho: ícone + texto, com estado aberto/fechado.
pub fn botao_hud(
    ui: &mut Ui,
    icone: Icone,
    texto: &str,
    selecionado: bool,
    largura: f32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let habilitado = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (mut fundo, mut borda, mut frente) = cores_botao(ui, &resp);
    if selecionado && habilitado {
        fundo = crate::COR_ACENTO_FUNDO;
        borda = ui.style().visuals.selection.stroke;
        frente = crate::COR_ACENTO_TEXTO;
    } else if habilitado && !resp.hovered() {
        frente = crate::COR_TITULO;
    }
    pinta_botao(ui, rect, &resp, icone, texto, fundo, borda, frente);
    resp
}

/// Botão de alternância (toggle) touch-friendly para substituir checkboxes pequenos.
///
/// Ativo pinta o fundo com `selection.bg_fill` e acende a bolinha à direita;
/// inativo mantém as cores normais de botão, então o estado nunca depende só da
/// cor do texto.
pub fn toggle(ui: &mut Ui, ativo: bool, icone: Icone, texto: &str, largura: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let habilitado = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (mut fundo, mut borda, mut frente) = cores_botao(ui, &resp);
    if ativo && habilitado {
        fundo = crate::COR_ACENTO_FUNDO;
        borda = ui.style().visuals.selection.stroke;
        frente = crate::COR_ACENTO_TEXTO;
    }
    pinta_botao(ui, rect, &resp, icone, texto, fundo, borda, frente);
    // Indicador visual de estado ligado/desligado à direita
    let pill_c = Pos2::new(rect.right() - 22.0, rect.center().y);
    let pill_cor = if ativo {
        crate::COR_ACENTO
    } else {
        crate::LINHA_FORTE
    };
    ui.painter().circle_filled(pill_c, 5.0, pill_cor);
    resp
}

/// Linha selecionável com texto alinhado à esquerda.
///
/// Usada na lista de movimentos; `selecionado` replica o visual de seleção do
/// tema para o item carregado, sem checkbox.
pub fn botao_texto(ui: &mut Ui, texto: &str, selecionado: bool, largura: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), texto)
    });
    let vis = *ui.style().interact(&resp);
    let (fundo, borda, frente) = if selecionado {
        (
            crate::COR_ACENTO_FUNDO,
            ui.style().visuals.selection.stroke,
            crate::COR_ACENTO_TEXTO,
        )
    } else {
        (vis.weak_bg_fill, vis.bg_stroke, vis.text_color())
    };
    let p = ui.painter();
    pinta_cortado(p, rect, CORTE_BOTAO, fundo, borda);
    foco_anel(p, rect, CORTE_BOTAO, &resp);
    p.text(
        Pos2::new(rect.left() + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        fonte_botao(ui),
        frente,
    );
    resp
}

/// Botão com cores fixas (STOP, travar garra): não segue o tema.
///
/// A cor de fundo clareia no clique e a borda acende no hover; desabilitado cai
/// para o visual `noninteractive`. Serve para ações em que a cor é o aviso.
pub fn botao_cor(
    ui: &mut Ui,
    icone: Icone,
    texto: &str,
    largura: f32,
    fundo: egui::Color32,
    frente: egui::Color32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let habilitado = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (fundo, frente, borda) = if habilitado {
        let f = if resp.is_pointer_button_down_on() {
            fundo.gamma_multiply(1.35)
        } else {
            fundo
        };
        let b = Stroke::new(
            1.0_f32,
            if resp.hovered() {
                frente.gamma_multiply(0.75)
            } else {
                fundo.gamma_multiply(1.7)
            },
        );
        (f, frente, b)
    } else {
        let w = ui.style().visuals.widgets.noninteractive;
        (w.weak_bg_fill, crate::COR_DESABILITADO, w.bg_stroke)
    };
    pinta_botao(ui, rect, &resp, icone, texto, fundo, borda, frente);
    resp
}

/// Botão só de ícone (listas, ações de linha).
///
/// O lado nunca fica abaixo de `crate::ALVO`, mesmo que o chamador peça menor:
/// é a garantia de que o alvo de toque continua confortável.
pub fn botao_quadrado(ui: &mut Ui, icone: Icone, lado: f32) -> Response {
    let lado = lado.max(crate::ALVO);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(lado), Sense::click());
    let habilitado = ui.is_enabled();
    let texto = icone.rotulo();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (fundo, borda, frente) = cores_botao(ui, &resp);
    let p = ui.painter();
    pinta_cortado(p, rect, CORTE_QUADRADO, fundo, borda);
    foco_anel(p, rect, CORTE_QUADRADO, &resp);
    desenha(p, rect.center(), lado * 0.22, icone, frente);
    resp
}

/// Botão só de ícone, compacto, para ações secundárias (atualizar lista).
///
/// Diferente de [`botao_quadrado`], não força 48 pt: é o par visual do botão
/// de 26 pt da barra de log, usado onde o alvo de toque não é primário.
pub fn botao_quadrado_compacto(ui: &mut Ui, icone: Icone, lado: f32) -> Response {
    let lado = lado.max(28.0);
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(lado), Sense::click());
    let habilitado = ui.is_enabled();
    let texto = icone.rotulo();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (fundo, borda, frente) = cores_botao(ui, &resp);
    let p = ui.painter();
    pinta_cortado(p, rect, CORTE_QUADRADO, fundo, borda);
    foco_anel(p, rect, CORTE_QUADRADO, &resp);
    desenha(p, rect.center(), lado * 0.24, icone, frente);
    resp
}

/// Botão compacto de texto (barra de log), com o mesmo corte dos demais.
pub fn botao_compacto(ui: &mut Ui, texto: &str, largura: f32) -> Response {
    let altura = 26.0;
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, altura), Sense::click());
    let habilitado = ui.is_enabled();
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, habilitado, texto));
    let (fundo, borda, frente) = cores_botao(ui, &resp);
    let p = ui.painter();
    pinta_cortado(p, rect, 3.0, fundo, borda);
    foco_anel(p, rect, 3.0, &resp);
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        texto,
        egui::TextStyle::Small.resolve(ui.style()),
        frente,
    );
    resp
}

/// Desenha o ícone centrado em `c` com raio `r`.
///
/// O atlas é branco; `cor` entra como tint. O raio `r` é a metade do lado do
/// quadrado visível, como antes dos ícones vetoriais.
pub fn desenha(p: &egui::Painter, c: Pos2, r: f32, icone: Icone, cor: egui::Color32) {
    let idx = icone.indice();
    let col = idx % COLUNAS;
    let row = idx / COLUNAS;
    let u = CELULA as f32 / ATLAS_LARGURA as f32;
    let v = CELULA as f32 / ATLAS_ALTURA as f32;
    let uv = egui::Rect::from_min_max(
        egui::pos2(col as f32 * u, row as f32 * v),
        egui::pos2((col + 1) as f32 * u, (row + 1) as f32 * v),
    );
    let rect = egui::Rect::from_center_size(c, Vec2::splat(r * 2.0));
    p.image(textura_icones(p.ctx()), rect, uv, cor);
}
