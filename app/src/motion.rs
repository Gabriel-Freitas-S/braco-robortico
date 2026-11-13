//! Gravação e reprodução de movimentos.
//!
//! A gravação é uma sequência de poses com carimbo de tempo em ms, na taxa em
//! que o firmware manda status (20 Hz). O replay interpola entre amostras, então
//! a taxa do app não precisa casar com a da gravação.
//!
//! # Formato em disco
//!
//! Um arquivo JSON por movimento, no diretório devolvido por `dir_movimentos`
//! em `main` (`~/.local/share/robocontrol/movimentos` por padrão):
//!
//! ```json
//! {
//!   "name": "pega_peca",
//!   "samples": [[0, [90.0, 60.0, 45.0, 10.0]], [50, [92.0, 60.0, 45.0, 10.0]]]
//! }
//! ```
//!
//! Invariantes: `samples` é ordenado por tempo, os tempos são crescentes e cada
//! pose tem quatro ângulos em graus (Base, Ombro, Cotovelo, Garra). O nome do
//! arquivo é o nome sanitizado; salvar de novo com o mesmo nome sobrescreve.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Um movimento gravado: nome legível + amostras temporizadas.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Recording {
    pub name: String,
    /// `(ms desde o início, pose em graus)`, ordenado por tempo.
    pub samples: Vec<(u32, [f32; 4])>,
}

impl Recording {
    /// Duração do movimento: carimbo da última amostra.
    ///
    /// É o tempo *gravado*, não o tempo de parede desde o início da captura.
    pub fn duration_ms(&self) -> u32 {
        self.samples.last().map(|s| s.0).unwrap_or(0)
    }

    /// Pose interpolada linearmente no instante `t_ms`.
    ///
    /// Antes da primeira e depois da última amostra, segura a pose da ponta
    /// (não estende o movimento). Posições intermediárias são achadas por busca
    /// binária (`partition_point`), então o custo é O(log n) por consulta.
    /// Devolve `None` apenas para gravação vazia; amostras com o mesmo tempo não
    /// dividem por zero — o intervalo degenerado usa `k = 0`.
    pub fn sample_at(&self, t_ms: f32) -> Option<[f32; 4]> {
        let first = self.samples.first()?;
        let last = self.samples.last()?;
        if t_ms <= first.0 as f32 {
            return Some(first.1);
        }
        if t_ms >= last.0 as f32 {
            return Some(last.1);
        }
        let idx = self.samples.partition_point(|s| (s.0 as f32) <= t_ms);
        let (t1, p1) = self.samples[idx];
        let (t0, p0) = self.samples[idx - 1];
        let span = (t1 - t0) as f32;
        let k = if span > 0.0 { (t_ms - t0 as f32) / span } else { 0.0 };
        let mut out = [0.0; 4];
        for i in 0..4 {
            out[i] = p0[i] + (p1[i] - p0[i]) * k;
        }
        Some(out)
    }

    /// Grava no diretório `dir`, criando-o se preciso.
    ///
    /// O nome do arquivo é o [`sanitize`] do `name` + `.json`; gravar de novo com
    /// o mesmo nome sobrescreve. Devolve o caminho final para o log.
    pub fn save(&self, dir: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.json", sanitize(&self.name)));
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    /// Remove o arquivo do movimento. Não mexe no estado da UI.
    pub fn delete(path: &Path) -> Result<(), String> {
        std::fs::remove_file(path).map_err(|e| e.to_string())
    }

    /// Lê e valida o JSON. Erros de IO e de formato viram texto para o log.
    pub fn load(path: &Path) -> Result<Self, String> {
        let txt = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&txt).map_err(|e| e.to_string())
    }

    /// Lista os `*.json` de `dir` em ordem de caminho.
    ///
    /// Diretório ausente vira lista vazia: a primeira execução não é erro.
    pub fn list(dir: &Path) -> Vec<PathBuf> {
        let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .filter(|p| p.extension().is_some_and(|x| x == "json"))
                    .collect()
            })
            .unwrap_or_default();
        out.sort();
        out
    }
}

/// Nome de arquivo seguro: mantém letras/números Unicode, `-` e `_` e troca o
/// resto por `_`. Nome vazio vira `"movimento"` para nunca gerar só `.json`.
fn sanitize(name: &str) -> String {
    let limpo: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if limpo.is_empty() {
        "movimento".into()
    } else {
        limpo
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec() -> Recording {
        Recording {
            name: "t".into(),
            samples: vec![
                (0, [90.0, 60.0, 45.0, 10.0]),
                (1000, [110.0, 60.0, 45.0, 10.0]),
            ],
        }
    }

    #[test]
    fn interpola_no_meio() {
        let p = rec().sample_at(500.0).unwrap();
        assert!((p[0] - 100.0).abs() < 1e-4, "{p:?}");
    }

    #[test]
    fn segura_nas_pontas() {
        let r = rec();
        assert_eq!(r.sample_at(-50.0).unwrap()[0], 90.0);
        assert_eq!(r.sample_at(5_000.0).unwrap()[0], 110.0);
        assert_eq!(r.duration_ms(), 1000);
    }

    #[test]
    fn grava_e_le() {
        let dir = std::env::temp_dir().join("robocontrol_teste");
        let path = rec().save(&dir).unwrap();
        let lido = Recording::load(&path).unwrap();
        assert_eq!(lido.samples.len(), 2);
        assert_eq!(lido.name, "t");
        let _ = std::fs::remove_file(path);
    }
}
