import http.server
import socketserver
import json
import base64
import os
import subprocess
import threading
import time
from PIL import Image, ImageDraw, ImageFont

PORT = 9998
BASE_DIR = r"D:\braco-robortico"
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
  <canvas id="c"></canvas>
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
      renderer.setClearColor(0x000000, 0); // Transparent background
      renderer.outputEncoding = THREE.sRGBEncoding;

      const loader = new THREE.GLTFLoader();

      // =======================================================================
      // 1. RENDER FIGURA 1.2: Exploded Assembly (braco_colorido_exploded)
      // =======================================================================
      await logMsg('Loading braco_exploded.glb...');
      renderer.setSize(1600, 1600);
      const scene1 = new THREE.Scene();
      scene1.background = null;

      const ambLight1 = new THREE.AmbientLight(0xffffff, 0.95);
      scene1.add(ambLight1);
      const keyLight1 = new THREE.DirectionalLight(0xffffff, 1.25);
      keyLight1.position.set(-250, -350, 500);
      scene1.add(keyLight1);
      const fillLight1 = new THREE.DirectionalLight(0xcfd8dc, 0.75);
      fillLight1.position.set(300, -100, 300);
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

          const camera1 = new THREE.PerspectiveCamera(32, 1600 / 1600, 1, 4000);
          camera1.up.set(0, 0, 1);
          // Angle showing base at bottom, towers, linkages, claw at top
          camera1.position.set(center.x - 550, center.y - 650, center.z + 320);
          camera1.lookAt(new THREE.Vector3(center.x, center.y + 10, center.z - 10));
          camera1.updateProjectionMatrix();

          renderer.render(scene1, camera1);
          await new Promise(r => setTimeout(r, 250));
          await sendImage('braco_colorido_exploded', canvas);
          resolve();
        });
      });

      // =======================================================================
      // 2. RENDER FIGURA 4.1: Knolled Plates (mesa_bambu_knolled_raw)
      // =======================================================================
      await logMsg('Loading plates_knolled.glb...');
      renderer.setSize(2200, 1100);
      const scene2 = new THREE.Scene();
      scene2.background = null;

      const ambLight2 = new THREE.AmbientLight(0xffffff, 0.98);
      scene2.add(ambLight2);
      const keyLight2 = new THREE.DirectionalLight(0xffffff, 1.25);
      keyLight2.position.set(-150, -400, 600);
      scene2.add(keyLight2);
      const fillLight2 = new THREE.DirectionalLight(0xcfd8dc, 0.75);
      fillLight2.position.set(250, -150, 350);
      scene2.add(fillLight2);

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

          const camera2 = new THREE.PerspectiveCamera(32, 2200 / 1100, 1, 4000);
          camera2.up.set(0, 0, 1);
          // Beautiful isometric perspective looking down on both plates
          camera2.position.set(0, -560, 520);
          camera2.lookAt(new THREE.Vector3(0, 0, 0));
          camera2.updateProjectionMatrix();

          renderer.render(scene2, camera2);
          await new Promise(r => setTimeout(r, 250));
          await sendImage('mesa_bambu_knolled_raw', canvas);
          resolve();
        });
      });

      await logMsg('ALL_THREEJS_RENDERS_DONE');
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

def add_knolled_badges():
    raw_path = os.path.join(RENDERS_DIR, "mesa_bambu_knolled_raw.png")
    final_path = os.path.join(RENDERS_DIR, "mesa_bambu_knolled.png")
    if not os.path.exists(raw_path):
        print(f"Error: {raw_path} not found.")
        return

    print("Post-processing mesa_bambu_knolled with PIL badges...")
    im = Image.open(raw_path).convert("RGBA")
    draw = ImageDraw.Draw(im)

    try:
        font = ImageFont.truetype("arialbd.ttf", 26)
    except:
        font = ImageFont.load_default()

    def draw_badge(text, cx, cy):
        bbox = draw.textbbox((0, 0), text, font=font)
        tw = bbox[2] - bbox[0]
        th = bbox[3] - bbox[1]
        pad_x = 24
        pad_y = 12
        w = tw + pad_x * 2
        h = th + pad_y * 2
        x0 = cx - w / 2
        y0 = cy - h / 2
        x1 = cx + w / 2
        y1 = cy + h / 2

        # Draw pill shadow
        draw.rounded_rectangle([x0+2, y0+3, x1+2, y1+3], radius=8, fill=(0, 0, 0, 100))
        # Draw dark fill
        draw.rounded_rectangle([x0, y0, x1, y1], radius=8, fill=(15, 23, 42, 245), outline=(255, 94, 0, 255), width=3)
        # Draw text
        draw.text((cx - tw/2, cy - th/2 - 1), text, font=font, fill=(255, 255, 255, 255))

    # Badge 1 at ~28% width, near bottom (y ~ 970 of 1100)
    draw_badge("PRATO 1 • CHASSI E TORRES (9 PEÇAS)", 570, 970)
    # Badge 2 at ~72% width
    draw_badge("PRATO 2 • BRAÇO E GARRA (19 PEÇAS)", 1630, 970)

    im.save(final_path)
    print(f"Saved finalized {final_path} successfully!")

def main():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("", PORT), RenderHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"Render server running on http://localhost:{PORT}")

    chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
    user_data_dir = r"C:\Users\gabri\AppData\Local\Temp\chrome_render_fig12_41"
    
    args = [
        chrome_path,
        "--headless=new",
        "--use-gl=angle",
        f"--user-data-dir={user_data_dir}",
        "--window-size=2200,1600",
        f"http://localhost:{PORT}/render.html"
    ]

    print("Launching headless Chrome to render Fig 1.2 & Fig 4.1...")
    proc = subprocess.Popen(args)

    success = done_event.wait(timeout=45)
    if success:
        print(f"\nSUCCESS: All {TOTAL_EXPECTED} renders completed and saved!")
        add_knolled_badges()
    else:
        print("\nTIMEOUT waiting for renders to complete.")
    
    proc.terminate()
    httpd.shutdown()

if __name__ == '__main__':
    main()
