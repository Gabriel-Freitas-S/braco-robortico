//! Porta serial em thread de leitura + escrita síncrona.
//!
//! Escrever direto da thread da UI é barato: um comando tem ~10 bytes, 1 ms a
//! 115200. Quem precisa de thread é a leitura, que fica bloqueada esperando.
//!
//! # Modelo de execução
//!
//! - [`Link::open`] abre a porta, clona o handle e sobe **uma** thread leitora.
//! - A thread acumula bytes até `\n`, interpreta com [`protocol::parse_line`] e
//!   entrega [`Event`]s por um canal `mpsc`.
//! - A UI chama [`Link::drain`] a cada tick (20 Hz), sem bloquear; a thread
//!   nunca toca em estado da UI.
//! - [`Link::send`] é *best-effort*: erro de escrita é descartado. Se o cabo
//!   cair, a thread leitora envia [`Event::Error`] e o app desconecta.
//!
//! O timeout de leitura (200 ms) não é timeout de protocolo: existe só para a
//! thread conseguir sair quando o canal fecha.

use crate::protocol::{self, Msg};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

/// Algo que chegou da placa e precisa ser tratado pela UI.
pub enum Event {
    Msg(Msg),
    /// Porta caiu ou o cabo foi removido. Depois deste evento o link morre.
    Error(String),
}

/// Handle da serial: escrita síncrona + eventos vindos da thread leitora.
pub struct Link {
    port: Box<dyn serialport::SerialPort>,
    rx: Receiver<Event>,
}

impl Link {
    /// Abre `path` a 115200 baud e sobe a thread de leitura.
    ///
    /// A thread remonta linhas por `\n`. Se o acumulador passar de 512 bytes sem
    /// newline, o buffer é descartado: significa protocolo desalinhado (ruído ou
    /// firmware reiniciando) e insistir só atrasaria a próxima linha boa.
    pub fn open(path: &str) -> Result<Self, String> {
        let port = serialport::new(path, 115_200)
            .timeout(Duration::from_millis(200))
            .open()
            .map_err(|e| format!("{path}: {e}"))?;
        let mut reader = port.try_clone().map_err(|e| e.to_string())?;
        let (tx, rx) = channel();

        std::thread::spawn(move || {
            let mut buf = [0u8; 256];
            let mut acc: Vec<u8> = Vec::new();
            loop {
                match reader.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        acc.extend_from_slice(&buf[..n]);
                        while let Some(pos) = acc.iter().position(|&b| b == b'\n') {
                            let line: Vec<u8> = acc.drain(..=pos).collect();
                            let text = String::from_utf8_lossy(&line[..line.len() - 1]);
                            if let Some(msg) = protocol::parse_line(&text) {
                                if tx.send(Event::Msg(msg)).is_err() {
                                    return;
                                }
                            }
                        }
                        // linha sem newline há muito tempo: protocolo desalinhado
                        if acc.len() > 512 {
                            acc.clear();
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {}
                    Err(e) => {
                        let _ = tx.send(Event::Error(e.to_string()));
                        return;
                    }
                }
            }
            let _ = tx.send(Event::Error("porta fechada".into()));
        });

        Ok(Self { port, rx })
    }

    /// Escreve um comando já formatado (incluindo `\n`). Erros são ignorados:
    /// o pior caso é repetir o alvo no próximo tick.
    pub fn send(&mut self, cmd: &str) {
        let _ = self.port.write_all(cmd.as_bytes());
    }

    /// Drena tudo que chegou, sem bloquear. Devolve `true` enquanto a porta
    /// estiver viva; `false` quando a thread leitora morreu ou reportou erro.
    ///
    /// O chamador deve tratar os eventos antes de assumir que o link continua
    /// utilizável — um `false` normalmente vem junto com [`Event::Error`].
    pub fn drain(&self, out: &mut Vec<Event>) -> bool {
        loop {
            match self.rx.try_recv() {
                Ok(ev) => {
                    let morreu = matches!(ev, Event::Error(_));
                    out.push(ev);
                    if morreu {
                        return false;
                    }
                }
                Err(TryRecvError::Empty) => return true,
                Err(TryRecvError::Disconnected) => return false,
            }
        }
    }
}
