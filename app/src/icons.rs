//! Ícones vetoriais desenhados na mão.
//!
//! Não uso glifo de fonte porque o conjunto de símbolos que as fontes padrão do
//! egui cobrem varia (e o que falta vira "quadradinho"). Desenhando com a
//! painter, o ícone existe sempre e acompanha a cor do tema.
//!
//! Todo ícone é desenhado por [`desenha`] centrado em `c` com raio `r`: nada é
//! posicionado em pixels absolutos, então o mesmo desenho serve ao botão grande
//! (raio ~9,5) e ao quadrado (raio ~`lado * 0.22`). A espessura do traço é
//! derivada do raio (`max(r * 0.28, 1.4)`), o que mantém o peso visual igual em
//! qualquer tamanho. Os botões usam o alvo de toque de `crate::ALVO` (48 pt).

use egui::{Pos2, Response, Sense, Stroke, Ui, Vec2};

/// Catálogo de ícones. Os pares com estado são desenhados pelo mesmo código com
/// uma diferença pequena: [`Icone::Travar`]/[`Icone::Liberar`] (arco do cadeado)
/// e [`Icone::Conectar`]/[`Icone::Desconectar`] (raio cortado).
#[derive(Clone, Copy)]
pub enum Icone {
    Play,
    Stop,
    Gravar,
    Travar,
    Liberar,
    Atualizar,
    Excluir,
    Conectar,
    Desconectar,
    Monitor,
    Joystick,
    Repetir,
    SetaCima,
    SetaBaixo,
}

impl Icone {
    /// Nome do ícone para leitores de tela e tooltip padrão.
    fn rotulo(self) -> &'static str {
        match self {
            Self::Play => "reproduzir",
            Self::Stop => "parar",
            Self::Gravar => "gravar",
            Self::Travar => "travar garra",
            Self::Liberar => "liberar garra",
            Self::Atualizar => "atualizar",
            Self::Excluir => "excluir",
            Self::Conectar => "conectar",
            Self::Desconectar => "desconectar",
            Self::Monitor => "assumir controle",
            Self::Joystick => "devolver ao joystick",
            Self::Repetir => "repetir continuamente",
            Self::SetaCima => "expandir",
            Self::SetaBaixo => "recolher",
        }
    }
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
        (w.weak_bg_fill, w.bg_stroke, crate::cor(0x6e, 0x7d, 0x8d))
    }
}

/// Anel de foco de teclado, desenhado por fora do controle para não encolher o
/// conteúdo. Só aparece quando o widget tem foco (`Tab`/setas).
fn foco_anel(p: &egui::Painter, rect: egui::Rect, raio: egui::CornerRadius, resp: &Response) {
    if resp.has_focus() {
        p.rect_stroke(
            rect.expand(2.0),
            raio,
            Stroke::new(1.5_f32, crate::COR_ACENTO),
            egui::StrokeKind::Inside,
        );
    }
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
    let raio = ui.style().interact(&resp).corner_radius;

    let p = ui.painter();
    p.rect(rect, raio, fundo, borda, egui::StrokeKind::Inside);
    foco_anel(p, rect, raio, &resp);

    let centro_icone = Pos2::new(rect.left() + 22.0, rect.center().y);
    desenha(p, centro_icone, 9.5, icone, frente);
    p.text(
        Pos2::new(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        egui::FontId::proportional(15.0),
        frente,
    );
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
        fundo = ui.style().visuals.selection.bg_fill;
        borda = ui.style().visuals.selection.stroke;
        frente = crate::cor(0xe8, 0xf2, 0xfc);
    }
    let raio = ui.style().interact(&resp).corner_radius;

    let p = ui.painter();
    p.rect(rect, raio, fundo, borda, egui::StrokeKind::Inside);
    foco_anel(p, rect, raio, &resp);

    desenha(p, Pos2::new(rect.left() + 22.0, rect.center().y), 9.5, icone, frente);
    p.text(
        Pos2::new(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        egui::FontId::proportional(15.0),
        frente,
    );
    // Indicador visual de estado ligado/desligado à direita
    let pill_c = Pos2::new(rect.right() - 22.0, rect.center().y);
    let pill_cor = if ativo { crate::COR_ACENTO } else { crate::cor(0x3a, 0x46, 0x54) };
    p.circle_filled(pill_c, 5.0, pill_cor);
    resp
}

/// Linha selecionável com texto alinhado à esquerda.
///
/// Usada na lista de movimentos; `selecionado` replica o visual de seleção do
/// tema para o item carregado, sem checkbox.
pub fn botao_texto(ui: &mut Ui, texto: &str, selecionado: bool, largura: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    resp.widget_info(|| egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), texto));
    let vis = ui.style().interact(&resp).clone();
    let fundo = if selecionado {
        ui.style().visuals.selection.bg_fill
    } else {
        vis.weak_bg_fill
    };
    let borda = if selecionado {
        ui.style().visuals.selection.stroke
    } else {
        vis.bg_stroke
    };
    let p = ui.painter();
    p.rect(rect, vis.corner_radius, fundo, borda, egui::StrokeKind::Inside);
    if resp.has_focus() {
        p.rect_stroke(
            rect.expand(2.0),
            vis.corner_radius,
            egui::Stroke::new(1.5_f32, crate::COR_ACENTO),
            egui::StrokeKind::Inside,
        );
    }
    p.text(
        Pos2::new(rect.left() + 14.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        egui::FontId::proportional(15.0),
        vis.text_color(),
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
        (w.weak_bg_fill, crate::cor(0x6e, 0x7d, 0x8d), w.bg_stroke)
    };
    let p = ui.painter();
    let raio = egui::CornerRadius::same(8);
    p.rect(rect, raio, fundo, borda, egui::StrokeKind::Inside);
    foco_anel(p, rect, raio, &resp);
    desenha(p, Pos2::new(rect.left() + 22.0, rect.center().y), 9.5, icone, frente);
    p.text(
        Pos2::new(rect.left() + 40.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        egui::FontId::proportional(15.0),
        frente,
    );
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
    let raio = ui.style().interact(&resp).corner_radius;
    let p = ui.painter();
    p.rect(rect, raio, fundo, borda, egui::StrokeKind::Inside);
    foco_anel(p, rect, raio, &resp);
    desenha(p, rect.center(), lado * 0.22, icone, frente);
    resp
}

/// Desenha o ícone centrado em `c` com raio `r`.
///
/// `r` é o raio da caixa do desenho (não do traço). A espessura sai de
/// `max(r * 0.28, 1.4)` para o ícone continuar legível nos tamanhos pequenos.
pub fn desenha(p: &egui::Painter, c: Pos2, r: f32, icone: Icone, cor: egui::Color32) {
    let t = (r * 0.28).max(1.4);
    match icone {
        Icone::Play => {
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x - r * 0.55, c.y - r),
                    Pos2::new(c.x - r * 0.55, c.y + r),
                    Pos2::new(c.x + r, c.y),
                ],
                cor,
                Stroke::NONE,
            ));
        }
        Icone::Stop => {
            let q = egui::Rect::from_center_size(c, Vec2::splat(r * 1.7));
            p.rect_filled(q, 2.0, cor);
        }
        Icone::Gravar => {
            p.circle_filled(c, r * 0.85, cor);
        }
        Icone::Travar | Icone::Liberar => {
            // corpo do cadeado
            let corpo = egui::Rect::from_center_size(
                Pos2::new(c.x, c.y + r * 0.32),
                Vec2::new(r * 1.55, r * 1.15),
            );
            p.rect(corpo, 2.5, cor, Stroke::NONE, egui::StrokeKind::Inside);
            // arco (fechado ou aberto)
            let centro_arco = Pos2::new(c.x, c.y - r * 0.22);
            let raio_arco = r * 0.48;
            let (inicio, fim) = if matches!(icone, Icone::Travar) {
                (180.0_f32, 360.0_f32)
            } else {
                (145.0_f32, 330.0_f32)
            };
            let mut pts = Vec::new();
            for i in 0..=14 {
                let a = (inicio + (fim - inicio) * i as f32 / 14.0).to_radians();
                pts.push(Pos2::new(
                    centro_arco.x + raio_arco * a.cos(),
                    centro_arco.y + raio_arco * a.sin(),
                ));
            }
            p.add(egui::Shape::line(pts, Stroke::new(t * 0.95, cor)));
        }
        Icone::Atualizar => {
            let raio = r * 0.82;
            let fim = 245.0_f32;
            let mut pts = Vec::new();
            for i in 0..=22 {
                let a = (-45.0 + (fim + 45.0) * i as f32 / 22.0).to_radians();
                pts.push(Pos2::new(c.x + raio * a.cos(), c.y + raio * a.sin()));
            }
            p.add(egui::Shape::line(pts, Stroke::new(t * 0.95, cor)));
            let a = fim.to_radians();
            let norm = Vec2::new(a.cos(), a.sin());
            let tang = Vec2::new(-a.sin(), a.cos());
            let ponta = c + norm * raio;
            p.add(egui::Shape::convex_polygon(
                vec![
                    ponta + tang * (r * 0.42),
                    ponta - tang * (r * 0.12) + norm * (r * 0.34),
                    ponta - tang * (r * 0.12) - norm * (r * 0.34),
                ],
                cor,
                Stroke::NONE,
            ));
        }
        Icone::Excluir => {
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.7, c.y - r * 0.7),
                    Pos2::new(c.x + r * 0.7, c.y + r * 0.7),
                ],
                Stroke::new(t * 1.1, cor),
            );
            p.line_segment(
                [
                    Pos2::new(c.x + r * 0.7, c.y - r * 0.7),
                    Pos2::new(c.x - r * 0.7, c.y + r * 0.7),
                ],
                Stroke::new(t * 1.1, cor),
            );
        }
        Icone::Conectar | Icone::Desconectar => {
            // raio
            p.add(egui::Shape::convex_polygon(
                vec![
                    Pos2::new(c.x + r * 0.15, c.y - r),
                    Pos2::new(c.x - r * 0.6, c.y + r * 0.15),
                    Pos2::new(c.x - r * 0.05, c.y + r * 0.15),
                    Pos2::new(c.x - r * 0.15, c.y + r),
                    Pos2::new(c.x + r * 0.6, c.y - r * 0.15),
                    Pos2::new(c.x + r * 0.05, c.y - r * 0.15),
                ],
                cor,
                Stroke::NONE,
            ));
            if matches!(icone, Icone::Desconectar) {
                p.line_segment(
                    [
                        Pos2::new(c.x - r, c.y + r),
                        Pos2::new(c.x + r, c.y - r),
                    ],
                    Stroke::new(t * 1.1, cor),
                );
            }
        }
        Icone::Monitor => {
            let tela = egui::Rect::from_center_size(
                Pos2::new(c.x, c.y - r * 0.2),
                Vec2::new(r * 1.9, r * 1.3),
            );
            p.rect_stroke(tela, 2.0, Stroke::new(t * 0.9, cor), egui::StrokeKind::Inside);
            p.line_segment(
                [
                    Pos2::new(c.x, c.y + r * 0.45),
                    Pos2::new(c.x, c.y + r * 0.85),
                ],
                Stroke::new(t * 0.9, cor),
            );
            p.line_segment(
                [
                    Pos2::new(c.x - r * 0.45, c.y + r * 0.9),
                    Pos2::new(c.x + r * 0.45, c.y + r * 0.9),
                ],
                Stroke::new(t * 0.9, cor),
            );
        }
        Icone::Joystick => {
            p.circle_filled(Pos2::new(c.x, c.y - r * 0.45), r * 0.4, cor);
            p.line_segment(
                [
                    Pos2::new(c.x, c.y - r * 0.05),
                    Pos2::new(c.x, c.y + r * 0.35),
                ],
                Stroke::new(t * 0.9, cor),
            );
            let base = egui::Rect::from_center_size(
                Pos2::new(c.x, c.y + r * 0.55),
                Vec2::new(r * 1.6, r * 0.55),
            );
            p.rect(base, 2.0, cor, Stroke::NONE, egui::StrokeKind::Inside);
        }
        Icone::Repetir => {
            let raio = r * 0.80;
            for offset in [0.0_f32, 180.0_f32] {
                let mut pts = Vec::new();
                for i in 0..=12 {
                    let a = (offset + 20.0 + 125.0 * i as f32 / 12.0).to_radians();
                    pts.push(Pos2::new(c.x + raio * a.cos(), c.y + raio * a.sin()));
                }
                p.add(egui::Shape::line(pts, Stroke::new(t * 0.9, cor)));
                let a = (offset + 145.0).to_radians();
                let norm = Vec2::new(a.cos(), a.sin());
                let tang = Vec2::new(-a.sin(), a.cos());
                let ponta = c + norm * raio;
                p.add(egui::Shape::convex_polygon(
                    vec![
                        ponta + tang * (r * 0.36),
                        ponta - tang * (r * 0.08) + norm * (r * 0.28),
                        ponta - tang * (r * 0.08) - norm * (r * 0.28),
                    ],
                    cor,
                    Stroke::NONE,
                ));
            }
        }
        Icone::SetaCima | Icone::SetaBaixo => {
            let dir = if matches!(icone, Icone::SetaCima) { -1.0 } else { 1.0 };
            let pts = vec![
                Pos2::new(c.x - r * 0.7, c.y - dir * r * 0.3),
                Pos2::new(c.x, c.y + dir * r * 0.4),
                Pos2::new(c.x + r * 0.7, c.y - dir * r * 0.3),
            ];
            p.add(egui::Shape::line(pts, Stroke::new(t, cor)));
        }
    }
}
