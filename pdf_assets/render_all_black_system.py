import http.server
import socketserver
import json
import base64
import os
import subprocess
import threading
import time

PORT = 9993
BASE_DIR = r"D:\braco-robortico"
RENDERS_DIR = os.path.join(BASE_DIR, "pdf_assets", "renders")
PARTS_DIR = os.path.join(BASE_DIR, "pdf_assets", "parts")
os.makedirs(RENDERS_DIR, exist_ok=True)
os.makedirs(PARTS_DIR, exist_ok=True)

# 28 parts + 6 assembly views + 4 subassemblies = 38 total renders
TOTAL_EXPECTED = 38
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

    window.onload = async function() {
      await logMsg('Three.js Master Renderer starting...');
      const canvas = document.getElementById('c');
      const renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true, preserveDrawingBuffer: true });
      renderer.setClearColor(0x000000, 0); // 100% transparent
      renderer.outputEncoding = THREE.sRGBEncoding;
      renderer.shadowMap.enabled = true;
      renderer.shadowMap.type = THREE.PCFSoftShadowMap;

      const loader = new THREE.GLTFLoader();

      // =======================================================================
      // PHASE 1: RENDER ALL 28 INDIVIDUAL PARTS (all_parts_flat.glb)
      // =======================================================================
      await logMsg('Loading all_parts_flat.glb for 28 parts...');
      renderer.setSize(1024, 1024);

      const partScene = new THREE.Scene();
      const pAmb = new THREE.AmbientLight(0xffffff, 0.90);
      partScene.add(pAmb);

      const pKey = new THREE.DirectionalLight(0xffffff, 1.25);
      pKey.position.set(-150, -200, 300);
      pKey.castShadow = true;
      partScene.add(pKey);

      const pFill = new THREE.DirectionalLight(0xcfd8dc, 0.65);
      pFill.position.set(200, -100, 200);
      partScene.add(pFill);

      const pRim = new THREE.DirectionalLight(0xffffff, 0.75);
      pRim.position.set(0, 250, 200);
      partScene.add(pRim);

      await new Promise((resolve) => {
        loader.load('/pdf_assets/all_parts_flat.glb?t=' + Date.now(), async (gltf) => {
          const partsModel = gltf.scene;
          partScene.add(partsModel);

          // Collect individual part meshes
          const partMeshes = [];
          partsModel.traverse((node) => {
            if (node.isMesh) {
              node.visible = false;
              node.castShadow = true;
              node.receiveShadow = true;
              if (node.material) {
                node.material.color.setHex(0x22262C); // Technical Black
                node.material.roughness = 0.45;
                node.material.metalness = 0.15;
              }
              partMeshes.push(node);
            }
          });

          // Sort by part index
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

            // Optimal 3/4 isometric viewpoint showing debossed numbers
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
      // PHASE 2: RENDER ASSEMBLY & SUBCONJUNTO VIEWS (braco_completo_com_componentes.glb)
      // =======================================================================
      await logMsg('Loading braco_completo_com_componentes.glb...');
      renderer.setSize(1600, 1200);

      const asmbScene = new THREE.Scene();
      const aAmb = new THREE.AmbientLight(0xffffff, 0.88);
      asmbScene.add(aAmb);

      const aKey = new THREE.DirectionalLight(0xffffff, 1.20);
      aKey.position.set(-200, -250, 400);
      aKey.castShadow = true;
      asmbScene.add(aKey);

      const aFill = new THREE.DirectionalLight(0xcfd8dc, 0.70);
      aFill.position.set(250, -100, 250);
      asmbScene.add(aFill);

      const aRim = new THREE.DirectionalLight(0xffffff, 0.85);
      aRim.position.set(0, 350, 250);
      asmbScene.add(aRim);

      await new Promise((resolve) => {
        loader.load('/braco_completo_com_componentes.glb?t=' + Date.now(), async (gltf) => {
          const asmbModel = gltf.scene;
          asmbScene.add(asmbModel);

          asmbModel.traverse((node) => {
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

          const bbox = new THREE.Box3().setFromObject(asmbModel);
          const center = bbox.getCenter(new THREE.Vector3());
          const asmbCam = new THREE.PerspectiveCamera(28, 1600 / 1200, 1, 4000);
          asmbCam.up.set(0, 0, 1);

          // 1. Full Assembly Views
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
            }
          ];

          for (let v of fullViews) {
            asmbModel.traverse(n => { if (n.isMesh) n.visible = true; });
            asmbCam.fov = v.fov;
            asmbCam.position.copy(v.pos);
            asmbCam.lookAt(v.target);
            asmbCam.updateProjectionMatrix();

            renderer.render(asmbScene, asmbCam);
            await new Promise(r => setTimeout(r, 100));
            await sendImage(v.name, canvas, 'renders');
          }

          // 2. Subconjunto Views
          const subconjuntos = [
            {
              name: 'subconjunto_1_base',
              parts: ['printed_01', 'printed_12', 'printed_16', 'printed_17', 'printed_18', 'printed_25', 'servo_01', 'espacador_base', 'pe_tpu', 'eletronica_arduino', 'cabo_servo_base'],
              fov: 22,
              pos: new THREE.Vector3(-240, 115 - 240, 120),
              target: new THREE.Vector3(0, 115, 25)
            },
            {
              name: 'subconjunto_2_torres',
              parts: ['printed_09', 'printed_11', 'printed_12', 'printed_14', 'printed_22', 'printed_23', 'printed_16', 'printed_25', 'servo_02', 'servo_03', 'screw_m3x30', 'nut_m3_nyloc_1', 'nut_m3_nyloc_2'],
              fov: 24,
              pos: new THREE.Vector3(-180, 115 - 280, 130),
              target: new THREE.Vector3(0, 115, 55)
            },
            {
              name: 'subconjunto_3_braco',
              parts: ['printed_02', 'printed_03', 'printed_04', 'printed_05', 'printed_06', 'printed_07', 'printed_08', 'printed_10', 'printed_13'],
              fov: 26,
              pos: new THREE.Vector3(-280, 130 - 240, 160),
              target: new THREE.Vector3(0, 140, 95)
            },
            {
              name: 'subconjunto_4_garra',
              parts: ['printed_15', 'printed_19', 'printed_20', 'printed_21', 'printed_24', 'printed_26', 'printed_27', 'printed_28', 'servo_04', 'cabo_servo_garra'],
              fov: 22,
              pos: new THREE.Vector3(-160, 200 - 220, 180),
              target: new THREE.Vector3(0, 210, 120)
            }
          ];

          for (let sc of subconjuntos) {
            asmbModel.traverse(n => {
              if (n.isMesh) {
                const match = sc.parts.some(prefix => n.name.toLowerCase().includes(prefix.toLowerCase()));
                n.visible = match;
              }
            });

            asmbCam.fov = sc.fov;
            asmbCam.position.copy(sc.pos);
            asmbCam.lookAt(sc.target);
            asmbCam.updateProjectionMatrix();

            renderer.render(asmbScene, asmbCam);
            await new Promise(r => setTimeout(r, 100));
            await sendImage(sc.name, canvas, 'renders');
          }

          await logMsg('ALL_38_RENDERS_FINISHED');
          resolve();
        });
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
            print(f"[Renderer Log] {payload.get('msg')}")
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

def main():
    socketserver.TCPServer.allow_reuse_address = True
    httpd = socketserver.TCPServer(("", PORT), RenderHandler)
    server_thread = threading.Thread(target=httpd.serve_forever, daemon=True)
    server_thread.start()
    print(f"Master Render Server running on http://localhost:{PORT}")

    chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
    user_data_dir = r"C:\Users\gabri\AppData\Local\Temp\chrome_render_master_black"
    
    args = [
        chrome_path,
        "--headless=new",
        "--use-gl=angle",
        f"--user-data-dir={user_data_dir}",
        "--window-size=1600,1200",
        f"http://localhost:{PORT}/render.html"
    ]

    print("Launching headless Chrome to render all 38 views in Black PLA...")
    proc = subprocess.Popen(args)

    success = done_event.wait(timeout=60)
    if success:
        print(f"\nSUCCESS: All {TOTAL_EXPECTED} Black PLA renders completed and saved!")
    else:
        print(f"\nTIMEOUT: Saved {saved_count}/{TOTAL_EXPECTED} renders.")

    try:
        proc.terminate()
    except Exception:
        pass
    httpd.shutdown()

if __name__ == '__main__':
    main()
