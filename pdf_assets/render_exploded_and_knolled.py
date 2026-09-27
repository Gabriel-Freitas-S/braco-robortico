import http.server
import socketserver
import json
import base64
import os
import subprocess
import threading
import time

PORT = 9997
BASE_DIR = r"d:\braco-robortico"
RENDERS_DIR = os.path.join(BASE_DIR, "pdf_assets", "renders")
os.makedirs(RENDERS_DIR, exist_ok=True)

TOTAL_EXPECTED = 2
saved_count = 0
done_event = threading.Event()

HTML_CONTENT = """<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <script src="/pdf_assets/three/three.min.js"></script>
  <script src="/pdf_assets/three/GLTFLoader.js"></script>
</head>
<body style="margin:0; background:transparent; overflow:hidden;">
  <canvas id="c" width="1800" height="1200" style="width:900px; height:600px;"></canvas>
  <script>
    async function logMsg(msg) {
      console.log(msg);
      try {
        await fetch('/log', {
          method: 'POST',
          headers: {'Content-Type': 'application/json'},
          body: JSON.stringify({ msg })
        });
      } catch(e) {}
    }

    async function sendImage(name, canvas) {
      const data = canvas.toDataURL('image/png', 0.98);
      await fetch('/save_render', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, data })
      });
      await logMsg('Saved render: ' + name);
    }

    window.onload = async function() {
      await logMsg('Page loaded, initializing Three.js...');
      const canvas = document.getElementById('c');
      const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
      renderer.setSize(1800, 1200);
      renderer.setClearColor(0x000000, 0); // 100% transparent background
      renderer.outputEncoding = THREE.sRGBEncoding;

      const loader = new THREE.GLTFLoader();

      // =======================================================================
      // RENDER 1: Exploded Assembly (braco_colorido_exploded.png)
      // =======================================================================
      await logMsg('Loading braco_exploded.glb...');
      const scene1 = new THREE.Scene();
      scene1.background = null;

      const ambLight1 = new THREE.AmbientLight(0xffffff, 0.95);
      scene1.add(ambLight1);
      const dirLight1 = new THREE.DirectionalLight(0xffffff, 1.1);
      dirLight1.position.set(-200, -300, 450);
      scene1.add(dirLight1);
      const fillLight1 = new THREE.DirectionalLight(0xcfd8dc, 0.7);
      fillLight1.position.set(300, -100, 250);
      scene1.add(fillLight1);

      await new Promise((resolve) => {
        loader.load('/pdf_assets/braco_exploded.glb?t=' + Date.now(), async (gltf) => {
          const model1 = gltf.scene;
          model1.traverse((node) => {
            if (node.isMesh && node.material) {
              node.material.roughness = 0.45;
              node.material.metalness = 0.15;
            }
          });
          scene1.add(model1);

          const bbox = new THREE.Box3().setFromObject(model1);
          const center = bbox.getCenter(new THREE.Vector3());

          const camera1 = new THREE.PerspectiveCamera(28, 1800 / 1200, 1, 3000);
          camera1.up.set(0, 0, 1);
          // Isometric angle showing front, side and top clearly
          camera1.position.set(center.x - 420, center.y - 450, center.z + 280);
          camera1.lookAt(new THREE.Vector3(center.x, center.y, center.z + 20));
          camera1.updateProjectionMatrix();

          renderer.render(scene1, camera1);
          await new Promise(r => setTimeout(r, 200));
          await sendImage('braco_colorido_exploded', canvas);
          resolve();
        });
      });

      // =======================================================================
      // RENDER 2: Knolled Plates (mesa_bambu_knolled.png)
      // =======================================================================
      await logMsg('Loading plates_knolled.glb...');
      renderer.setSize(2000, 1000);
      const scene2 = new THREE.Scene();
      scene2.background = null;

      const ambLight2 = new THREE.AmbientLight(0xffffff, 0.95);
      scene2.add(ambLight2);
      const dirLight2 = new THREE.DirectionalLight(0xffffff, 1.15);
      dirLight2.position.set(-100, -350, 500);
      scene2.add(dirLight2);
      const fillLight2 = new THREE.DirectionalLight(0xcfd8dc, 0.7);
      fillLight2.position.set(200, -100, 300);
      scene2.add(fillLight2);

      // Add 3D Plate Bed outlines / grid
      const gridHelper1 = new THREE.GridHelper(256, 16, 0x475569, 0x334155);
      gridHelper1.rotation.x = Math.PI / 2;
      gridHelper1.position.set(-150, 0, 0.2);
      scene2.add(gridHelper1);

      const gridHelper2 = new THREE.GridHelper(256, 16, 0x475569, 0x334155);
      gridHelper2.rotation.x = Math.PI / 2;
      gridHelper2.position.set(150, 0, 0.2);
      scene2.add(gridHelper2);

      await new Promise((resolve) => {
        loader.load('/pdf_assets/plates_knolled.glb?t=' + Date.now(), async (gltf) => {
          const model2 = gltf.scene;
          model2.traverse((node) => {
            if (node.isMesh && node.material) {
              node.material.roughness = 0.40;
              node.material.metalness = 0.12;
            }
          });
          scene2.add(model2);

          const camera2 = new THREE.PerspectiveCamera(32, 2000 / 1000, 1, 3000);
          camera2.up.set(0, 0, 1);
          // Angle showing both plates side-by-side from slight perspective
          camera2.position.set(0, -560, 580);
          camera2.lookAt(new THREE.Vector3(0, 0, 0));
          camera2.updateProjectionMatrix();

          renderer.render(scene2, camera2);
          await new Promise(r => setTimeout(r, 200));

          // Draw plate label banners on 2D canvas overlay
          const ctx = canvas.getContext('2d');
          
          function drawBadge(text, x, y) {
            ctx.save();
            ctx.font = 'bold 22px Outfit, sans-serif';
            const metrics = ctx.measureText(text);
            const w = metrics.width + 36;
            const h = 42;
            
            // Background pill
            ctx.fillStyle = '#0f172a';
            ctx.strokeStyle = '#FF5E00';
            ctx.lineWidth = 3;
            ctx.beginPath();
            ctx.roundRect(x - w/2, y - h/2, w, h, 8);
            ctx.fill();
            ctx.stroke();

            // Text
            ctx.fillStyle = '#ffffff';
            ctx.textAlign = 'center';
            ctx.textBaseline = 'middle';
            ctx.fillText(text, x, y);
            ctx.restore();
          }

          // Convert 3D plate bottom centers to 2D canvas screen coords
          const p1Bottom3D = new THREE.Vector3(-150, -145, 0);
          p1Bottom3D.project(camera2);
          const p1X = (p1Bottom3D.x * 0.5 + 0.5) * 2000;
          const p1Y = (-(p1Bottom3D.y * 0.5) + 0.5) * 1000;

          const p2Bottom3D = new THREE.Vector3(150, -145, 0);
          p2Bottom3D.project(camera2);
          const p2X = (p2Bottom3D.x * 0.5 + 0.5) * 2000;
          const p2Y = (-(p2Bottom3D.y * 0.5) + 0.5) * 1000;

          drawBadge('PRATO 1 • CHASSI E TORRES (9 PEÇAS)', p1X, p1Y);
          drawBadge('PRATO 2 • BRAÇO E GARRA (19 PEÇAS)', p2X, p2Y);

          await sendImage('mesa_bambu_knolled', canvas);
          resolve();
        });
      });

      await logMsg('ALL_DONE');
    };
  </script>
</body>
</html>
"""

class RenderHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=BASE_DIR, **kwargs)

    def do_GET(self):
        if self.path.startswith('/render.html'):
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            self.wfile.write(HTML_CONTENT.encode('utf-8'))
        else:
            super().do_GET()

    def do_POST(self):
        global saved_count
        content_len = int(self.headers.get('Content-Length', 0))
        post_body = self.rfile.read(content_len)
        payload = json.loads(post_body.decode('utf-8'))

        if self.path == '/log':
            print(f"[Browser Log] {payload.get('msg')}")
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')
        elif self.path == '/save_render':
            name = payload['name']
            data_url = payload['data']
            header, encoded = data_url.split(',', 1)
            img_bytes = base64.b64decode(encoded)

            out_path = os.path.join(RENDERS_DIR, f"{name}.png")
            with open(out_path, 'wb') as f:
                f.write(img_bytes)

            saved_count += 1
            print(f"[{saved_count}/{TOTAL_EXPECTED}] Saved: {out_path} ({len(img_bytes)//1024} KB)")

            self.send_response(200)
            self.send_header('Content-Type', 'application/json')
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')

            if saved_count >= TOTAL_EXPECTED:
                done_event.set()
        else:
            self.send_response(404)
            self.end_headers()

    def log_message(self, format, *args):
        pass

def main():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("", PORT), RenderHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"Render server running on http://localhost:{PORT}")

    chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
    user_data_dir = r"C:\Users\gabri\AppData\Local\Temp\chrome_render_knolled"
    
    args = [
        chrome_path,
        "--headless=new",
        "--use-gl=angle",
        f"--user-data-dir={user_data_dir}",
        "--window-size=2000,1200",
        f"http://localhost:{PORT}/render.html"
    ]

    print("Launching headless Chrome to render exploded & knolled plates...")
    proc = subprocess.Popen(args)

    success = done_event.wait(timeout=45)
    if success:
        print(f"\nSUCCESS: All {TOTAL_EXPECTED} renders completed and saved!")
    else:
        print("\nTIMEOUT waiting for renders to complete.")
    
    proc.terminate()
    httpd.shutdown()

if __name__ == '__main__':
    main()
