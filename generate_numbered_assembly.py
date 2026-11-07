"""
generate_numbered_assembly.py
=============================
Gera um modelo 3D do braço robótico MONTADO onde cada uma das 28 peças
impressas é um objeto separado, colorido de forma distinta e com um número 3D
(label flutuante) acima dela, para facilitar a identificação de qual peça está
com problema.

Fonte de posições: preview_data.json (vertices_assembled).
Saída: braco_montado_numerado.glb e braco_montado_numerado.stl
"""
import json
import os
import tempfile
import numpy as np
import trimesh
from build123d import BuildPart, BuildSketch, Text, extrude, export_stl

BASE_DIR = os.path.dirname(os.path.abspath(__file__))
PREVIEW_JSON = os.path.join(BASE_DIR, "preview_data.json")
OUT_GLB = os.path.join(BASE_DIR, "braco_montado_numerado.glb")
OUT_STL = os.path.join(BASE_DIR, "braco_montado_numerado.stl")

# Paleta de cores distintas (RGBA) para as 28 peças
PALETTE = [
    (34, 38, 44), (180, 50, 50), (50, 130, 60), (40, 80, 180),
    (180, 130, 30), (120, 60, 160), (30, 150, 150), (200, 100, 40),
    (90, 90, 90), (220, 60, 120), (60, 180, 120), (100, 60, 200),
    (200, 180, 40), (40, 120, 200), (160, 80, 40), (80, 160, 80),
    (160, 40, 120), (40, 160, 160), (120, 120, 40), (200, 40, 80),
    (80, 40, 200), (40, 200, 80), (200, 120, 40), (40, 80, 120),
    (120, 40, 80), (80, 120, 40), (160, 160, 40), (40, 160, 80),
]

def _smoothstep(t):
    t = np.clip(t, 0.0, 1.0)
    return t * t * (3.0 - 2.0 * t)


def bend(z):
    """1 dentro do trecho do balancim (Z 45..105), 0 fora, com transicoes suaves."""
    up = _smoothstep((z - 36.0) / (44.0 - 36.0))
    down = 1.0 - _smoothstep((z - 105.0) / (112.0 - 105.0))
    return np.minimum(up, down)


def make_label_mesh(number):
    with BuildPart() as p:
        with BuildSketch():
            Text(str(number), font_size=8)
        extrude(amount=2.5)
    fd, path = tempfile.mkstemp(suffix=".stl")
    os.close(fd)
    export_stl(p.part, path)
    m = trimesh.load(path)
    os.remove(path)
    # centraliza na origem
    m.apply_translation(-m.bounds.mean(axis=0))
    return m

def main():
    with open(PREVIEW_JSON, "r", encoding="utf-8") as f:
        data = json.load(f)

    scene = trimesh.Scene()
    all_meshes = []

    for idx, part in enumerate(data["parts"]):
        name = part["name"]
        number = idx + 1
        # número real da peça (prefixo do nome) se existir
        try:
            number = int(name.split("_")[0])
        except (ValueError, IndexError):
            number = idx + 1

        verts = np.array(part["vertices_assembled"], dtype=float).reshape(-1, 3)
        faces = np.array(part["faces"], dtype=int).reshape(-1, 3)
        mesh = trimesh.Trimesh(vertices=verts, faces=faces, process=False)

        # Correcao: peca 04 (Tirante Paralelogramo Medio) interpenetrava a peca 10
        # (Balancim). Curva a peca na direcao X (espessura) no trecho do balancim,
        # desviando dele sem mover os furos das extremidades.
        if number == 4:
            v = np.array(mesh.vertices, dtype=float)
            v[:, 0] += -4.0 * bend(v[:, 2])
            mesh = trimesh.Trimesh(vertices=v, faces=faces, process=False)

        # Substitui peca 26 pela haste_articulacao_olhal_v2.stl remodelada (25.9 mm entre centros)
        if number == 26:
            olhal_stl = os.path.join(BASE_DIR, "haste_articulacao_olhal_v2.stl")
            mesh = trimesh.load(olhal_stl)
            p_crank = np.array([-9.431, 182.135])
            p_servo = np.array([9.780, 164.693])
            theta_olhal = np.arctan2(p_servo[1] - p_crank[1], p_servo[0] - p_crank[0])
            rot = trimesh.transformations.rotation_matrix(theta_olhal, [0, 0, 1])
            mesh.apply_transform(rot)
            mesh.apply_translation([p_crank[0], p_crank[1], 25.77])
            verts = np.array(mesh.vertices, dtype=float)
            part["center_assembled"] = verts.mean(axis=0).tolist()

        # Em preview_data.json, o item 28 continha a guia esquerda do punho;
        # preserva a guia do punho e adiciona a haste_articulacao_curta_v2.stl real na garra.
        if number == 28:
            wrist_color = PALETTE[26 % len(PALETTE)] + (255,)
            mesh.visual = trimesh.visual.ColorVisuals(
                mesh=mesh, vertex_colors=np.tile(np.array(wrist_color, dtype=np.uint8), (len(mesh.vertices), 1))
            )
            scene.add_geometry(mesh, node_name="peca_27b_Guia_Articulacao_Garra_Esq")
            all_meshes.append(mesh)

            curta_stl = os.path.join(BASE_DIR, "haste_articulacao_curta_v2.stl")
            mesh = trimesh.load(curta_stl)
            p_crank = np.array([-9.431, 182.135])
            p_jaw = np.array([0.867, 199.200])
            theta_curta = np.arctan2(p_jaw[1] - p_crank[1], p_jaw[0] - p_crank[0])
            rot = trimesh.transformations.rotation_matrix(theta_curta, [0, 0, 1])
            mesh.apply_transform(rot)
            mesh.apply_translation([p_crank[0], p_crank[1], 28.77])
            verts = np.array(mesh.vertices, dtype=float)
            part["center_assembled"] = verts.mean(axis=0).tolist()

        color = PALETTE[idx % len(PALETTE)] + (255,)
        mesh.visual = trimesh.visual.ColorVisuals(
            mesh=mesh, vertex_colors=np.tile(np.array(color, dtype=np.uint8), (len(mesh.vertices), 1))
        )
        node_name = f"peca_{number:02d}_{name}"
        scene.add_geometry(mesh, node_name=node_name)
        all_meshes.append(mesh)

        # label flutuante acima da peça
        cx, cy, cz = part["center_assembled"]
        label = make_label_mesh(number)
        z_top = float(verts[:, 2].max())
        label.apply_translation([cx, cy, z_top + 8.0])
        label.visual = trimesh.visual.ColorVisuals(
            mesh=label, vertex_colors=np.tile(np.array([255, 215, 0, 255], dtype=np.uint8), (len(label.vertices), 1))
        )
        scene.add_geometry(label, node_name=f"label_{number:02d}")
        all_meshes.append(label)

        print(f"  [{number:02d}] {name}: verts={len(verts)} label colocado em Z={z_top+8:.1f}")

    # Export GLB
    glb_bytes = scene.export(file_type="glb")
    with open(OUT_GLB, "wb") as f:
        f.write(glb_bytes)
    print(f"\nGLB salvo: {OUT_GLB} ({len(glb_bytes):,} bytes)")

    # Export STL (mesh único)
    combined = trimesh.util.concatenate(all_meshes)
    combined.export(OUT_STL)
    print(f"STL salvo: {OUT_STL} ({os.path.getsize(OUT_STL):,} bytes, {len(combined.faces)} faces)")

    print("\nConcluído. Abra o GLB/STL em um visualizador 3D; cada peça tem um")
    print("número (label amarelo) flutuando acima dela para identificação.")

if __name__ == "__main__":
    main()