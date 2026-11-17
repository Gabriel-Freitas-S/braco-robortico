//! Firmware embutido no binário e gravação do sketch para upload.
//!
//! O `.ino` do repositório é a fonte única da versão: o app lê
//! `#define FW_VER` do próprio texto em vez de manter uma constante paralela
//! que envelhece sozinha. O sketch vai compilado junto (`include_str!`), então
//! o app funciona offline e os três pacotes (AppImage, zip do Windows e
//! `.app`) carregam exatamente o mesmo firmware.
//!
//! A gravação é só de texto: quem grava na placa é o Arduino IDE ou o
//! arduino-cli, com o arquivo salvo por [`salvar`]. O app nunca executa nada
//! baixado nem manda firmware pela serial.

use std::cmp::Ordering;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

/// Sketch do firmware, compilado junto com o app.
pub const SKETCH: &str = include_str!("../../firmware/braco_4dof/braco_4dof.ino");

/// Nome da pasta e do arquivo que o Arduino IDE exige (`pasta/arquivo.ino`).
const NOME: &str = "braco_4dof";

/// Versão declarada no sketch embutido.
pub fn versao() -> &'static str {
    static VERSAO: OnceLock<&'static str> = OnceLock::new();
    VERSAO.get_or_init(|| versao_de(SKETCH).unwrap_or("?"))
}

/// Extrai o valor de `#define FW_VER "x.y.z"`.
pub fn versao_de(texto: &str) -> Option<&str> {
    let linha = texto
        .lines()
        .find(|l| l.trim_start().starts_with("#define FW_VER"))?;
    let (_, resto) = linha.split_once('"')?;
    let (versao, _) = resto.split_once('"')?;
    valida_versao(versao.trim())
}

/// Aceita só `X.Y.Z` numérico. É o que vira URL de release e nome de arquivo,
/// então anything else é recusado antes de sair do app.
pub fn valida_versao(s: &str) -> Option<&str> {
    let mut partes = s.split('.');
    let (a, b, c) = (partes.next()?, partes.next()?, partes.next()?);
    if partes.next().is_some() {
        return None;
    }
    for p in [a, b, c] {
        if p.is_empty() || p.len() > 4 || !p.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
    }
    Some(s)
}

/// Compara duas versões válidas; `None` se alguma não for `X.Y.Z`.
pub fn compara(a: &str, b: &str) -> Option<Ordering> {
    Some(chave(a)?.cmp(&chave(b)?))
}

fn chave(s: &str) -> Option<(u32, u32, u32)> {
    let mut partes = valida_versao(s)?.split('.');
    Some((
        partes.next()?.parse().ok()?,
        partes.next()?.parse().ok()?,
        partes.next()?.parse().ok()?,
    ))
}

/// Valida um sketch que veio da rede antes de gravar: tamanho de sketch, a
/// biblioteca que o firmware usa e uma versão legível no cabeçalho. Pega
/// página de erro/HTML servida no lugar do arquivo.
pub fn validar(texto: &str) -> Option<&str> {
    if !(5_000..=200_000).contains(&texto.len()) || texto.contains('\0') {
        return None;
    }
    if !texto.contains("#include <Servo.h>") {
        return None;
    }
    versao_de(texto)
}

/// Grava `texto` em `<dir>/braco_4dof/braco_4dof.ino`, criando a pasta.
///
/// Escrita atômica (`tmp` + `rename`): um download interrompido não deixa um
/// sketch pela metade no lugar do bom. `dir` é o diretório de dados do usuário
/// — nunca ao lado do executável, que no AppImage é somente leitura.
pub fn salvar(dir: &Path, texto: &str) -> std::io::Result<PathBuf> {
    let pasta = dir.join(NOME);
    fs::create_dir_all(&pasta)?;
    let destino = pasta.join(format!("{NOME}.ino"));
    let tmp = pasta.join(format!("{NOME}.ino.tmp"));
    fs::write(&tmp, texto)?;
    fs::rename(&tmp, &destino)?;
    Ok(destino)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versao_embutida_vem_do_proprio_sketch() {
        let v = versao();
        assert_eq!(valida_versao(v), Some(v));
        assert_eq!(versao_de(SKETCH), Some(v));
    }

    #[test]
    fn comparacao_de_versoes() {
        assert_eq!(compara("1.3.7", "1.3.7"), Some(Ordering::Equal));
        assert_eq!(compara("1.4.0", "1.3.7"), Some(Ordering::Greater));
        assert_eq!(compara("1.10.0", "1.9.9"), Some(Ordering::Greater));
        assert_eq!(compara("1.3", "1.3.0"), None);
        assert_eq!(compara("1.3.7-beta", "1.3.7"), None);
        assert_eq!(compara("01.3.7", "1.3.7"), Some(Ordering::Equal));
    }

    #[test]
    fn validar_recusa_pagina_de_erro() {
        assert!(validar("<html>404</html>").is_none());
        assert!(validar("#define FW_VER \"9.9.9\"\n#include <Servo.h>\n").is_none());

        let mut bom = String::from("#define FW_VER \"9.9.9\"\n#include <Servo.h>\n");
        bom.push_str(&"x".repeat(6_000));
        assert_eq!(validar(&bom), Some("9.9.9"));
    }

    #[test]
    fn salvar_cria_pasta_com_nome_do_sketch() {
        let dir = std::env::temp_dir().join(format!("robocontrol-fw-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);

        let p = salvar(&dir, "#define FW_VER \"1.2.3\"\n").unwrap();
        assert!(p.ends_with("braco_4dof/braco_4dof.ino"));
        assert_eq!(
            fs::read_to_string(&p).unwrap(),
            "#define FW_VER \"1.2.3\"\n"
        );
        assert!(!dir.join("braco_4dof/braco_4dof.ino.tmp").exists());

        let _ = fs::remove_dir_all(&dir);
    }
}
