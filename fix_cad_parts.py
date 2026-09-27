import sys
from build123d import *

# Parametric CAD models for the robotic arm fixes

def create_link(name, hole_dist, width=10, thickness=3.2, hole_dia=3.2):
    with BuildPart() as p:
        with BuildSketch():
            # A slot (capsule) shape
            SlotOverall(hole_dist + width, width)
            # Two holes
            with Locations((hole_dist/2, 0), (-hole_dist/2, 0)):
                Circle(hole_dia/2, mode=Mode.SUBTRACT)
        extrude(amount=thickness)
    
    p.part.label = name
    return p.part

def fix_parts():
    # 1. Fix the Parallelogram link (04_Tirante) to match the forearm hole distance (80.0 mm)
    # The forearm has holes at exactly 80.0 mm center-to-center distance.
    link_04 = create_link("04_Tirante_Paralelogramo_Medio_corrigido", hole_dist=80.0)
    
    # Export to cad-output
    export_step(link_04, "cad-output/04_Tirante_Paralelogramo_Medio_corrigido.step")
    export_stl(link_04, "cad-output/04_Tirante_Paralelogramo_Medio_corrigido.stl")
    
    print("Fixed 04_Tirante created in cad-output/")

if __name__ == "__main__":
    import os
    os.makedirs("cad-output", exist_ok=True)
    fix_parts()
