import trimesh
import numpy as np

def analyze_holes(mesh, name):
    print(f"=== {name} ===")
    # Take a slice at mid-height
    z_mid = (mesh.bounds[0][2] + mesh.bounds[1][2]) / 2.0
    slice_2d = mesh.section(plane_origin=[0, 0, z_mid], plane_normal=[0, 0, 1])
    if slice_2d is None:
        print("Slice is None")
        return
    path = slice_2d.to_planar()[0]
    print(f"Polygons: {len(path.polygons_full)}")
    for i, poly in enumerate(path.polygons_full):
        print(f"Poly {i}: area={poly.area:.2f}, bounds={poly.bounds}")
        for j, hole in enumerate(poly.interiors):
            print(f"  Hole {j}: area={hole.area:.2f}, bounds={hole.bounds}")

analyze_holes(trimesh.load('pecas_impressao_3d/16_Berco_Servo_Rotacao_Base.stl'), '16_Berco')
analyze_holes(trimesh.load('pecas_impressao_3d/14_Separador_Espacador_Central_Base.stl'), '14_Separador')
analyze_holes(trimesh.load('pecas_impressao_3d/25_Trava_Horn_Servo_Base.stl'), '25_Trava')
analyze_holes(trimesh.load('pecas_impressao_3d/12_Mesa_Giratoria_Turntable.stl'), '12_Mesa')
