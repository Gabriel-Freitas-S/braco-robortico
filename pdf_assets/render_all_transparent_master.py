import http.server
import socketserver
import json
import base64
import os
import subprocess
import threading
import time
from PIL import Image, ImageDraw, ImageFont

PORT = 9991
BASE_DIR = r"D:\braco-robortico"
RENDERS_DIR = os.path.join(BASE_DIR, "pdf_assets", "renders")
PARTS_DIR = os.path.join(BASE_DIR, "pdf_assets", "parts")
os.makedirs(RENDERS_DIR, exist_ok=True)
os.makedirs(PARTS_DIR, exist_ok=True)

# 28 parts + 7 assembly views + 3 subassemblies + 1 exploded + 1 knolled raw = 40 renders
TOTAL_EXPECTED = 40
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

    async function sendImage(name, canvas, targetFolder='renders') {
      const data = canvas.toDataURL('image/png', 0.98);
      await fetch('/save_render', {
        method: 'POST',
        headers: { 'Content-Type': 'application/json' },
        body: JSON.stringify({ name, data, targetFolder })
      });
      await logMsg('Saved ' + targetFolder + '/' + name);
    }

    function createLights(scene) {
      const amb = new THREE.AmbientLight(0xffffff, 0.92);
      scene.add(amb);
      const key = new THREE.DirectionalLight(0xffffff, 1.25);
      key.position.set(-200, -250, 400);
      scene.add(key);
      const fill = new THREE.DirectionalLight(0xcfd8dc, 0.70);
      fill.position.set(250, -100, 250);
      scene.add(fill);
      const rim = new THREE.DirectionalLight(0xffffff, 0.85);
      rim.position.set(0, 350, 250);
      scene.add(rim);
    }

    window.onload = async function() {
      await logMsg('Master Transparent Renderer starting...');
      const canvas = document.getElementById('c');
      const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
      renderer.setClearColor(0x000000, 0); // 100% TRANSPARENT BACKGROUND
      renderer.outputEncoding = THREE.sRGBEncoding;

      const loader = new THREE.GLTFLoader();

      // =======================================================================
      // PHASE 1: RENDER ALL 28 INDIVIDUAL PARTS (all_parts_flat.glb)
      // =======================================================================
      await logMsg('PHASE 1: Loading all_parts_flat.glb for 28 parts...');
      renderer.setSize(1024, 1024);

      const partScene = new THREE.Scene();
      createLights(partScene);

      await new Promise((resolve) => {
        loader.load('/pdf_assets/all_parts_flat.glb?t=' + Date.now(), async (gltf) => {
          const partsModel = gltf.scene;
          partScene.add(partsModel);

          const partMeshes = [];
          partsModel.traverse((node) => {
            if (node.isMesh) {
              node.visible = false;
              if (node.material) {
                node.material.color.setHex(0x22262C); // Technical Black PLA
                node.material.roughness = 0.45;
                node.material.metalness = 0.15;
              }
              partMeshes.push(node);
            }
          });

          partMeshes.sort((a, b) => a.name.localeCompare(b.name));
          await logMsg('Found ' + partMeshes.length + ' part meshes. Rendering...');

          const partCam = new THREE.PerspectiveCamera(30, 1, 1, 3000);
          partCam.up.set(0, 0, 1);

          for (let i = 0; i < partMeshes.length; i++) {
            const m = partMeshes[i];
            m.visible = true;

            const bbox = new THREE.Box3().setFromObject(m);
            const size = bbox.getSize(new THREE.Vector3());
            const center = bbox.getCenter(new THREE.Vector3());
            const maxDim = Math.max(size.x, size.y, size.z, 25.0);

            const dist = maxDim * 2.2;
            partCam.position.set(center.x - dist * 0.7, center.y - dist * 0.9, center.z + dist * 1.1);
            partCam.lookAt(new THREE.Vector3(center.x, center.y, center.z));
            partCam.updateProjectionMatrix();

            renderer.render(partScene, partCam);
            await new Promise(r => setTimeout(r, 60));

            const partFileName = m.name; // e.g. "part_01_Base_Chassi_Inferior"
            await sendImage(partFileName, canvas, 'parts');
            m.visible = false;
          }
          resolve();
        });
      });

      // =======================================================================
      // PHASE 2: FULL ASSEMBLY VIEWS (braco_completo_com_componentes.glb)
      // =======================================================================
      await logMsg('PHASE 2: Loading braco_completo_com_componentes.glb...');
      renderer.setSize(1600, 1200);

      const asmbScene = new THREE.Scene();
      createLights(asmbScene);

      await new Promise((resolve) => {
        loader.load('/braco_completo_com_componentes.glb?t=' + Date.now(), async (gltf) => {
          const asmbModel = gltf.scene;
          asmbScene.add(asmbModel);

          asmbModel.traverse((node) => {
            if (node.isMesh && node.material) {
              node.material.roughness = 0.45;
              node.material.metalness = 0.15;
              const n = node.name.toLowerCase();
              if (n.includes('screw') || n.includes('nut') || n.includes('espacador') || n.includes('fixador')) {
                node.material.roughness = 0.22;
                node.material.metalness = 0.88;
              }
            }
          });

          const bbox = new THREE.Box3().setFromObject(asmbModel);
          const center = bbox.getCenter(new THREE.Vector3());
          const asmbCam = new THREE.PerspectiveCamera(28, 1600 / 1200, 1, 4000);
          asmbCam.up.set(0, 0, 1);

          const fullViews = [
            {
              name: 'braco_completo_com_componentes_iso',
              fov: 28,
              pos: new THREE.Vector3(center.x - 360, center.y - 380, center.z + 180),
              target: new THREE.Vector3(center.x, center.y, center.z)
            },
            {
              name: 'braco_completo_com_componentes_side',
              fov: 26,
              pos: new THREE.Vector3(center.x - 520, center.y, center.z + 10),
              target: new THREE.Vector3(center.x, center.y, center.z + 10)
            },
            {
              name: 'braco_completo_com_componentes_front',
              fov: 26,
              pos: new THREE.Vector3(center.x, center.y - 520, center.z + 30),
              target: new THREE.Vector3(center.x, center.y, center.z + 30)
            },
            {
              name: 'braco_completo_com_componentes_base',
              fov: 20,
              pos: new THREE.Vector3(-220, 115 - 220, 95),
              target: new THREE.Vector3(0, 115, 25)
            },
            {
              name: 'braco_completo_com_componentes_top',
              fov: 24,
              pos: new THREE.Vector3(center.x, center.y, center.z + 650),
              target: new THREE.Vector3(center.x, center.y, center.z)
            },
            {
              name: 'braco_capa_lateral_dinamica',
              fov: 30,
              pos: new THREE.Vector3(center.x - 420, center.y - 280, center.z + 140),
              target: new THREE.Vector3(center.x + 10, center.y + 20, center.z - 10)
            },
            {
              name: 'subconjunto_1_base',
              fov: 22,
              pos: new THREE.Vector3(-240, 115 - 240, 110),
              target: new THREE.Vector3(0, 115, 25)
            }
          ];

          for (let v of fullViews) {
            asmbModel.traverse(n => {
              if (n.isMesh) {
                if (v.name === 'subconjunto_1_base') {
                  const nName = n.name.toLowerCase();
                  n.visible = nName.includes('01') || nName.includes('tpu') || nName.includes('espacador') || nName.includes('arduino') || nName.includes('shield') || nName.includes('base');
                } else {
                  n.visible = true;
                }
              }
            });

            asmbCam.fov = v.fov;
            asmbCam.position.copy(v.pos);
            asmbCam.lookAt(v.target);
            asmbCam.updateProjectionMatrix();

            renderer.render(asmbScene, asmbCam);
            await new Promise(r => setTimeout(r, 80));
            await sendImage(v.name, canvas, 'renders');
          }
          resolve();
        });
      });

      // =======================================================================
      // PHASE 3: SUBCONJUNTOS 2, 3, 4 (Dedicated GLBs)
      // =======================================================================
      await logMsg('PHASE 3: Rendering subconjuntos 2, 3, 4...');
      renderer.setSize(1200, 1000);

      const subFiles = [
        { file: '/pdf_assets/subconjunto_2_torres.glb', name: 'subconjunto_2_torres', fov: 24, distMult: 2.4 },
        { file: '/pdf_assets/subconjunto_3_braco.glb',  name: 'subconjunto_3_braco',  fov: 24, distMult: 2.2 },
        { file: '/pdf_assets/subconjunto_4_garra.glb',  name: 'subconjunto_4_garra',  fov: 22, distMult: 2.2 }
      ];

      for (let s of subFiles) {
        await new Promise((resolve) => {
          const sScene = new THREE.Scene();
          createLights(sScene);
          loader.load(s.file + '?t=' + Date.now(), async (gltf) => {
            const sm = gltf.scene;
            sScene.add(sm);
            sm.traverse((node) => {
              if (node.isMesh && node.material) {
                node.material.color.setHex(0x22262C);
                node.material.roughness = 0.45;
                node.material.metalness = 0.15;
              }
            });

            const bbox = new THREE.Box3().setFromObject(sm);
            const size = bbox.getSize(new THREE.Vector3());
            const center = bbox.getCenter(new THREE.Vector3());
            const maxDim = Math.max(size.x, size.y, size.z, 50.0);

            const sCam = new THREE.PerspectiveCamera(s.fov, 1200 / 1000, 1, 3000);
            sCam.up.set(0, 0, 1);
            const dist = maxDim * s.distMult;
            sCam.position.set(center.x - dist * 0.7, center.y - dist * 0.85, center.z + dist * 0.65);
            sCam.lookAt(center);
            sCam.updateProjectionMatrix();

            renderer.render(sScene, sCam);
            await new Promise(r => setTimeout(r, 80));
            await sendImage(s.name, canvas, 'renders');
            resolve();
          });
        });
      }

      // =======================================================================
      // PHASE 4: EXPLODED VIEW (braco_exploded.glb)
      // =======================================================================
      await logMsg('PHASE 4: Rendering braco_exploded.glb...');
      renderer.setSize(1600, 1600);
      const scExploded = new THREE.Scene();
      createLights(scExploded);

      await new Promise((resolve) => {
        loader.load('/pdf_assets/braco_exploded.glb?t=' + Date.now(), async (gltf) => {
          const mExploded = gltf.scene;
          mExploded.traverse((n) => {
            if (n.isMesh && n.material) {
              n.material.color.setHex(0x22262C);
              n.material.roughness = 0.45;
              n.material.metalness = 0.15;
            }
          });
          scExploded.add(mExploded);

          const bbox = new THREE.Box3().setFromObject(mExploded);
          const center = bbox.getCenter(new THREE.Vector3());

          const camExploded = new THREE.PerspectiveCamera(32, 1, 1, 4000);
          camExploded.up.set(0, 0, 1);
          camExploded.position.set(center.x - 550, center.y - 650, center.z + 320);
          camExploded.lookAt(new THREE.Vector3(center.x, center.y + 10, center.z - 10));
          camExploded.updateProjectionMatrix();

          renderer.render(scExploded, camExploded);
          await new Promise(r => setTimeout(r, 100));
          await sendImage('braco_colorido_exploded', canvas, 'renders');
          resolve();
        });
      });

      // =======================================================================
      // PHASE 5: KNOLLED PLATES VIEW (plates_knolled.glb)
      // =======================================================================
      await logMsg('PHASE 5: Rendering plates_knolled.glb...');
      renderer.setSize(2200, 1100);
      const scKnolled = new THREE.Scene();
      createLights(scKnolled);

      await new Promise((resolve) => {
        loader.load('/pdf_assets/plates_knolled.glb?t=' + Date.now(), async (gltf) => {
          const mKnolled = gltf.scene;
          scKnolled.add(mKnolled);

          const camKnolled = new THREE.PerspectiveCamera(32, 2200 / 1100, 1, 4000);
          camKnolled.up.set(0, 0, 1);
          camKnolled.position.set(0, -560, 520);
          camKnolled.lookAt(new THREE.Vector3(0, 0, 0));
          camKnolled.updateProjectionMatrix();

          renderer.render(scKnolled, camKnolled);
          await new Promise(r => setTimeout(r, 100));
          await sendImage('mesa_bambu_knolled_raw', canvas, 'renders');
          resolve();
        });
      });

      await logMsg('ALL_40_MASTER_RENDERS_DONE');
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
            print(f"[Renderer] {payload.get('msg')}")
            self.send_response(200)
            self.end_headers()
            self.wfile.write(b'{"status":"ok"}')
        elif self.path == '/save_render':
            name = payload['name']
            folder = payload.get('targetFolder', 'renders')
            data_url = payload['data']
            header, encoded = data_url.split(',', 1)
            img_bytes = base64.b64decode(encoded)

            target_dir = RENDERS_DIR if folder == 'renders' else PARTS_DIR
            out_path = os.path.join(target_dir, f"{name}.png")
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

    print("Post-processing mesa_bambu_knolled with PIL transparent badges...")
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

        # Draw soft shadow
        draw.rounded_rectangle([x0+2, y0+3, x1+2, y1+3], radius=8, fill=(0, 0, 0, 100))
        # Draw dark fill
        draw.rounded_rectangle([x0, y0, x1, y1], radius=8, fill=(15, 23, 42, 245), outline=(255, 94, 0, 255), width=3)
        # Draw text
        draw.text((cx - tw/2, cy - th/2 - 1), text, font=font, fill=(255, 255, 255, 255))

    draw_badge("PRATO 1 • CHASSI E TORRES (9 PEÇAS - PLA PRETO)", 570, 970)
    draw_badge("PRATO 2 • BRAÇO E GARRA (19 PEÇAS - PLA PRETO)", 1630, 970)

    im.save(final_path)
    print(f"Saved finalized transparent {final_path} successfully!")

def main():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("", PORT), RenderHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"Master Render Server running on http://localhost:{PORT}")

    chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
    user_data_dir = r"C:\Users\gabri\AppData\Local\Temp\chrome_render_master_transp"
    
    args = [
        chrome_path,
        "--headless=new",
        "--use-gl=angle",
        f"--user-data-dir={user_data_dir}",
        "--window-size=2200,1600",
        f"http://localhost:{PORT}/render.html"
    ]

    print("Launching headless Chrome to render all 40 views with TRANSPARENT background...")
    proc = subprocess.Popen(args)

    success = done_event.wait(timeout=90)
    if success:
        print(f"\nSUCCESS: All {TOTAL_EXPECTED} transparent Black PLA renders completed and saved!")
        add_knolled_badges()
    else:
        print(f"\nTIMEOUT: Saved {saved_count}/{TOTAL_EXPECTED} renders.")

    try:
        proc.terminate()
    except Exception:
        pass
    httpd.shutdown()

if __name__ == '__main__':
    main()
