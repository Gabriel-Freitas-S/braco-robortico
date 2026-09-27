A análise técnica do manual revela que o documento possui **erros graves de engenharia mecânica, inversão de peças no passo a passo, falhas nos modelos 3D renderizados e divergências na lista de parafusos**. Se você seguir o manual literalmente, o braço ficará mecanicamente travado ou com peças invertidas.

---

## 1. Erros na Montagem dos Servomotores

* **Servo 1 (Base/Yaw) — Inversão entre as peças 12 e 16 (Etapa 1, pág. 12):**
* **O erro no manual:** O Passo 1.3 manda encaixar o corpo do Servo 1 no nicho retangular da peça `12_Mesa_Giratoria_Turntable` e usar a peça `25_Trava_Horn_Servo_Base` para prender as abas do motor. Já o Passo 1.4 manda fixar a cruzeta (*horn*) embaixo da peça `16_Berco_Servo_Rotacao_Base`, mas no Passo 1.5 volta a dizer para encaixar a peça 12 no eixo estriado do servo.

* **Por que está errado:** As funções das peças 12 e 16 foram trocadas no texto. Conforme o catálogo (págs. 6 e 7), quem possui o nicho retangular fixo para alojar o corpo do SG90 é a peça **16 (`Berco_Servo_Rotacao_Base`)**, que deve ficar presa nos 4 espaçadores metálicos de 24 mm. Já a peça **12 (`Mesa_Giratoria_Turntable`)** é o prato superior móvel que recebe o *horn* (cruzeta) solidarizado pela peça **25 (`Trava_Horn_Servo_Base`)** e gira sobre o chassi. Além disso, o Passo 1.2 esquece de instruir a fixação da peça 16 no topo dos 4 espaçadores metálicos.

* **Servos 2 e 3 (Ombro e Cotovelo) — Colisão física e contradição de lado (Etapa 2, pág. 13):**
* **Colisão interna entre os motores:** Os Passos 2.1 e 2.2 mandam inserir os dois servos SG90 **pelo lado interno** das torres 09 e 11 (em espelho, apontando para fora) e o Passo 2.3 une as duas torres com parafusos **M3 x 30 mm**. Como cada servo SG90 tem cerca de 22,5 mm de profundidade até as abas (mais a saída do cabo), dois motores montados de dentro para fora na mesma linha colidem traseira com traseira dentro de um espaçamento limitado por parafusos de 30 mm.

* **Contradição entre texto e imagens 3D:** Enquanto os Passos 2.1 e 2.2 dizem que o Servo 2 fica na torre direita e o Servo 3 na torre esquerda, os renders 3D da capa (pág. 1), Figura 1.1 (pág. 2) e Etapa 1 (pág. 12) mostram **as duas alavancas brancas (horns) saindo do mesmo lado (torre esquerda)**, uma posicionada acima da outra, com um dos servos montado na vertical.

* **Peça 07 com descrição conflitante:** Na página 2 e na Etapa 2, o projeto afirma que os servos do ombro e cotovelo ficam concentrados nas torres da base. Porém, no catálogo (pág. 5), a peça `07_Braco_Principal_Direito` é descrita como tendo *"Encaixe para servo"* e *"berço de fixação do servo motor do cotovelo"*.

* **Servo 4 (Garra) — Eixo apontado para o lado errado e alavanca desconectada (Etapa 4, pág. 15):**
* **Orientação incorreta do eixo (Passo 4.1):** O texto afirma que *"O eixo estriado do servo deve apontar para frente"*. Se o eixo apontar para a frente (na direção dos dedos da pinça), o giro do servo será perpendicular ao plano de abertura dos dedos. Como mostram os próprios renders da capa (pág. 1) e da Vista Lateral (pág. 2), o servo da garra trabalha na vertical (com o eixo estriado apontando **para baixo/cima**, perpendicular à base da garra 19).

* **Peça 26 solta na montagem (Passos 4.2 a 4.5):** O Passo 4.2 manda instalar a alavanca `26_Engrenagem_Acionadora_Garra` no eixo do Servo 4, mas os Passos 4.3, 4.4 e 4.5 montam os dedos (20 e 21), a guia (27) e a biela curta (28) **sem nunca conectar a alavanca 26 à biela 28 ou aos dedos**.

---

## 2. Erros de Cinemática e Montagem das Peças

* **Cinemática colapsada nos Renders 3D (Capa, pág. 1, pág. 2 e pág. 14):**
* **Braço montado ao contrário no 3D:** Na imagem de capa (pág. 1) e na Vista Lateral (pág. 2), o braço foi montado de forma geometricamente impossível. As hastes do braço principal sobem verticalmente e o antebraço **desce de volta para a base**, deixando a garra colada na frente das torres inferiores e formando uma torre triangular vazia no topo.

* **Treliça traseira travada:** No render da capa (pág. 1), a haste vertical traseira está presa na torre e ligada ao braço por duas barras horizontais paralelas, formando um trapézio rígido que impediria qualquer movimento do braço.

* **Render da Etapa 3 inutilizável (pág. 14):** A ilustração da Etapa 3 mostra todas as 9 hastes empilhadas umas sobre as outras verticalmente e uma peça solta flutuando à direita, sem representar a montagem.

* **Travamento mecânico do Paralelogramo nos Passos 3.2, 3.3 e 3.4 (pág. 14):**
* **Falta de ancoragem fixa na torre:** Para que o paralelogramo mantenha a garra nivelada horizontalmente, um dos tirantes precisa ser ancorado em um **ponto fixo da torre (09 ou 11)** até o `10_Balancim_Triangular_Articulador`. O manual não conecta nenhum tirante ao pivô fixo da torre.

* **Triângulo rígido no antebraço:** O Passo 3.3 manda conectar tanto o `13_Tirante_Superior_Garra` quanto o `04_Tirante_Paralelogramo_Medio` juntos no mesmo **Pivô 3 (Superior)** do balancim triangular (10). Em seguida, o Passo 3.4 manda ligar o tirante 04 no meio do antebraço, enquanto o antebraço já está articulado no mesmo eixo do balancim (10) no topo do braço principal. Isso forma um triângulo fechado que **trava o cotovelo e o balancim**, além de ser dimensionalmente incompatível, pois o tirante 04 mede **94,8 mm** e o antebraço inteiro mede apenas **81,6 mm a 85,8 mm** (pág. 4 e 5).

* **Nomes e miniaturas 3D trocados no Catálogo (págs. 7, 8 e 9):**
* As peças `19_Garra_Base_Suporte_Fixo` (pág. 8), `22_Biela_Conexao_Horn_Direito` (pág. 9), `23_Biela_Conexao_Horn_Esquerdo` (pág. 9) e `24_Suporte_Encaixe_Servo_Garra` (pág. 9) exibem todas a **mesma geometria de colar retangular vazado** (suporte de retenção do corpo do servo SG90), o que não condiz com as descrições de "biela de tração" ou "base onde se articulam os dedos".

* Já a peça `15_Suporte_Cabeca_Garra` (pág. 7), descrita como *"Placa de ancoragem do micro servomotor"*, exibe na imagem 3D uma haste fina com furos nas pontas.

---

## 3. Divergências na Lista de Materiais (BOM) e Impressão 3D

| Item / Fixador | Quantidade na BOM (Pág. 3) | Soma Real nas Etapas (Págs. 12–15) | Problema Identificado |
| --- | --- | --- | --- |
| **Parafuso M3 x 30 mm** | 4 un

 | 2 un (Etapa 2)

 | Sobram 2 unidades na BOM.

 |
| **Parafuso M3 x 16 mm** | 5 un

 | 4 un listadas na Etapa 3 (3 citadas no texto)

 | Divergência entre BOM, cabeçalho e texto da Etapa 3.

 |
| **Parafuso M3 x 12 mm** | 12 un

 | 14 un (4 na Etapa 1 + 6 na Etapa 3 + 4 no texto da Etapa 4)

 | **Faltam 2 parafusos** na BOM (a Etapa 4 lista 2 no topo, mas pede 4 nos Passos 4.4 e 4.6).

 |
| **Parafuso M3 x 10 mm** | 8 un

 | 10 un (4 na Etapa 1 + 2 na Etapa 2 + 4 na Etapa 4)

 | **Faltam 2 parafusos** na BOM.

 |
| **Porca Nyloc M3** | 15 un

 | 20 un (4 na Etapa 2 + 10 na Etapa 3 + 6 na Etapa 4)

 | **Faltam 5 porcas auto-travantes** na BOM.

 |
| **Arruela de Nylon M3** | 16 un

 | 24 un (4 na Etapa 1 + 14 na Etapa 3 + 6 na Etapa 4)

 | **Faltam 8 arruelas** na BOM.

 |

* **Conflito de orientação de impressão das Torres 09 e 11:** No catálogo (pág. 6), a orientação indicada para as peças 09 e 11 é **"Base apoiada na mesa, vertical"**. Já na tabela de fatiamento da página 11, a instrução muda para **"Deitadas no plano XY + Tree Slim"** (deitadas são muito mais resistentes contra quebra das espigas inferiores e dos mancais).

---

## 4. Como Corrigir a Montagem na Prática

1. **Base (Eixo Yaw):** Prenda os 4 espaçadores M3x24mm na base `01`, fixe o berço do servo (`16`) no topo dos espaçadores com o Servo 1 apontando para cima, prenda a cruzeta (*horn*) embaixo da mesa giratória (`12`) usando a trava (`25`), alinhe o servo em **90°** e parafuse a mesa `12` no eixo do servo.

2. **Servos das Torres (Ombro e Cotovelo):** Posicione os servos nas torres `09` e `11` usando os colares retangulares de retenção pelo lado externo (ou verifique o recorte exato dos seus arquivos STL) para que os corpos dos servos não colidam no centro ao apertar os parafusos M3x30mm no espaçador `14`. Centralize ambos eletronicamente em **90°** antes de parafusar os *horns*.

3. **Duplo Paralelogramo (Braço e Antebraço):**

* **Acionamento do Ombro (Servo Direto):** A alavanca do Servo 2 movimenta o braço principal (`02`/`07`) através da haste de acionamento (`05`).

* **Acionamento do Cotovelo:** A alavanca do Servo 3 (na base) conecta-se ao tirante que vai até a **extensão traseira do antebraço**, fazendo o antebraço subir e descer livremente sobre o pivô do cotovelo.

* **Nivelamento da Garra:** O outro tirante longo deve sair do **furo fixo na torre da base** e subir paralelo ao braço principal até o **Pivô Traseiro do Balancim Triangular (`10`)** (cujo pivô central fica no cotovelo). Do **Pivô Superior do Balancim (`10`)**, sai exclusivamente o `13_Tirante_Superior_Garra` paralelo ao antebraço até o suporte da garra (`15`).

1. **Garra (Servo 4):** Monte o Servo 4 com o eixo perpendicular ao plano dos dedos (`20` e `21`), calibre-o em **0°** com os dedos totalmente fechados e conecte a alavanca acionadora (`26`) à biela curta (`28`) para transmitir o movimento aos dedos engrenados.
