import http.server
import socketserver
import json
import base64
import os
import subprocess
import threading
import time

PORT = 9993
BASE_DIR = r"d:\braco-robortico"
RENDERS_DIR = os.path.join(BASE_DIR, "pdf_assets", "renders")
PARTS_DIR = os.path.join(BASE_DIR, "pdf_assets", "parts")
os.makedirs(RENDERS_DIR, exist_ok=True)
os.makedirs(PARTS_DIR, exist_ok=True)

TOTAL_EXPECTED = 6
saved_count = 0
done_event = threading.Event()

HTML_CONTENT = """<!DOCTYPE html>
<html>
<head>
  <meta charset="utf-8">
  <script src="/pdf_assets/three/three.min.js"></script>
  <script src="/pdf_assets/three/GLTFLoader.js"></script>
</head>
<body style="margin:0; background:#f8fafc; overflow:hidden;">
  <canvas id="c" width="1600" height="1200" style="width:800px; height:600px;"></canvas>
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

    async function sendImage(name, canvas, folder='renders') {
      const data = canvas.toDataURL('image/png', 0.98);
      await fetch('/save_render', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, data, folder })
      });
      await logMsg('Saved render: ' + name);
    }

    window.onload = async function() {
      await logMsg('Page loaded, initializing Three.js...');
      const canvas = document.getElementById('c');
      const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
      renderer.setSize(1600, 1200);
      renderer.outputEncoding = THREE.sRGBEncoding;
      renderer.shadowMap.enabled = true;
      renderer.shadowMap.type = THREE.PCFSoftShadowMap;

      const scene = new THREE.Scene();
      scene.background = new THREE.Color(0xf8fafc);

      // Studio 3-point Lighting (Z is UP)
      const ambLight = new THREE.AmbientLight(0xffffff, 0.88);
      scene.add(ambLight);

      const keyLight = new THREE.DirectionalLight(0xffffff, 1.20);
      keyLight.position.set(-200, -250, 400);
      keyLight.castShadow = true;
      keyLight.shadow.mapSize.width = 2048;
      keyLight.shadow.mapSize.height = 2048;
      keyLight.shadow.bias = -0.0005;
      scene.add(keyLight);

      const fillLight = new THREE.DirectionalLight(0xcfd8dc, 0.70);
      fillLight.position.set(250, -100, 250);
      scene.add(fillLight);

      const rimLight = new THREE.DirectionalLight(0xffffff, 0.85);
      rimLight.position.set(0, 350, 250);
      scene.add(rimLight);

      // Ground plane at Z = 0
      const shadowGeo = new THREE.PlaneGeometry(1200, 1200);
      const shadowMat = new THREE.ShadowMaterial({ opacity: 0.14 });
      const shadowMesh = new THREE.Mesh(shadowGeo, shadowMat);
      shadowMesh.position.set(0, 118, 0);
      shadowMesh.receiveShadow = true;
      scene.add(shadowMesh);

      const loader = new THREE.GLTFLoader();
      await logMsg('Loading braco_completo_com_componentes.glb...');

      loader.load('/braco_completo_com_componentes.glb?t=' + Date.now(), async (gltf) => {
        await logMsg('GLB loaded! Processing materials...');
        const model = gltf.scene;

        model.traverse((node) => {
          if (node.isMesh) {
            node.castShadow = true;
            node.receiveShadow = true;
            if (node.material) {
              node.material.roughness = 0.45;
              node.material.metalness = 0.15;
              const n = node.name.toLowerCase();
              if (n.includes('screw') || n.includes('nut') || n.includes('espacador') || n.includes('washer') || n.includes('fixador')) {
                node.material.roughness = 0.22;
                node.material.metalness = 0.88;
              }
            }
          }
        });

        scene.add(model);

        const bbox = new THREE.Box3().setFromObject(model);
        const center = bbox.getCenter(new THREE.Vector3());
        const size = bbox.getSize(new THREE.Vector3());
        await logMsg(`Model center: ${center.x.toFixed(1)}, ${center.y.toFixed(1)}, ${center.z.toFixed(1)} | size: ${size.x.toFixed(1)}, ${size.y.toFixed(1)}, ${size.z.toFixed(1)}`);

        // Camera setup with Z-UP
        const camera = new THREE.PerspectiveCamera(30, 1600 / 1200, 1, 3000);
        camera.up.set(0, 0, 1);

        const targetMain = new THREE.Vector3(center.x, center.y, center.z);
        const targetBase = new THREE.Vector3(0, 115, 25);
        const targetShield = new THREE.Vector3(10.9, 56.5, 42.0);

        const views = [
          {
            name: 'braco_completo_com_componentes_iso',
            folder: 'renders',
            fov: 28,
            pos: new THREE.Vector3(center.x - 360, center.y - 380, center.z + 180),
            target: targetMain
          },
          {
            name: 'braco_completo_com_componentes_side',
            folder: 'renders',
            fov: 26,
            pos: new THREE.Vector3(center.x - 520, center.y, center.z + 10),
            target: new THREE.Vector3(center.x, center.y, center.z + 10)
          },
          {
            name: 'braco_completo_com_componentes_front',
            folder: 'renders',
            fov: 26,
            pos: new THREE.Vector3(center.x, center.y - 520, center.z + 30),
            target: new THREE.Vector3(center.x, center.y, center.z + 30)
          },
          {
            name: 'braco_completo_com_componentes_base',
            folder: 'renders',
            fov: 20,
            pos: new THREE.Vector3(-220, 115 - 220, 95),
            target: targetBase
          },
          {
            name: 'shield_dupont_connections_zoom',
            folder: 'renders',
            fov: 16,
            pos: new THREE.Vector3(10.9 - 140, 56.5 - 180, 42.0 + 110),
            target: targetShield
          },
          {
            name: 'base_servo_underside_check',
            folder: 'renders',
            fov: 20,
            pos: new THREE.Vector3(-180, 115 - 180, 20),
            target: new THREE.Vector3(3.0, 115, 20)
          }
        ];

        for (let i = 0; i < views.length; i++) {
          const v = views[i];
          camera.fov = v.fov;
          camera.position.copy(v.pos);
          camera.lookAt(v.target);
          camera.updateProjectionMatrix();

          renderer.render(scene, camera);
          await new Promise(r => setTimeout(r, 200));

          await sendImage(v.name, canvas, v.folder);
        }

        await logMsg('ALL_RENDERS_DONE');
      }, undefined, async (err) => {
        await logMsg('ERROR_LOADING_GLB: ' + err.message);
      });
    };
  </script>
</body>
</html>
"""

class RenderHandler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=BASE_DIR, **kwargs)

    def do_GET(self):
        if self.path.startswith('/render.html') or self.path == '/':
            self.send_response(200)
            self.send_header('Content-Type', 'text/html; charset=utf-8')
            self.end_headers()
            self.wfile.write(HTML_CONTENT.encode('utf-8'))
        else:
            super().do_GET()

    def do_POST(self):
        global saved_count
        content_length = int(self.headers.get('Content-Length', 0))
        post_data = self.rfile.read(content_length)
        payload = json.loads(post_data.decode('utf-8'))

        if self.path == '/log':
            print(f"[Browser Log] {payload.get('msg')}")
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')
        elif self.path == '/save_render':
            name = payload['name']
            folder = payload.get('folder', 'renders')
            data_url = payload['data']
            header, encoded = data_url.split(',', 1)
            img_bytes = base64.b64decode(encoded)

            target_dir = RENDERS_DIR if folder == 'renders' else PARTS_DIR
            out_path = os.path.join(target_dir, f"{name}.png")
            with open(out_path, 'wb') as f:
                f.write(img_bytes)

            saved_count += 1
            print(f"[{saved_count}/{TOTAL_EXPECTED}] Successfully saved: {out_path} ({len(img_bytes)//1024} KB)")

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
    server_thread = threading.Thread(target=httpd.serve_forever)
    server_thread.daemon = True
    server_thread.start()
    print(f"Render server running on http://localhost:{PORT}")

    chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
    user_data_dir = r"C:\Users\gabri\AppData\Local\Temp\chrome_render_temp_fixed"
    
    args = [
        chrome_path,
        "--headless=new",
        "--use-gl=angle",
        f"--user-data-dir={user_data_dir}",
        "--window-size=1600,1200",
        f"http://localhost:{PORT}/render.html"
    ]

    print("Launching headless Chrome to render updated GLB views...")
    proc = subprocess.Popen(args)

    success = done_event.wait(timeout=40)
    if success:
        print(f"\nSUCCESS: All {TOTAL_EXPECTED} GLB renders completed and saved!")
    else:
        print(f"\nTIMEOUT: Saved {saved_count}/{TOTAL_EXPECTED} renders.")

    try:
        proc.terminate()
    except Exception:
        pass
    httpd.shutdown()

if __name__ == '__main__':
    main()
