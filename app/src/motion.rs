//! Gravação e reprodução de movimentos.
//!
//! A gravação é uma sequência de poses com carimbo de tempo em ms, na taxa em
//! que o firmware manda status (20 Hz). O replay interpola entre amostras, então
//! a taxa do app não precisa casar com a da gravação.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Recording {
    pub name: String,
    /// (ms desde o início, pose em graus). Ordenado por tempo.
    pub samples: Vec<(u32, [f32; 4])>,
}

impl Recording {
    pub fn duration_ms(&self) -> u32 {
        self.samples.last().map(|s| s.0).unwrap_or(0)
    }

    /// Pose interpolada no instante `t_ms`.
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

    pub fn save(&self, dir: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{}.json", sanitize(&self.name)));
        let json = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(&path, json).map_err(|e| e.to_string())?;
        Ok(path)
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let txt = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        serde_json::from_str(&txt).map_err(|e| e.to_string())
    }

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
