import os
import subprocess
import time
import pypdfium2 as pdfium

chrome_path = r"C:\Program Files\Google\Chrome\Application\chrome.exe"
html_path = r"d:\braco-robortico\manual_tecnico_braco_robotico.html"
pdf_path = r"d:\braco-robortico\Manual_Braco_Robotico_Bambu_A1.pdf"
pages_dir = r"d:\braco-robortico\pdf_assets\pdf_pages"

os.makedirs(pages_dir, exist_ok=True)

print("Iniciando geracao do PDF com Google Chrome headless...")
cmd = [
    chrome_path,
    "--headless=new",
    "--disable-gpu",
    "--run-all-compositor-stages-before-draw",
    "--no-pdf-header-footer",
    f"--print-to-pdf={pdf_path}",
    html_path
]

start_time = time.time()
res = subprocess.run(cmd, capture_output=True, text=True)
elapsed = time.time() - start_time

if res.returncode != 0:
    print(f"Erro ao executar Chrome: {res.stderr}")
    exit(res.returncode)

file_size = os.path.getsize(pdf_path)
print(f"PDF gerado com sucesso em {elapsed:.2f}s! Tamanho: {file_size:,} bytes")

print("Validando e extraindo paginas renderizadas...")
pdf = pdfium.PdfDocument(pdf_path)
page_count = len(pdf)
print(f"Total de paginas no PDF: {page_count}")

for i in range(page_count):
    page = pdf[i]
    # Render at 2x scale for crisp preview
    img = page.render(scale=2.0).to_pil()
    out_file = os.path.join(pages_dir, f"page_{i+1:02d}.png")
    img.save(out_file)
    print(f"  [x] Pagina {i+1:02d} renderizada -> {out_file} ({img.size[0]}x{img.size[1]}px)")

# Also update page1_preview if it exists
page1_preview_path = r"d:\braco-robortico\pdf_assets\page1_preview.png"
pdf[0].render(scale=2.0).to_pil().save(page1_preview_path)
print(f"Preview da Capa atualizado: {page1_preview_path}")

print("Concluido com sucesso!")
