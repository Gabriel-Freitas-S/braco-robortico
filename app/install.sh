#!/usr/bin/env bash
# Instala o RoboControl no perfil do usuário (sem root):
#   binário  → ~/.local/bin/robocontrol
#   ícone    → ~/.local/share/icons/hicolor/scalable/apps/robocontrol.svg
#   menu     → ~/.local/share/applications/robocontrol.desktop
# Movimentos gravados ficam em ~/.local/share/robocontrol/movimentos.
set -euo pipefail

cd "$(dirname "$0")"

echo "== compilando (release)"
cargo build --release

BIN="$HOME/.local/bin"
ICONES="$HOME/.local/share/icons/hicolor/scalable/apps"
APPS="$HOME/.local/share/applications"
mkdir -p "$BIN" "$ICONES" "$APPS" "$HOME/.local/share/robocontrol/movimentos"

echo "== instalando binário"
install -m 755 target/release/robocontrol "$BIN/robocontrol"

echo "== instalando ícone"
install -m 644 robocontrol.svg "$ICONES/robocontrol.svg"

echo "== criando entrada de menu"
# O .desktop do repositório é a fonte única (o mesmo que vai no AppImage);
# aqui só trocamos o Exec para o caminho absoluto da instalação.
install -m 644 robocontrol.desktop "$APPS/robocontrol.desktop"
sed -i "s|^Exec=.*|Exec=$BIN/robocontrol|" "$APPS/robocontrol.desktop"
chmod 644 "$APPS/robocontrol.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

# Atalho na área de trabalho (respeita o nome localizado, ex.: "Área de trabalho").
DESKTOP_DIR="$(command -v xdg-user-dir >/dev/null && xdg-user-dir DESKTOP || echo "$HOME/Desktop")"
if [ -d "$DESKTOP_DIR" ]; then
  echo "== atalho na área de trabalho ($DESKTOP_DIR)"
  install -m 755 "$APPS/robocontrol.desktop" "$DESKTOP_DIR/robocontrol.desktop"
  # alguns ambientes (GNOME/COSMIC) só executam .desktop marcado como confiável
  command -v gio >/dev/null && gio set "$DESKTOP_DIR/robocontrol.desktop" metadata::trusted true 2>/dev/null || true
fi

echo "== pronto: procure 'RoboControl' no menu, na área de trabalho, ou: $BIN/robocontrol"
