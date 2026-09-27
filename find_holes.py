import sys
from build123d import *

def analyze_holes(filepath):
    print(f"--- Analyzing {filepath} ---")
    try:
        obj = import_stl(filepath)
        # Bounding box
        print(f"Bounding Box: {obj.bounding_box()}")
        # We can't directly get hole centers from STL faces easily without finding cylinders,
        # but since it's a flat plate, let's find the max/min points.
        bbox = obj.bounding_box()
        print(f"Center: {bbox.center()}")
        # Assuming holes are along the X axis
    except Exception as e:
        print(f"Error: {e}")

analyze_holes("pecas_impressao_3d/07_Braco_Principal_Direito.stl")
