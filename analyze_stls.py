import sys
from build123d import *

def analyze_stl(filepath):
    try:
        obj = import_stl(filepath)
        bbox = obj.bounding_box()
        print(f"--- {filepath} ---")
        print(f"Bounding Box: {bbox}")
        print(f"Size X: {bbox.size.X}, Size Y: {bbox.size.Y}, Size Z: {bbox.size.Z}")
        
        # approximate hole centers by looking at vertices or edges? 
        # STLs only have faces.
    except Exception as e:
        print(f"Error loading {filepath}: {e}")

analyze_stl("pecas_impressao_3d/07_Braco_Principal_Direito.stl")
analyze_stl("pecas_impressao_3d/04_Tirante_Paralelogramo_Medio.stl")
analyze_stl("pecas_impressao_3d/09_Torre_Suporte_Servo_Direita.stl")
