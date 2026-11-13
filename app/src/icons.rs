//! Ícones vetoriais desenhados na mão.
//!
//! Não uso glifo de fonte porque o conjunto de símbolos que as fontes padrão do
//! egui cobrem varia (e o que falta vira "quadradinho"). Desenhando com a
//! painter, o ícone existe sempre e acompanha a cor do tema.

use egui::{Pos2, Response, Sense, Stroke, Ui, Vec2};

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
}

/// Botão com ícone à esquerda e texto, no tema do app.
pub fn botao(ui: &mut Ui, icone: Icone, texto: &str, largura: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let vis = ui.style().interact(&resp).clone();
    let habilitado = ui.is_enabled();

    let (fundo, borda, frente) = if habilitado {
        (vis.weak_bg_fill, vis.bg_stroke, vis.text_color())
    } else {
        let w = ui.style().visuals.widgets.noninteractive;
        (w.weak_bg_fill, w.bg_stroke, ui.style().visuals.weak_text_color())
    };

    let p = ui.painter();
    p.rect(rect, vis.corner_radius, fundo, borda, egui::StrokeKind::Inside);

    let centro_icone = Pos2::new(rect.left() + 18.0, rect.center().y);
    desenha(p, centro_icone, 8.5, icone, frente);
    p.text(
        Pos2::new(rect.left() + 34.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        egui::FontId::proportional(14.5),
        frente,
    );
    resp
}

/// Botão com cores fixas (STOP, travar garra): não segue o tema.
pub fn botao_cor(
    ui: &mut Ui,
    icone: Icone,
    texto: &str,
    largura: f32,
    fundo: egui::Color32,
    frente: egui::Color32,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(largura, crate::ALVO), Sense::click());
    let borda = Stroke::new(
        1.0_f32,
        if resp.hovered() {
            frente.gamma_multiply(0.7)
        } else {
            fundo.gamma_multiply(1.8)
        },
    );
    let p = ui.painter();
    p.rect(rect, egui::CornerRadius::same(8), fundo, borda, egui::StrokeKind::Inside);
    desenha(p, Pos2::new(rect.left() + 18.0, rect.center().y), 8.5, icone, frente);
    p.text(
        Pos2::new(rect.left() + 34.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        texto,
        egui::FontId::proportional(14.5),
        frente,
    );
    resp
}

/// Botão só de ícone (listas, ações de linha).
pub fn botao_quadrado(ui: &mut Ui, icone: Icone, lado: f32) -> Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::new(lado, crate::ALVO), Sense::click());
    let vis = ui.style().interact(&resp).clone();
    let habilitado = ui.is_enabled();
    let (fundo, borda, frente) = if habilitado {
        (vis.weak_bg_fill, vis.bg_stroke, vis.text_color())
    } else {
        let w = ui.style().visuals.widgets.noninteractive;
        (w.weak_bg_fill, w.bg_stroke, ui.style().visuals.weak_text_color())
    };
    let p = ui.painter();
    p.rect(rect, vis.corner_radius, fundo, borda, egui::StrokeKind::Inside);
    desenha(p, rect.center(), lado * 0.22, icone, frente);
    resp
}

/// Desenha o ícone centrado em `c` com raio `r`.
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
                Pos2::new(c.x, c.y + r * 0.35),
                Vec2::new(r * 1.7, r * 1.3),
            );
            p.rect(corpo, 2.0, cor, Stroke::NONE, egui::StrokeKind::Inside);
            // arco (fechado ou aberto)
            let centro_arco = Pos2::new(c.x, c.y - r * 0.25);
            let raio_arco = r * 0.55;
            let inicio = if matches!(icone, Icone::Travar) { 180.0 } else { 135.0 };
            let mut pts = Vec::new();
            for i in 0..=12 {
                let a = (inicio + (180.0 * i as f32 / 12.0)).to_radians();
                pts.push(Pos2::new(
                    centro_arco.x + raio_arco * a.cos(),
                    centro_arco.y + raio_arco * a.sin(),
                ));
            }
            p.add(egui::Shape::line(pts, Stroke::new(t * 0.9, cor)));
        }
        Icone::Atualizar => {
            let mut pts = Vec::new();
            for i in 0..=22 {
                let a = (-60.0 + 300.0 * i as f32 / 22.0).to_radians();
                pts.push(Pos2::new(c.x + r * 0.85 * a.cos(), c.y + r * 0.85 * a.sin()));
            }
            p.add(egui::Shape::line(pts, Stroke::new(t * 0.9, cor)));
            // ponta da seta no fim do arco
            let a = (240.0_f32).to_radians();
            let ponta = Pos2::new(c.x + r * 0.85 * a.cos(), c.y + r * 0.85 * a.sin());
            p.add(egui::Shape::convex_polygon(
                vec![
                    ponta + Vec2::new(r * 0.45, -r * 0.1),
                    ponta + Vec2::new(r * 0.1, r * 0.42),
                    ponta + Vec2::new(-r * 0.05, -r * 0.18),
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
    }
}
