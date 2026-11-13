//! Protocolo serial do braço 4-DOF (firmware v1.3+).
//!
//! Enlace USB CDC a **115200 baud, 8N1**, linhas terminadas em `\n`.
//! O PC nunca gera PWM: ele só propõe ALVOS. A rampa, os limites mecânicos e o
//! hold continuam no firmware, então um comando perdido não vira tranco.
//!
//! # Convenções
//!
//! - Juntas sempre na ordem **Base, Ombro, Cotovelo, Garra**, em graus inteiros.
//! - Sticks virtuais em `-100..=100` por eixo; o eixo Y chega invertido em
//!   relação à tela (ver [`cmd_sticks`]).
//! - Comandos começam com `#`; mensagens da placa começam com `S` ou `B`.
//!
//! # Comandos (PC → placa)
//!
//! | Item | Bytes | Efeito |
//! |---|---|---|
//! | [`CMD_ASSUME`] | `#M\n` | adota o controle do PC |
//! | [`CMD_RELEASE`] | `#J\n` | devolve o controle ao joystick |
//! | [`CMD_STOP`] | `#H\n` | segura as juntas onde estão |
//! | [`CMD_DIAG`] | `#B\n` | pede o diagnóstico |
//! | [`cmd_lock`] | `#L1\n` / `#L0\n` | trava (só aperta) / libera a garra |
//! | [`cmd_targets`] | `#P...` | alvos absolutos por junta |
//! | [`cmd_sticks`] | `#V...` | sticks virtuais |
//!
//! # Mensagens (placa → PC)
//!
//! | Prefixo | Tipo | Formato |
//! |---|---|---|
//! | `S` | [`Status`] | `S<pose0>;<pose1>;<pose2>;<pose3>;<pc_mode>;<trava>` |
//! | `B` | [`Diag`] | `B<versão>;boots=<n>;limb=<min>;<max>;limo=...;limc=...;limg=...` |
//! | outro | [`Msg::Text`] | banner, ajuda ou debug |
//!
//! O campo `trava` do status é opcional: firmware anterior não o enviava e o
//! parser assume `false`.

pub const CMD_ASSUME: &str = "#M\n";
pub const CMD_RELEASE: &str = "#J\n";
pub const CMD_STOP: &str = "#H\n";

/// Trava de pressão da garra: com ela ativa a garra só aperta, nunca solta.
///
/// É o mesmo estado do botão físico D4 no firmware, então app e bancada nunca
/// discordam. Enquanto travada, nem `#P` nem `#V` conseguem abrir a garra.
pub fn cmd_lock(travar: bool) -> &'static str {
    if travar { "#L1\n" } else { "#L0\n" }
}

/// Pedido de diagnóstico. A placa responde com [`Diag`] em [`Msg::Diag`].
pub const CMD_DIAG: &str = "#B\n";

/// `#P90;60;45;20\n` — alvos absolutos em graus, na ordem
/// Base, Ombro, Cotovelo, Garra.
///
/// O firmware interpola até esses alvos com a própria rampa; valores são
/// arredondados aqui porque o protocolo transporta inteiros.
pub fn cmd_targets(pose: [f32; 4]) -> String {
    format!(
        "#P{};{};{};{}\n",
        pose[0].round() as i32,
        pose[1].round() as i32,
        pose[2].round() as i32,
        pose[3].round() as i32
    )
}

/// `#V<lx>;<ly>;<rx>;<ry>\n` — sticks virtuais, `-100..=100` por eixo.
///
/// Entram no firmware pelo mesmo caminho dos joysticks físicos, então respeitam
/// a mesma rampa e os mesmos limites. Lx/Ly são o pad esquerdo
/// (Base/Cotovelo) e Rx/Ry o direito (Garra/Ombro); Y positivo é *para cima*,
/// por isso o chamador nega o Y da tela. Valores fora da faixa são grampeados.
pub fn cmd_sticks(v: [i32; 4]) -> String {
    format!("#V{};{};{};{}\n", v[0].clamp(-100, 100), v[1].clamp(-100, 100), v[2].clamp(-100, 100), v[3].clamp(-100, 100))
}

/// Estado periódico enviado pelo firmware (~20 Hz).
#[derive(Debug, Clone, PartialEq)]
pub struct Status {
    /// Pose medida, em graus: Base, Ombro, Cotovelo, Garra.
    pub pose: [f32; 4],
    /// `true` quando o firmware está aceitando alvos do PC.
    pub pc_mode: bool,
    /// `true` quando a garra está travada com pressão (botão D4 ou app).
    pub trava: bool,
}

/// Resposta de [`CMD_DIAG`]: identidade, contador de boots e limites mecânicos.
///
/// `boots` só cresce enquanto a placa está ligada; um salto inesperado entre
/// duas leituras indica reinício (o app usa isso para avisar sobre alimentação).
#[derive(Debug, Clone, PartialEq)]
pub struct Diag {
    pub version: String,
    pub boots: u32,
    /// `[[min, max]; 4]` na ordem Base, Ombro, Cotovelo, Garra.
    pub limits: [[f32; 2]; 4],
}

/// Uma linha já interpretada da serial.
#[derive(Debug, Clone, PartialEq)]
pub enum Msg {
    Status(Status),
    Diag(Diag),
    /// Texto livre do firmware (banner, ajuda, debug).
    Text(String),
}

/// Converte uma linha recebida em mensagem. Nunca entra em pânico com lixo.
///
/// Linha vazia (ou só `\r\n`) vira `None`; prefixo `S`/`B` malformado também é
/// descartado; qualquer outra coisa vira [`Msg::Text`] para aparecer no log.
pub fn parse_line(line: &str) -> Option<Msg> {
    let line = line.trim_end_matches(['\r', '\n']).trim();
    if line.is_empty() {
        return None;
    }
    match line.as_bytes()[0] {
        b'S' => parse_status(line).map(Msg::Status),
        b'B' => parse_diag(line).map(Msg::Diag),
        _ => Some(Msg::Text(line.to_string())),
    }
}

/// `S<pose0>;...;<pose3>;<pc_mode>[;<trava>]`.
///
/// Falha (`None`) se faltar qualquer um dos cinco campos obrigatórios ou se um
/// número não parsear; campos extras são ignorados.
fn parse_status(line: &str) -> Option<Status> {
    let mut it = line[1..].split(';');
    let mut pose = [0.0; 4];
    for p in pose.iter_mut() {
        *p = it.next()?.parse().ok()?;
    }
    let pc_mode = it.next()?.trim() == "1";
    // Campo da trava é opcional: firmware anterior não mandava.
    let trava = it.next().map(|f| f.trim() == "1").unwrap_or(false);
    Some(Status { pose, pc_mode, trava })
}

/// `B<versão>;boots=<n>;limb=<min>;<max>;limo=...;limc=...;limg=...`.
///
/// Os limites vêm em pares chave/valor (`limb=5;175`), uma chave por junta;
/// chaves desconhecidas são ignoradas para permitir firmware mais novo.
fn parse_diag(line: &str) -> Option<Diag> {
    let parts: Vec<&str> = line[1..].split(';').collect();
    let version = parts.first()?.to_string();
    let mut boots = 0;
    let mut limits = [[0.0, 180.0]; 4];
    for i in 0..parts.len() {
        let part = parts[i];
        if let Some(v) = part.strip_prefix("boots=") {
            boots = v.parse().ok()?;
            continue;
        }
        let Some((key, min)) = part.split_once('=') else { continue };
        let idx = match key {
            "limb" => 0,
            "limo" => 1,
            "limc" => 2,
            "limg" => 3,
            _ => continue,
        };
        limits[idx] = [min.parse().ok()?, parts.get(i + 1)?.parse().ok()?];
    }
    Some(Diag { version, boots, limits })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_do_firmware() {
        let msg = parse_line("S95;63;154;10;1;0\n").unwrap();
        assert_eq!(
            msg,
            Msg::Status(Status { pose: [95.0, 63.0, 154.0, 10.0], pc_mode: true, trava: false })
        );
        // firmware antigo, sem o campo da trava
        let velho = parse_line("S95;63;154;10;0").unwrap();
        assert_eq!(velho, Msg::Status(Status { pose: [95.0, 63.0, 154.0, 10.0], pc_mode: false, trava: false }));
    }

    #[test]
    fn diag_do_firmware() {
        let msg = parse_line("B1.3.0;boots=7;limb=5;175;limo=0;180;limc=0;180;limg=0;90").unwrap();
        match msg {
            Msg::Diag(d) => {
                assert_eq!(d.version, "1.3.0");
                assert_eq!(d.boots, 7);
                assert_eq!(d.limits[0], [5.0, 175.0]);
                assert_eq!(d.limits[3], [0.0, 90.0]);
            }
            other => panic!("esperava diag, veio {other:?}"),
        }
    }

    #[test]
    fn texto_livre_e_lixo() {
        assert_eq!(
            parse_line("firmware v1.3.0 boots=2"),
            Some(Msg::Text("firmware v1.3.0 boots=2".into()))
        );
        assert_eq!(parse_line(""), None);
        assert_eq!(parse_line("S95;63"), None);
    }

    #[test]
    fn sticks_limitam() {
        assert_eq!(cmd_sticks([120, -150, 0, 37]), "#V100;-100;0;37\n");
    }

    #[test]
    fn alvos_arredondam() {
        assert_eq!(cmd_targets([90.4, 59.6, 45.0, 20.5]), "#P90;60;45;21\n");
    }
}
