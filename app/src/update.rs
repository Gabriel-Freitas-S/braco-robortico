//! Checagem das versões publicadas: app (GitHub Releases) e firmware (`main`).
//!
//! Roda em uma thread por checagem, entrega o resultado por `mpsc` e morre —
//! o mesmo modelo de [`crate::link`]. A UI nunca espera rede: se não houver
//! conexão, o resultado chega vazio e o app segue com o firmware embutido.
//!
//! # Por que não a API do GitHub
//!
//! A API sem token tem teto de 60 requisições por hora **por IP** e devolve 403
//! quando estoura (comum em laboratório atrás de NAT). Aqui a versão do app vem
//! do redirect de `github.com/.../releases/latest` — HTML, sem cota — e o
//! firmware vem de `raw.githubusercontent.com`, também sem cota.
//!
//! # Segurança
//!
//! - Só HTTPS e só URLs fixas neste módulo; nenhuma resposta vira URL. A tag do
//!   app passa por [`firmware::valida_versao`] antes de montar o link da
//!   release com [`url_release`], e o destino final de cada redirect é
//!   conferido contra o prefixo esperado.
//! - Com timeouts e teto de leitura: host que não responde ou resposta gigante
//!   não travam a UI nem estouram a memória.
//! - Nada é executado: o firmware é texto validado por [`firmware::validar`] e
//!   salvo como arquivo; uma versão nova do app só abre a página da release no
//!   navegador. Não há auto-update.
//! - Falha vira linha no log, sem popup — a checagem é um extra, não um
//!   pré-requisito.
//!
//! # Limites conhecidos
//!
//! O firmware é lido de `main`. `shortcut: main é a versão publicada neste
//! projeto; se deixar de ser, troque a URL por tags fw-v*`. Se o release mais
//! recente do repositório for de firmware (tag fora de `app-v`), a versão do
//! app fica desconhecida — de propósito, para não anunciar update falso.

use crate::firmware;
use std::io::Read;
use std::sync::mpsc::{self, Receiver};
use std::time::Duration;

/// Repositório de onde saem binários e firmware.
pub const REPO: &str = "Gabriel-Freitas-S/braco-robortico";

const URL_ULTIMA_RELEASE: &str =
    "https://github.com/Gabriel-Freitas-S/braco-robortico/releases/latest";
const PREFIXO_TAG: &str = "https://github.com/Gabriel-Freitas-S/braco-robortico/releases/tag/";
const URL_FIRMWARE: &str = "https://raw.githubusercontent.com/Gabriel-Freitas-S/braco-robortico/main/firmware/braco_4dof/braco_4dof.ino";
const PREFIXO_RAW: &str = "https://raw.githubusercontent.com/Gabriel-Freitas-S/braco-robortico/";

/// Teto de leitura do sketch (bytes). O arquivo real tem ~60 KB.
const TETO_SKETCH: u64 = 128 * 1024;

/// Identificação exigida pelos serviços do GitHub.
const AGENTE: &str = concat!("robocontrol/", env!("CARGO_PKG_VERSION"));

/// Versão do app em execução.
pub fn versao_app() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// O que o repositório oferece hoje, já baixado.
#[derive(Clone, Debug, Default)]
pub struct Novidades {
    /// Tag `app-vX.Y.Z` do release mais recente, se houver algum.
    pub app: Option<String>,
    /// `FW_VER` do sketch em `main`.
    pub firmware: Option<String>,
    /// Texto do sketch de `main`, para o botão "Salvar".
    pub sketch: Option<String>,
    /// Primeira falha de rede/parse; o que funcionou continua válido.
    pub erro: Option<String>,
}

/// Dispara a checagem em segundo plano. Uma mensagem e a thread termina.
pub fn checar() -> Receiver<Novidades> {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let mut n = Novidades::default();

        match ultima_app() {
            Ok(Some(v)) => n.app = Some(v),
            Ok(None) => {} // nenhum release publicado ainda: silêncio
            Err(e) => n.erro = Some(e),
        }

        match baixa_sketch() {
            Ok(corpo) => match firmware::validar(&corpo) {
                Some(v) => {
                    n.firmware = Some(v.to_string());
                    n.sketch = Some(corpo);
                }
                None => n.erro = Some("sketch de main não parece válido".into()),
            },
            Err(e) => {
                n.erro.get_or_insert(e);
            }
        }

        let _ = tx.send(n);
    });
    rx
}

/// Link da página da release de uma versão já validada.
pub fn url_release(versao: &str) -> Option<String> {
    firmware::valida_versao(versao)?;
    Some(format!(
        "https://github.com/{REPO}/releases/tag/app-v{versao}"
    ))
}

/// Versão embutida no destino de `releases/latest`; `None` se o release mais
/// recente não for do app (ex.: um release de firmware).
fn versao_app_da_url(url: &str) -> Option<&str> {
    url.strip_prefix(PREFIXO_TAG)?
        .strip_prefix("app-v")
        .and_then(firmware::valida_versao)
}

/// Última versão do app publicada, seguindo o redirect de `releases/latest`.
fn ultima_app() -> Result<Option<String>, String> {
    let resposta = match agente()
        .get(URL_ULTIMA_RELEASE)
        .set("User-Agent", AGENTE)
        .call()
    {
        Ok(r) => r,
        //  Sem nenhum release o GitHub responde 404: é o estado inicial, não
        //  um erro de rede, e não deve poluir o log.
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(e) => return Err(format!("releases: {e}")),
    };
    Ok(versao_app_da_url(resposta.get_url()).map(str::to_string))
}

/// Baixa o sketch de `main` (mesma fonte que o firmware embutido).
fn baixa_sketch() -> Result<String, String> {
    let resposta = agente()
        .get(URL_FIRMWARE)
        .set("User-Agent", AGENTE)
        .call()
        .map_err(|e| format!("firmware: {e}"))?;
    let url = resposta.get_url();
    if !url.starts_with(PREFIXO_RAW) {
        return Err(format!("firmware: destino inesperado ({url})"));
    }
    let mut corpo = String::new();
    resposta
        .into_reader()
        .take(TETO_SKETCH)
        .read_to_string(&mut corpo)
        .map_err(|e| format!("firmware: {e}"))?;
    Ok(corpo)
}

fn agente() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout_read(Duration::from_secs(5))
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versao_do_app_vem_so_de_tag_app_v() {
        assert_eq!(
            versao_app_da_url(
                "https://github.com/Gabriel-Freitas-S/braco-robortico/releases/tag/app-v0.2.0"
            ),
            Some("0.2.0")
        );
        // Release de firmware não é versão do app.
        assert_eq!(
            versao_app_da_url(
                "https://github.com/Gabriel-Freitas-S/braco-robortico/releases/tag/v1.4.0"
            ),
            None
        );
        // Host/prefixo diferente ou tag fora do formato: ignora.
        assert_eq!(
            versao_app_da_url("https://evil.example/releases/tag/app-v9.9.9"),
            None
        );
        assert_eq!(
            versao_app_da_url(
                "https://github.com/Gabriel-Freitas-S/braco-robortico/releases/tag/app-v0.2"
            ),
            None
        );
    }

    #[test]
    fn url_da_release_recusa_tag_maliciosa() {
        assert_eq!(
            url_release("0.2.0").as_deref(),
            Some("https://github.com/Gabriel-Freitas-S/braco-robortico/releases/tag/app-v0.2.0")
        );
        assert!(url_release("0.2.0/../../evil").is_none());
        assert!(url_release("0.2.0?x=1").is_none());
        assert!(url_release("").is_none());
    }

    /// Checagem real, mesma função do boot (TLS + URLs + parse). Fora do CI
    /// porque depende de rede: `cargo test --release -- --ignored`.
    #[test]
    #[ignore = "usa a rede"]
    fn checagem_real_baixa_o_sketch() {
        let n = checar()
            .recv_timeout(Duration::from_secs(20))
            .expect("a thread não respondeu");
        let fw = n.firmware.expect("versão do firmware não lida");
        let sketch = n.sketch.expect("sketch não baixado");
        assert_eq!(firmware::versao_de(&sketch), Some(fw.as_str()));
        assert!(n.erro.is_none(), "erro inesperado: {:?}", n.erro);
    }
}
