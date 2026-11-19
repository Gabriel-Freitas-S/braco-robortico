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
//!   "samples": [[0, [90.0, 60.0, 45.0, 10.0]], [50, [92.0, 60.0, 45.0, 10.0]]],
//!   "locks": [[0, false], [1500, true], [3000, false]]
//! }
//! ```
//!
//! Invariantes: `samples` é ordenado por tempo, os tempos são crescentes e cada
//! pose tem quatro ângulos em graus (Base, Ombro, Cotovelo, Garra). `locks` é
//! opcional (arquivos antigos só têm `samples`) e guarda as transições da trava
//! de pressão da garra, também ordenadas por tempo. O nome do arquivo é o nome
//! sanitizado; salvar de novo com o mesmo nome sobrescreve.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Fatores de velocidade disponíveis para reprodução na interface.
pub const VELOCIDADES_REPLAY: [f32; 3] = [0.5, 1.0, 2.0];

/// Um movimento gravado: nome legível + amostras temporizadas.
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Recording {
    pub name: String,
    /// `(ms desde o início, pose em graus)`, ordenado por tempo.
    pub samples: Vec<(u32, [f32; 4])>,
    /// `(ms desde o início, travada)`: transições da trava de pressão da garra.
    ///
    /// O replay reaplica cada transição com `#L1`/`#L0`; sem isso a pressão
    /// contínua não volta e a garra não segura o objeto. `#[serde(default)]`
    /// mantém os arquivos antigos (só pose) válidos: sem eventos, nunca se manda
    /// `#L`.
    #[serde(default)]
    pub locks: Vec<(u32, bool)>,
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

    /// Estado da trava de pressão em `t_ms`: o do último evento até esse
    /// instante. Antes do primeiro evento a garra está liberada (`false`), como
    /// no firmware ao ligar.
    pub fn lock_at(&self, t_ms: f32) -> bool {
        self.locks
            .iter()
            .take_while(|(t, _)| (*t as f32) <= t_ms)
            .last()
            .map(|(_, travada)| *travada)
            .unwrap_or(false)
    }

    /// Caminho do arquivo para `name` em `dir` (nome sanitizado + `.json`).
    ///
    /// É a mesma regra usada por [`Recording::save`]; exposta para a UI poder
    /// detectar colisão antes de gravar em cima de outro movimento.
    pub fn path_for(dir: &Path, name: &str) -> PathBuf {
        dir.join(format!("{}.json", sanitize(name)))
    }

    /// Grava no diretório `dir`, criando-o se preciso.
    ///
    /// O nome do arquivo é o [`sanitize`] do `name` + `.json`; gravar de novo com
    /// o mesmo nome sobrescreve. Devolve o caminho final para o log.
    pub fn save(&self, dir: &Path) -> Result<PathBuf, String> {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
        let path = Self::path_for(dir, &self.name);
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

    /// Remove pausas paradas no início e no fim da gravação e realinha `t = 0`.
    ///
    /// Mantém uma amostra estacionária imediatamente antes do primeiro movimento e
    /// uma imediatamente após o último movimento (limiar de `0.5°`). Se nenhuma
    /// junta tiver se movido, reduz para uma única pose em `t = 0`. As transições
    /// da trava acompanham o corte: o estado em vigor no novo `t = 0` vira um
    /// evento em `0`, e os eventos seguintes são deslocados junto.
    pub fn trim_idle(&mut self) {
        const EPS: f32 = 0.5;
        if self.samples.len() < 2 {
            if let Some(t0) = self.samples.first().map(|s| s.0) {
                let travada = self.lock_at(t0 as f32);
                self.samples[0].0 = 0;
                self.locks = if travada { vec![(0, true)] } else { Vec::new() };
            }
            return;
        }
        let pose_inicial = self.samples[0].1;
        let Some(first_move) = self.samples.iter().position(|(_, p)| pose_diff(*p, pose_inicial) > EPS)
        else {
            let t0 = self.samples[0].0;
            let travada = self.lock_at(t0 as f32);
            self.samples.truncate(1);
            self.samples[0].0 = 0;
            self.locks = if travada { vec![(0, true)] } else { Vec::new() };
            return;
        };
        let start_idx = first_move.saturating_sub(1);
        let pose_final = self.samples[self.samples.len() - 1].1;
        let last_change = self
            .samples
            .iter()
            .rposition(|(_, p)| pose_diff(*p, pose_final) > EPS)
            .unwrap_or(start_idx);
        let end_idx = (last_change + 1).min(self.samples.len() - 1).max(start_idx);
        self.samples = self.samples[start_idx..=end_idx].to_vec();
        let t0 = self.samples[0].0;
        for (t, _) in &mut self.samples {
            *t = t.saturating_sub(t0);
        }
        self.rebase_locks(t0);
    }

    /// Desloca as transições da trava para o novo `t = 0` depois do corte.
    ///
    /// O estado em `t0` vira um evento em `0`, para o replay começar com a garra
    /// na mesma condição da gravação; eventos que passariam do fim ficam no fim.
    fn rebase_locks(&mut self, t0: u32) {
        let travada = self.lock_at(t0 as f32);
        self.locks.retain(|(t, _)| *t > t0);
        let fim = self.duration_ms();
        for (t, _) in &mut self.locks {
            *t = t.saturating_sub(t0).min(fim);
        }
        if travada {
            self.locks.insert(0, (0, true));
        }
    }
}

/// Maior diferença absoluta (em graus) entre duas poses de 4 juntas.
fn pose_diff(a: [f32; 4], b: [f32; 4]) -> f32 {
    (0..4).map(|i| (a[i] - b[i]).abs()).fold(0.0_f32, f32::max)
}

/// Estado de execução de um replay com pré-posicionamento, pausa e velocidade.
#[derive(Clone, Debug)]
pub struct PlayState {
    pub rec: Recording,
    /// Posição atual na linha do tempo da gravação, em ms (`0..=duration_ms`).
    pub pos_ms: f32,
    /// `true` enquanto o braço ainda está viajando até a pose inicial (`samples[0]`).
    pub preparando: bool,
    /// Tempo acumulado (ms) aguardando o pré-posicionamento (timeout de segurança).
    pub espera_ms: f32,
    /// `true` quando pausado pelo usuário.
    pub pausado: bool,
}

impl PlayState {
    /// Inicia um replay em `0 ms`, aguardando o braço alcançar a pose inicial.
    pub fn new(rec: Recording) -> Self {
        Self {
            rec,
            pos_ms: 0.0,
            preparando: true,
            espera_ms: 0.0,
            pausado: false,
        }
    }

    /// Verifica se todas as juntas estão a até `2.5°` do alvo efetivo.
    ///
    /// Respeita os limites mecânicos e a trava da garra (que impede abertura
    /// além de `live[3]`), para que o pré-posicionamento não trave esperando um
    /// movimento bloqueado pelo firmware.
    pub fn chegou(live: [f32; 4], alvo: [f32; 4], limits: [[f32; 2]; 4], garra_travada: bool) -> bool {
        for i in 0..4 {
            let mut esp = alvo[i].clamp(limits[i][0], limits[i][1]);
            if i == 3 && garra_travada {
                esp = esp.min(live[3]);
            }
            if (live[i] - esp).abs() > 2.5 {
                return false;
            }
        }
        true
    }

    /// Avança o replay por `dt_ms` no fator `vel` e devolve o próximo alvo `#P`.
    ///
    /// Devolve `None` quando o movimento termina sem `looping` (ou se a gravação
    /// estiver vazia). Durante o pré-posicionamento, segura `pos_ms = 0` até o
    /// braço alcançar `samples[0]` ou atingir o timeout de `4500 ms`.
    pub fn step(
        &mut self,
        dt_ms: f32,
        vel: f32,
        looping: bool,
        live: [f32; 4],
        limits: [[f32; 2]; 4],
        garra_travada: bool,
    ) -> Option<[f32; 4]> {
        let p0 = self.rec.sample_at(0.0)?;
        if self.pausado {
            return if self.preparando {
                Some(live)
            } else {
                self.rec.sample_at(self.pos_ms)
            };
        }
        if self.preparando {
            self.espera_ms += dt_ms;
            if Self::chegou(live, p0, limits, garra_travada) || self.espera_ms >= 4500.0 {
                self.preparando = false;
                self.espera_ms = 0.0;
            }
            return Some(p0);
        }

        self.pos_ms += dt_ms * vel;
        let dur = self.rec.duration_ms() as f32;
        let pose = self.rec.sample_at(self.pos_ms)?;
        if self.pos_ms >= dur {
            if looping {
                self.pos_ms = 0.0;
                self.preparando = true;
                self.espera_ms = 0.0;
                return Some(pose);
            }
            return None;
        }
        Some(pose)
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
            locks: vec![],
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
        let mut r = rec();
        r.locks = vec![(0, false), (500, true)];
        let path = r.save(&dir).unwrap();
        let lido = Recording::load(&path).unwrap();
        assert_eq!(lido.samples.len(), 2);
        assert_eq!(lido.name, "t");
        assert_eq!(lido.locks, r.locks);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn caminho_usa_nome_sanitizado() {
        let path = Recording::path_for(Path::new("/mov"), "pega peça");
        assert_eq!(path.file_name().unwrap(), "pega_peça.json");
    }

    #[test]
    fn trava_acompanha_a_linha_do_tempo() {
        let mut r = rec();
        r.locks = vec![(100, true), (900, false)];
        assert!(!r.lock_at(0.0));
        assert!(!r.lock_at(99.0));
        assert!(r.lock_at(100.0));
        assert!(r.lock_at(500.0));
        assert!(!r.lock_at(900.0));
        assert!(!r.lock_at(5_000.0));
    }

    #[test]
    fn arquivo_antigo_sem_trava_carrega() {
        let json = r#"{"name":"velho","samples":[[0,[90.0,60.0,45.0,10.0]]]}"#;
        let r: Recording = serde_json::from_str(json).unwrap();
        assert!(r.locks.is_empty());
        assert!(!r.lock_at(0.0));
    }

    #[test]
    fn corta_silencio_nas_pontas() {
        let mut r = Recording {
            name: "pausas".into(),
            samples: vec![
                (0, [90.0, 60.0, 45.0, 10.0]),
                (250, [90.0, 60.0, 45.0, 10.0]),
                (500, [90.0, 60.0, 45.0, 10.0]),
                (1000, [100.0, 60.0, 45.0, 10.0]),
                (1500, [110.0, 60.0, 45.0, 10.0]),
                (1750, [110.0, 60.0, 45.0, 10.0]),
                (2000, [110.0, 60.0, 45.0, 10.0]),
            ],
            locks: vec![],
        };
        r.trim_idle();
        assert_eq!(r.samples.first().unwrap().0, 0);
        assert_eq!(r.duration_ms(), 1000);
        assert_eq!(r.samples.len(), 3);
        assert_eq!(r.samples[0].1[0], 90.0);
        assert_eq!(r.samples[2].1[0], 110.0);

        let mut estatica = Recording {
            name: "parada".into(),
            samples: vec![
                (100, [90.0, 60.0, 45.0, 10.0]),
                (500, [90.2, 60.0, 45.0, 10.0]),
            ],
            locks: vec![],
        };
        estatica.trim_idle();
        assert_eq!(estatica.samples.len(), 1);
        assert_eq!(estatica.samples[0].0, 0);
    }

    #[test]
    fn corte_preserva_a_trava_em_vigor() {
        let mut r = Recording {
            name: "trava".into(),
            samples: vec![
                (0, [90.0, 60.0, 45.0, 10.0]),
                (500, [90.0, 60.0, 45.0, 10.0]),
                (1000, [110.0, 60.0, 45.0, 10.0]),
                (1500, [110.0, 60.0, 45.0, 10.0]),
            ],
            locks: vec![(0, false), (200, true), (800, false)],
        };
        r.trim_idle();
        // O corte começa em 500 ms: a garra já estava travada ali.
        assert_eq!(r.samples.first().unwrap().0, 0);
        assert_eq!(r.duration_ms(), 500);
        assert!(r.lock_at(0.0));
        assert!(r.lock_at(299.0));
        assert!(!r.lock_at(300.0));
    }

    #[test]
    fn replay_pre_posiciona_pausa_e_escala_velocidade() {
        let lim = [[0.0, 180.0], [0.0, 180.0], [0.0, 180.0], [0.0, 90.0]];
        let r = Recording {
            name: "t".into(),
            samples: vec![
                (0, [90.0, 60.0, 45.0, 10.0]),
                (1000, [110.0, 60.0, 45.0, 10.0]),
            ],
            locks: vec![],
        };
        let mut play = PlayState::new(r.clone());

        // Braço longe da pose inicial: permanece preparando em pos_ms = 0
        let longe = [20.0, 60.0, 45.0, 10.0];
        let p = play.step(50.0, 1.0, false, longe, lim, false).unwrap();
        assert_eq!(p[0], 90.0);
        assert!(play.preparando);
        assert_eq!(play.pos_ms, 0.0);

        // Braço chega perto de p0: libera o cronômetro
        let perto = [89.0, 60.0, 45.0, 10.0];
        let p = play.step(50.0, 1.0, false, perto, lim, false).unwrap();
        assert_eq!(p[0], 90.0);
        assert!(!play.preparando);
        assert_eq!(play.pos_ms, 0.0);

        // Escala de velocidade 2x: dt=100ms avança 200ms
        let p = play.step(100.0, 2.0, false, perto, lim, false).unwrap();
        assert!((play.pos_ms - 200.0).abs() < 1e-4);
        assert!((p[0] - 94.0).abs() < 1e-4);

        // Pausado: pos_ms não avança
        play.pausado = true;
        let p_pausa = play.step(100.0, 2.0, false, perto, lim, false).unwrap();
        assert!((play.pos_ms - 200.0).abs() < 1e-4);
        assert!((p_pausa[0] - 94.0).abs() < 1e-4);
        play.pausado = false;

        // Com looping = true ao passar da duração: reinicia pos_ms e volta a preparar
        let mut play_loop = PlayState { pos_ms: 950.0, preparando: false, ..PlayState::new(r) };
        let p_fim = play_loop.step(100.0, 1.0, true, perto, lim, false).unwrap();
        assert_eq!(p_fim[0], 110.0);
        assert_eq!(play_loop.pos_ms, 0.0);
        assert!(play_loop.preparando);

        // Sem looping ao passar da duração: retorna None
        play_loop.preparando = false;
        play_loop.pos_ms = 950.0;
        assert!(play_loop.step(100.0, 1.0, false, perto, lim, false).is_none());
    }
}
