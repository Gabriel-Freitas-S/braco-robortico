import http.server
import socketserver
import json
import base64
import os
import subprocess
import threading

PORT = 9994
BASE_DIR = r"d:\braco-robortico"
PARTS_DIR = os.path.join(BASE_DIR, "pdf_assets", "parts")
os.makedirs(PARTS_DIR, exist_ok=True)

done_event = threading.Event()

HTML_CONTENT = """<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <script src="/pdf_assets/three/three.min.js"></script>
  <script src="/pdf_assets/three/GLTFLoader.js"></script>
</head>
<body style="margin:0; background:transparent; overflow:hidden;">
  <canvas id="c" width="800" height="800" style="width:400px; height:400px;"></canvas>
  <script>
    window.onload = async function() {
      const canvas = document.getElementById('c');
      const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
      renderer.setSize(800, 800);
      renderer.outputEncoding = THREE.sRGBEncoding;
      renderer.shadowMap.enabled = true;
      renderer.shadowMap.type = THREE.PCFSoftShadowMap;

      const scene = new THREE.Scene();

      // Lighting
      const ambLight = new THREE.AmbientLight(0xffffff, 0.85);
      scene.add(ambLight);

      const keyLight = new THREE.DirectionalLight(0xffffff, 1.3);
      keyLight.position.set(-150, -200, 300);
      keyLight.castShadow = true;
      scene.add(keyLight);

      const fillLight = new THREE.DirectionalLight(0xcfd8dc, 0.7);
      fillLight.position.set(200, -100, 200);
      scene.add(fillLight);

      const loader = new THREE.GLTFLoader();
      loader.load('/cad-output/01_Base_Chassi_Inferior.glb', async (gltf) => {
        const model = gltf.scene;
        model.traverse((node) => {
          if (node.isMesh) {
            node.castShadow = true;
            node.receiveShadow = true;
            if (node.material) {
              node.material.roughness = 0.45;
              node.material.metalness = 0.15;
            }
          }
        });
        scene.add(model);

        const bbox = new THREE.Box3().setFromObject(model);
        const center = bbox.getCenter(new THREE.Vector3());

        const camera = new THREE.PerspectiveCamera(32, 1, 1, 1000);
        camera.up.set(0, 0, 1);
        // Clean 3/4 isometric part view
        camera.position.set(center.x - 180, center.y - 250, center.z + 240);
        camera.lookAt(center);
        camera.updateProjectionMatrix();

        renderer.render(scene, camera);
        await new Promise(r => setTimeout(r, 200));

        const data = canvas.toDataURL('image/png', 0.98);
        await fetch('/save_part', {
          method: 'POST',
          headers: { 'Content-Type': 'application/json' },
          body: JSON.stringify({ name: 'part_01_Base_Chassi_Inferior', data })
        });
      });
    };
  </script>
</body>
</html>
"""

class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=BASE_DIR, **kwargs)

    def do_GET(self):
        self.send_response(200)
        self.send_header('Content-Type', 'text/html; charset=utf-8')
        self.end_headers()
        self.wfile.write(HTML_CONTENT.encode('utf-8'))

    def do_POST(self):
        if self.path == '/save_part':
            content_length = int(self.headers.get('Content-Length', 0))
            post_data = self.rfile.read(content_length)
            payload = json.loads(post_data.decode('utf-8'))

            data_url = payload['data']
            header, encoded = data_url.split(',', 1)
            img_bytes = base64.b64decode(encoded)

            out_path = os.path.join(PARTS_DIR, "part_01_Base_Chassi_Inferior.png")
            with open(out_path, 'wb') as f:
                f.write(img_bytes)

            print(f"Saved updated part image: {out_path} ({len(img_bytes)//1024} KB)")
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')
            done_event.set()

    def log_message(self, format, *args):
        pass

def main():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("", PORT), Handler)
    threading.Thread(target=httpd.serve_forever, daemon=True).start()

    chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
    user_data_dir = r"C:\Users\gabri\AppData\Local\Temp\chrome_render_part"
    args = [
        chrome_path, "--headless=new", "--use-gl=angle",
        f"--user-data-dir={user_data_dir}",
        "--window-size=800,800", f"http://localhost:{PORT}/"
    ]
    proc = subprocess.Popen(args)
    done_event.wait(timeout=25)
    try:
        proc.terminate()
    except Exception:
        pass
    httpd.shutdown()

if __name__ == '__main__':
    main()
