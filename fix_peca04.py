"""
fix_peca04.py
=============
Corrige a peca 04 (Tirante Paralelogramo Medio) que interpenetrava a peca 10
(Balancim Triangular) no assembly.

A peca e curvada suavemente na direcao da espessura (X global / Z local) no
trecho onde cruza o balancim. Os dois furos das extremidades permanecem nas
posicoes originais (0..80 mm), preservando as conexoes com a peca 14 e o
braco 02. A curva de 5 mm afasta a peca do balancim sem alterar o comprimento
entre furos.

Gera:
  cad-output/04_Tirante_Paralelogramo_Medio_corrigido.stl
  pecas_impressao_3d/04_Tirante_Paralelogramo_Medio_corrigido.stl
"""
import os
import numpy as np
from build123d import (
    BuildPart, BuildSketch, BuildLine, Plane, Polyline, make_face,
    extrude, Cylinder, Locations, Mode, export_stl,
)

BEND = 5.0          # desvio maximo no meio (mm)
WIDTH = 10.0        # largura da peca
THICK = 3.2         # espessura
HOLE_SPACING = 80.0 # distancia entre furos
HOLE_R = 1.6        # raio do furo (M3)


def smoothstep(t):
    t = np.clip(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def bump(x):
    """0 fora do trecho do balancim, 1 no trecho, transicoes suaves."""
    up = smoothstep((x + 37.8) / 8.0)
    down = 1.0 - smoothstep((x - 31.2) / 7.0)
    return float(min(up, down))


def build():
    half = HOLE_SPACING / 2.0
    end = half + 5.0
    xs = np.linspace(-end, end, 181)
    zc = np.array([BEND * bump(x) for x in xs])
    top = [(float(x), float(z + THICK / 2)) for x, z in zip(xs, zc)]
    bot = [(float(x), float(z - THICK / 2)) for x, z in zip(xs[::-1], zc[::-1])]

    with BuildPart() as p:
        with BuildSketch(Plane.XZ):
            with BuildLine():
                Polyline(*(top + bot), close=True)
            make_face()
        extrude(amount=WIDTH / 2, both=True)
        with Locations((half, 0, 0), (-half, 0, 0)):
            Cylinder(radius=HOLE_R, height=WIDTH + 2, mode=Mode.SUBTRACT)
    return p.part


def main():
    base = os.path.dirname(os.path.abspath(__file__))
    part = build()
    outputs = [
        os.path.join(base, "cad-output", "04_Tirante_Paralelogramo_Medio_corrigido.stl"),
        os.path.join(base, "pecas_impressao_3d", "04_Tirante_Paralelogramo_Medio_corrigido.stl"),
    ]
    for path in outputs:
        os.makedirs(os.path.dirname(path), exist_ok=True)
        export_stl(part, path)
        print(f"salvo: {path} ({os.path.getsize(path):,} bytes)")
    print(f"volume: {part.volume:.1f} mm3")
    print("peca 04 corrigida: curva de 5 mm, furos mantidos a 80 mm.")


if __name__ == "__main__":
    main()
