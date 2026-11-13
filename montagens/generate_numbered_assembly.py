"""
generate_numbered_assembly.py
=============================
Gera um modelo 3D do braço robótico MONTADO onde cada uma das peças
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

# Paleta de cores distintas (RGBA) para as peças
PALETTE = [
    (34, 38, 44), (180, 50, 50), (50, 130, 60), (40, 80, 180),
    (180, 130, 30), (120, 60, 160), (30, 150, 150), (200, 100, 40),
    (90, 90, 90), (220, 60, 120), (60, 180, 120), (100, 60, 200),
    (200, 180, 40), (40, 120, 200), (160, 80, 40), (80, 160, 80),
    (160, 40, 120), (40, 160, 160), (120, 120, 40), (200, 40, 80),
    (80, 40, 200), (40, 200, 80), (200, 120, 40), (40, 80, 120),
    (120, 40, 80), (80, 120, 40), (160, 160, 40), (40, 160, 80),
]

PARTS_DIR = os.path.join(BASE_DIR, "..", "impressao", "pecas")

# Pecas cuja geometria mudou depois do preview_data.json: usam o STL atual,
# posicionado pela transformacao rígida entre vertices_flat e vertices_assembled.
CURRENT_STLS = {
    "04_Tirante_Paralelogramo_Medio": "04_Tirante_Paralelogramo_Medio.stl",
    "12_Mesa_Giratoria_Turntable": "12_Mesa_Giratoria_Turntable.stl",
}


def flat_to_assembled(part):
    """Rotacao + translacao exatas entre a pose de impressao e a montada."""
    a = np.array(part["vertices_flat"], dtype=float).reshape(-1, 3)
    b = np.array(part["vertices_assembled"], dtype=float).reshape(-1, 3)
    ca, cb = a.mean(axis=0), b.mean(axis=0)
    u, _, vt = np.linalg.svd((a - ca).T @ (b - cb))
    rot = vt.T @ u.T
    if np.linalg.det(rot) < 0:
        vt[-1] *= -1
        rot = vt.T @ u.T
    return rot, cb - rot @ ca


def make_label_mesh(text):
    with BuildPart() as p:
        with BuildSketch():
            Text(str(text), font_size=8)
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

        # Peca com geometria nova: vale o STL atual, nao o mesh do preview_data.
        if name in CURRENT_STLS:
            mat = np.eye(4)
            mat[:3, :3], mat[:3, 3] = flat_to_assembled(part)
            mesh = trimesh.load(os.path.join(PARTS_DIR, CURRENT_STLS[name]))
            mesh.apply_transform(mat)
            verts = np.array(mesh.vertices, dtype=float)

        # A peca 26 (engrenagem) saiu do projeto: a haste de olhal ocupa o lugar dela.
        if number == 26:
            olhal_stl = os.path.join(PARTS_DIR, "haste_articulacao_olhal_v4.stl")
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
        # preserva a guia do punho e adiciona a haste curta real na garra.
        if number == 28:
            wrist_color = PALETTE[26 % len(PALETTE)] + (255,)
            mesh.visual = trimesh.visual.ColorVisuals(
                mesh=mesh, vertex_colors=np.tile(np.array(wrist_color, dtype=np.uint8), (len(mesh.vertices), 1))
            )
            scene.add_geometry(mesh, node_name="peca_27b_Guia_Articulacao_Garra_Esq")
            all_meshes.append(mesh)

            curta_stl = os.path.join(PARTS_DIR, "haste_articulacao_curta.stl")
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
        label_text = "H1" if number == 26 else str(number)
        node_name = "haste_articulacao_olhal" if number == 26 else f"peca_{number:02d}_{name}"
        scene.add_geometry(mesh, node_name=node_name)
        all_meshes.append(mesh)

        # label flutuante acima da peça
        cx, cy, cz = part["center_assembled"]
        label = make_label_mesh(label_text)
        z_top = float(verts[:, 2].max())
        label.apply_translation([cx, cy, z_top + 8.0])
        label.visual = trimesh.visual.ColorVisuals(
            mesh=label, vertex_colors=np.tile(np.array([255, 215, 0, 255], dtype=np.uint8), (len(label.vertices), 1))
        )
        scene.add_geometry(label, node_name=f"label_{label_text}")
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