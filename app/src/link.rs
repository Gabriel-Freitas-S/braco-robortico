//! Porta serial em thread de leitura + escrita síncrona.
//!
//! Escrever direto da thread da UI é barato: um comando tem ~10 bytes, 1 ms a
//! 115200. Quem precisa de thread é a leitura, que fica bloqueada esperando.

use crate::protocol::{self, Msg};
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, TryRecvError};
use std::time::Duration;

pub enum Event {
    Msg(Msg),
    /// Porta caiu ou o cabo foi removido.
    Error(String),
}

pub struct Link {
    port: Box<dyn serialport::SerialPort>,
    rx: Receiver<Event>,
}

impl Link {
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

    pub fn send(&mut self, cmd: &str) {
        let _ = self.port.write_all(cmd.as_bytes());
    }

    /// Drena tudo que chegou. Devolve `true` enquanto a porta estiver viva.
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
