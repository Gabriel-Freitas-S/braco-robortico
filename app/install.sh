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
cat > "$APPS/robocontrol.desktop" <<EOF
[Desktop Entry]
Type=Application
Name=RoboControl
GenericName=Controle de braço robótico
Comment=Controle por toque e gravação de movimentos do braço 4-DOF (Arduino USB)
Exec=$BIN/robocontrol
Icon=robocontrol
Terminal=false
Categories=Utility;Engineering;
Keywords=robo;braco;arduino;servo;robotic;
StartupWMClass=robocontrol
EOF
chmod 644 "$APPS/robocontrol.desktop"

command -v update-desktop-database >/dev/null && update-desktop-database "$APPS" || true
command -v gtk-update-icon-cache >/dev/null && gtk-update-icon-cache -q -t "$HOME/.local/share/icons/hicolor" 2>/dev/null || true

echo "== pronto: procure 'RoboControl' no menu (ou: $BIN/robocontrol)"
