/*
 * ============================================================================
 *  BRAÇO ROBÓTICO 4-DOF — Firmware
 * ============================================================================
 *
 *  O QUE ESTE CÓDIGO FAZ
 *  ----------------------
 *  Você move dois joysticks e o braço robótico acompanha o movimento.
 *  São 4 atuadores (servomotores), um para cada articulação:
 *
 *      Base (Yaw)  → gira o braço inteiro para esquerda/direita, como um radar
 *      Ombro       → levanta e abaixa o braço principal
 *      Cotovelo    → dobra e estica o antebraço
 *      Garra       → abre e fecha a pinça
 *
 *  COMO O MOVIMENTO ACONTECE (a ideia mais importante deste código)
 *  -------------------------------------------------------------
 *  Cada junta tem DUAS variáveis de ângulo, e entender essa distinção é
 *  o que faz o braço se mover de forma suave em vez de "dar trancos":
 *
 *      angulo ALVO   → para onde a junta DEVERIA estar
 *                       É este que o joystick altera.
 *
 *      angulo ATUAL  → onde a junta ESTÁ de fato agora
 *                       A cada ciclo do loop() ele dá um PASSO na direção
 *                       do alvo (nunca chega de uma vez).
 *
 *  Exemplo com o ombro: se você joga o joystick para cima, o ALVO vai para
 *  150°. Mas o ATUAL não vira 150° de uma vez — ele caminha 90 → 96 → 102
 *  → ... → 150, sempre 15% da distância que falta. Esse "degrau" de 15%
 *  é a constante PASSO_SMOOTH. É ela que elimina o tranco.
 *
 *  Sem essa separação, o servo receberia um comando de 150° e se jogaria
 *  a 150° imediatamente, causa comum de "o braço treme ao Controlling".
 *
 *
 *  HARDWARE
 *  --------
 *  - Arduino Uno R3
 *  - Shield Joystick Duplo meARM.WDT v2.10
 *  - 4x servomotores SG90 / MG90S
 *
 *  ⚠️  ALERTA DE ENERGIA — leia antes de ligar
 *  Cada SG90 travado consome quase 650 mA. Quatro juntos passam fácil de
 *  2 A, e o regulador de 5 V da placa Arduino NÃO fornece isso: ele
 *  aguenta ~500 mA. Se alimentar os servos pelo Arduino, ele reinicia
 *  sozinho (brownout) no meio do movimento.
 *  Solução: use uma fonte 5 V / 3 A separada para os servos e ligue o GND
 *  dela junto com o GND do Arduino (fio comum obrigatório).
 *
 *
 *  PINAGEM
 *  -------
 *  Dos quatro atuadores, apenas três canais PWM são necessários porque a
 *  garra não faz PWM — os pinos chosen aqui evitam conflito com os leds.
 *
 *    Servo Base (Yaw)    D9     0..180    neutro 90
 *    Servo Ombro         D10   30..150    neutro 90
 *    Servo Cotovelo      D11   20..160    neutro 90
 *    Servo Garra         D6      0..90    neutro  0 (fechada)
 *
 *    Joystick esquerdo X A0   → Cotovelo
 *    Joystick esquerdo Y A1   → Base
 *    Joystick direito  X A2   → Garra
 *    Joystick direito  Y A3   → Ombro
 *
 *    LED de status       D13
 *
 *  ⚠️  A pinagem vem do manual do projeto. Confirme no silk da shield
 *  antes de encaixar os servos — pino errado queima o driver.
 *
 *
 *  COMANDOS NO MONITOR SERIAL (115200 baud, fim de linha "Newline")
 *  ---------------------------------------------------------------
 *    h → ajuda (lista os comandos)
 *    t → teste: varre cada servo de ponta a ponta. LIGUE O BRAÇO LONGE
 *        DA MÃO E DO ROSTO — ele se move sozinho!
 *    s → status: ângulos atuais das quatro juntas
 *    r → reset: volta todas as juntas para a posição de repouso
 *    d → liga/desliga a leitura contínua dos joysticks
 *
 * ============================================================================
 */

#include <Servo.h>

// ===========================================================================
//  1. ATUADORES — os quatro servomotores
// ===========================================================================
//  uint8_t = inteiro de 8 bits (0..255). É o tipo certo para número de
//  pino: ocupa 1 byte em vez de 2, economizando memória no Arduino.

const uint8_t PIN_SERVO_BASE    = 9;
const uint8_t PIN_SERVO_OMBRO   = 10;
const uint8_t PIN_SERVO_COTOVEL = 11;
const uint8_t PIN_SERVO_GARRA   = 6;
const uint8_t PIN_LED           = 13;

//  Um objeto Servo por motor. A biblioteca suporta no máximo 12 na Uno R3,
//  e estamos usando 4 — sobra folga para quantos quiser adicionar.
Servo servoBase;
Servo servoOmbro;
Servo servoCotovelo;
Servo servoGarra;

// ===========================================================================
//  2. JOYSTICKS — leitura analógica
// ===========================================================================
//  Joystick PS2 tem dois potenciômetros: um para X (esquerda/direita) e um
//  para Y (frente/trás). Cada um é um resistor variável que o Arduino mede
//  como tensão de 0 a 5 V, convertida em número de 0 a 1023.

const uint8_t PIN_JOY1_X = A0;   // joystick esquerdo, eixo horizontal
const uint8_t PIN_JOY1_Y = A1;   // joystick esquerdo, eixo vertical
const uint8_t PIN_JOY2_X = A2;   // joystick direito, eixo horizontal
const uint8_t PIN_JOY2_Y = A3;   // joystick direito, eixo vertical

//  Valor lido com o joystick no meio. O potenciômetro raramente dá
//  exatamente 512 (a metade de 0..1023) — pode dar 500 ou 523. Por isso
//  medimos em torno de 512 em vez de exigir 512.
const int JOY_CENTER = 512;

//  ZONA MORTA — tolerância em torno do centro.
//
//  Quando você solta o joystick, ele não volta a 512: vibração mecânica e
//  ruído elétrico fazem o valor oscilar alguns pontos. O motor le qualquer
//  mudança como comando, então o braço treme parado. A zona morta ignora
//  qualquer variação menor que este valor.
//
//  60 de 1023 ≈ 6% do curso para cada lado ≈ 12% no total. É o ponto
//  ideal: se for pequena demais, o braço treme parado; se for grande
//  demais, você perde curso de controle e não consegue comandos finos
//  de garra.
const int JOY_DEADZONE = 60;

// ===========================================================================
//  3. POSIÇÃO DE REPOUSO ("parking")
// ===========================================================================
//  Ângulo em que cada junta fica quando o programa liga e quando você
//  aperta 'r'. São ângulos CENTRAIS de propósito: a junta fica no meio do
//  curso, com folga dos dois lados. Se a repousasse num extremo, um
//  comando errado do joystick poderia travar o mecanismo contra o batente.
//
//  A garra repousa FECHADA (0°) para proteger a pinça no transporte.

const float PARK_BASE    = 90.0;
const float PARK_OMBRO   = 90.0;
const float PARK_COTOVEL = 90.0;
const float PARK_GARRA   = 0.0;

// ===========================================================================
//  4. OS DOIS ÂNGULOS DE CADA JUNTA
// ===========================================================================
//  (explicado em detalhe no cabeçalho do arquivo — leia lá se ainda não leu)

float angBase     = PARK_BASE;   // ALVO: mudado pelo joystick
float angOmbro    = PARK_OMBRO;
float angCotovelo = PARK_COTOVEL;
float angGarra    = PARK_GARRA;

float alvoBase     = PARK_BASE;  // ATUAL: caminha em direção ao alvo
float alvoOmbro    = PARK_OMBRO;
float alvoCotovelo = PARK_COTOVEL;
float alvoGarra    = PARK_GARRA;

// ===========================================================================
//  5. LIMITES MECÂNICOS — a parte mais importante para não QUEBRAR o braço
// ===========================================================================
//  Limites de ângulo REAIS de cada junta, em graus. Vêm do manual do
//  projeto e correspondem ao espaço físico entre as peças.
//
//  Não são programação arbitrária: ultrapassar estes valores faz o servo
//  forçar contra a articulação impressa em PLA. PLA trincar, entalhar ou
//  partir é o resultado — e a engrenagem da garra (peças 20, 21, 26) é o
//  ponto mais frágil do conjunto.
//
//  ⚠️  ESTES VALORES NÃO FORAM TESTADOS NO BRAÇO MONTADO.
//  Antes de usar o comando 't' (teste), confira cada limite aqui contra
//  o manual e ajuste se o ângulo real do braço diferir. É mais seguro
//  começar com uma faixa estreita e alargar aos poucos.

float limBaseMin     = 0.0,   limBaseMax     = 180.0;
float limOmbroMin    = 30.0,  limOmbroMax    = 150.0;
float limCotoveloMin = 20.0,  limCotoveloMax = 160.0;
float limGarraMin    = 0.0,   limGarraMax    = 90.0;

// ===========================================================================
//  6. GANHOS — velocidade de resposta de cada junta
// ===========================================================================
//  Quantos GRAUS o servo anda por cada unidade de leitura do joystick
//  (fora da zona morta).
//
//  Como funciona a conta, usando o ombro:
//    leitura 700 (joystick ~40% para cima), JOY_CENTER 512
//    diferença = 700 - 512 = 188
//    menos zona morta = 188 - 60 = 128
//    ângulo por ciclo = 128 × 0.020 = 2.56°
//
//  O loop() roda milhares de vezes por segundo, então esses 2.56° se
//  acumulam rápido e o braço responde bem. Diminuir o ganho = mais lento
//  e mais suave. Aumentar = mais rápido e mais tremor.
//
//  A garra tem ganho pela metade (0.010) porque tem curso curto (0..90°)
//  e engrenagem: movimento rápido demais aqui faz os dentes de PLA
//  escorregar ou o servo forçar o mecanismo.

const float GANHO_BASE    = 0.020;
const float GANHO_OMBRO   = 0.020;
const float GANHO_COTOVEL = 0.020;
const float GANHO_GARRA   = 0.010;

// ===========================================================================
//  7. SUAVIZAÇÃO — o degrau que elimina o tranco
// ===========================================================================
//  Fração da distância restante que a junta percorre a cada volta do loop.
//
//  Com 0.15: se faltam 60° para o alvo, a junta anda 9° por ciclo.
//  Com 0.50: ela andaria 30° por ciclo — bem mais rápida e mais brusca.
//
//  Menor = movimento mais macio e "pesado", mas com atraso (o braço parece
//  preguiçoso). Maior = resposta rápida, mais tremida.
//
//  0.15 é um bom equilíbrio para braço com-motor pequeno como o SG90.

const float PASSO_SMOOTH = 0.15;

// ===========================================================================
//  8. PULSO DO SERVO — o que realmente comanda o motor
// ===========================================================================
//  Um servo NÃO recebe "90 graus". Ele recebe um PULSO de largura variável,
//  repetido a cada ~20 ms. A posição do eixo depende da duração desse pulso:
//
//      pulso curto (~544 µs) → eixo todo para um lado   (0°)
//      pulso médio (~1500 µs) → eixo no meio            (90°)
//      pulso longo (~2336 µs) → eixo todo para o outro (180°)
//
//  Por que usar microssegundos em vez de servo.write(graus)?
//  servo.write() só aceita INTEIROS de 0 a 180, então guarda a posição com
//  resolução de 1 grau — o servo fica parado dentro dessa faixa, sem
// Interpolação. writeMicroseconds() dá resolução de ~0.3°, o servo pode
//  assumir qualquer posição intermediária. Para um braço com 4 juntas
//  combinando, essa diferença é visível: os cantos ficam mais contínuos.
//
//  Os valores 544 e 2336 são o padrão do SG90. Alguns SG90 aceitam 500 e
//  2400; outros servos (MG996R) usam 1000..2000 e NÃO funcionam com estes
//  números. Se o servo tremer muito ou aquecer, ajuste estes dois valores.

const float US_MIN = 544.0;   // microssegundos para 0°
const float US_MAX = 2336.0;  // microssegundos para 180°

//  Velocidade do monitor serial. 115200 é padrão e mostra bem no Serial
//  Monitor do Arduino IDE.
const unsigned long BAUD = 115200;

//  Liga/desliga a leitura contínua dos joysticks pelo comando 'd'.
bool debugOn = true;

// ===========================================================================
//  9. FUNÇÕES AUXILIARES
// ===========================================================================

//  --- Prende um valor dentro de uma faixa (o "constrain" nativo do Arduino)
//
//  Se o ângulo calculado for menor que o mínimo, devolve o mínimo.
//  Se for maior que o máximo, devolve o máximo.
//  Caso contrário devolve o próprio valor.
//
//  Esta função é a barreira que impede o servo de ir além do limite
//  mecânico — ela é a última proteção antes do comando chegar ao motor.

float constrainAngle(float v, float lo, float hi) {
  if (v < lo) return lo;
  if (v > hi) return hi;
  return v;
}

//  --- Converte ângulo em graus para o pulso em microssegundos
//
//  Faz uma interpolação linear: 0° → US_MIN, 180° → US_MAX, e tudo
//  entre eles proporcionalmente. Exemplo, 90° (metade do caminho):
//
//      544 + (90/180) × (2336 - 544)
//      544 + 0.5 × 1792
//      544 + 896 = 1440 µs
//
//  Depois travamos o resultado entre US_MIN e US_MAX, para que uma falha
//  de cálculo nunca envie um pulso fora da faixa segura do servo.

int anguloParaUs(float ang) {
  float us = US_MIN + (ang / 180.0) * (US_MAX - US_MIN);
  if (us < US_MIN) us = US_MIN;
  if (us > US_MAX) us = US_MAX;
  return (int)us;
}

//  --- Um degrau de suavização
//
//  Calcula a distância que falta (alvo - atual) e devolve um ponto 15%
//  adiante no caminho.
//
//    faltando 60°  → anda 9°
//    faltando 10°  → anda 1.5°
//    faltando 0.01° ou menos → chega (e "trava" no alvo, sem ficar
//                            oscilando indefinidamente em torno dele)
//
//  Guardar a última condição (0.01°) evita um problema sutil: se não
//  existisse, a junta ficaria pulando os últimos milésimos de grau para
//  sempre, gastando CPU sem parar. Com ela, o servo descansa no alvo.

float suaviza(float atual, float alvo) {
  float d = alvo - atual;
  if (d > -0.01 && d < 0.01) return alvo;
  return atual + d * PASSO_SMOOTH;
}

//  --- Converte uma faixa de valores em outra (versão do "map" nativo)
//
//  Usada só para o texto do monitor serial: transforma a leitura crua
//  0..1023 em -100..+100, muito mais fácil de ler que "512".
//  Não afeta o controle dos servos.

int mapADC(int x, int in_min, int in_max, int out_min, int out_max) {
  return (x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min;
}

// ===========================================================================
//  10. INICIALIZAÇÃO SEGURA — a sequência mais importante do programa
// ===========================================================================
//  Esta função roda UMA vez, no setup(). A ordem das operações não é
//  aleatória — é uma sequência de segurança:
//
//    Passo 1: só a GARRA se move, e vai para FECHADA.
//
//      Os quatro servos são ligados ao mesmo tempo, mas ainda estão em
//      posição aleatória (o servo não tem memória de onde parou). Ao
//      mandar o braço inteiro para o centro de uma vez, as quatro juntas
//      giram simultaneamente e podem COLIDIR entre si ou com o chassi —
//      o braço se contorce, e o servo força contra o batente.
//
//      Por isso movemos uma junta por vez, com pausa entre elas. A garra
//      primeiro porque é a única que se fecha sem mover o resto do braço:
//      não há risco de colisão, e ela fica protegida desde o início.
//
//    Passo 2: base, ombro e cotovelo juntas, já que não têm colisão
//             entre si nesta posição.
//
//    Passo 3: LED pisca 3 vezes = "programa pronto, pode mexer".
//
//  Os delays não são desperdício: dão tempo de os motores chegarem
//  fisicamente à posição antes do próximo comando.

void sequenceInicial() {
  // --- Passo 1: garra fecha sozinha
  alvoGarra = PARK_GARRA;
  angGarra = PARK_GARRA;
  servoGarra.write(anguloParaUs(angGarra));
  delay(300);

  // --- Passo 2: as três juntas grandes vão para o centro
  //     Note que alteramos ALVO e ATUAL juntas, para a junta ir
  //     exatamente ao repouso em um passo só (sem suavizar a partida).
  alvoBase = PARK_BASE;
  alvoOmbro = PARK_OMBRO;
  alvoCotovelo = PARK_COTOVEL;

  angBase = alvoBase;
  angOmbro = alvoOmbro;
  angCotovelo = alvoCotovelo;

  servoBase.write(anguloParaUs(angBase));
  servoOmbro.write(anguloParaUs(angOmbro));
  servoCotovelo.write(anguloParaUs(angCotovelo));
  delay(300);

  // --- Passo 3: 3 piscadas de confirmação
  for (int i = 0; i < 3; i++) {
    digitalWrite(PIN_LED, HIGH);
    delay(120);
    digitalWrite(PIN_LED, LOW);
    delay(120);
  }
}

// ===========================================================================
//  11. MODO DE TESTE — verificação de hardware
// ===========================================================================
//  Serve para conferir, uma junta por vez, se o servo está:
//    - conectado no pino certo
//    - girando no sentido certo
//    - BATENDO nos limites mecânicos antes de chegar no ângulo do software
//
//  ⚠️  PERIGO: este modo varre o servo de um limite ao outro sozinho.
//  Tenha o braço totalmente livre, sem mão, rosto ou objeto perto.
//  Rode o comando 't' com o braço ainda sobre a mesa, se possível.

//  Percorre um servo do mínimo ao máximo e volta, devagar.
//  Usa & na assinatura porque recebe uma REFERÊNCIA ao objeto Servo — assim
//  mexe no servo original sem precisar criar uma cópia.
void testaServo(Servo &s, float min, float max, const __FlashStringHelper *nome) {
  Serial.print(F("testando ")); Serial.println(nome);

  //  Ida: 31 passos do mínimo ao máximo.
  //  O i/30.0 (e não i/30) é obrigatório: divisão de inteiros em C++
  //  arredonda para baixo e devolveria sempre 0, travando o servo.
  for (int i = 0; i <= 30; i++) {
    float a = min + (max - min) * i / 30.0;
    s.write(anguloParaUs(a));
    delay(40);   // 40 ms entre passos: rápido o bastante para não
                 // demorar, lento o bastante para o servo acompanhar
  }

  //  Volta: mesmos 31 passos, agora do máximo ao mínimo.
  for (int i = 30; i >= 0; i--) {
    float a = min + (max - min) * i / 30.0;
    s.write(anguloParaUs(a));
    delay(40);
  }

  Serial.println(F("ok"));
}

//  Orquestra o teste dos quatro servos, um por vez, e volta ao repouso.
void modoTeste() {
  Serial.println(F("=== MODO TESTE ==="));
  Serial.println(F("libere o mecanismo: o braco vai se mover!"));
  delay(2000);   // 2 s de aviso antes de qualquer movimento

  testaServo(servoBase, limBaseMin, limBaseMax, F("base"));
  testaServo(servoOmbro, limOmbroMin, limOmbroMax, F("ombro"));
  testaServo(servoCotovelo, limCotoveloMin, limCotoveloMax, F("cotovelo"));
  testaServo(servoGarra, limGarraMin, limGarraMax, F("garra"));

  sequenceInicial();   // devolve tudo ao repouso ao final
  Serial.println(F("=== TESTE CONCLUIDO ==="));
}

// ===========================================================================
//  12. RESET — voltar à posição de repouso
// ===========================================================================
//  Aperte 'r' se o braço ficou numa posição travada.
//
//  Aqui alteramos ALVO e ATUAL juntos. Isso é diferente do loop normal, e
//  é proposital: se ajustássemos só o alvo, a junta percorreria todo o
//  caminho suavizadamente e o.reset levaria alguns segundos. Zerando os dois,
//  o servo vai direto ao repouso. Para um comando de emergência, rapidez
//  vale mais que suavidade.
//
//  ⚠️  Zerando os dois, um braço muito longe do repouso vai saltar de
//  posição. Segure o braço se ele estiver estendido.

void resetPark() {
  alvoBase = PARK_BASE;
  alvoOmbro = PARK_OMBRO;
  alvoCotovelo = PARK_COTOVEL;
  alvoGarra = PARK_GARRA;

  angBase = alvoBase;
  angOmbro = alvoOmbro;
  angCotovelo = alvoCotovelo;
  angGarra = alvoGarra;

  servoBase.write(anguloParaUs(angBase));
  servoOmbro.write(anguloParaUs(angOmbro));
  servoCotovelo.write(anguloParaUs(angCotovelo));
  servoGarra.write(anguloParaUs(angGarra));
  Serial.println(F("reset para posicao de repouso"));
}

//  Lista os comandos aceitos. Chamada no setup e ao apertar 'h'.
void mostraAjuda() {
  Serial.println(F("comandos: h=ajuda t=teste s=status r=reset d=debug"));
}

//  Mostra o ângulo ATUAL das quatro juntas (não o alvo — o que o servo
//  está fazendo de verdade agora).
void status() {
  Serial.print(F("B=")); Serial.print((int)angBase);
  Serial.print(F(" O=")); Serial.print((int)angOmbro);
  Serial.print(F(" C=")); Serial.print((int)angCotovelo);
  Serial.print(F(" G=")); Serial.println((int)angGarra);
}

// ===========================================================================
//  13. LEITURA DO JOYSTICK → ÂNGULO ALVO
// ===========================================================================

//  Aplica o comando de um eixo do joystick sobre um ângulo alvo.
//
//  Parâmetros:
//    leitura  → valor bruto do analogRead (0..1023)
//    atual    → o ângulo alvo no momento (quem é alterado)
//    min/max  → limites mecânicos desta junta
//    ganho    → quantos graus por unidade de leitura
//
//  A lógica em três passos:
//
//    1. e = leitura - 512        → "quanto o joystick saiu do centro"
//    2. se |e| > zona morta       → está dentro de folga, não faz nada
//    3. senão                    → tira a zona morta e aplica o ganho
//
//  O passo 3 tem um detalhe: subtraímos a zona morta de novo (linha do
//  "excesso"). Isso é para a junta não dar um salto ao cruzar a zona
//  morta. Sem isso, quando o joystick sai de 51 para 61 (a primeira leitura
//  que conta), o servo saltaria de 0° direto para o valor cheio do ganho —
//  um tranco no instante em que você começa a empurrar. Subtraindo a zona
//  morta, o movimento começa do zero, onde quer que o joystick estava.

float aplicaEixo(int leitura, float atual, float min, float max, float ganho) {
  int e = leitura - JOY_CENTER;

  if (abs(e) > JOY_DEADZONE) {
    int excesso = e - (e > 0 ? JOY_DEADZONE : -JOY_DEADZONE);
    float delta = excesso * ganho;
    return constrainAngle(atual + delta, min, max);
  }

  return atual;   // joystick no centro: segura a posição
}

// ===========================================================================
//  14. LOOP — roda para sempre, milhares de vezes por segundo
// ===========================================================================
//  Esta é a parte que realmente comanda os servos. O Arduino não tem
//  sistema operacional nem tarefas: quando setup() acaba, ele entra aqui
//  e fica repetindo este bloco até ser desligado.

void loop() {

  //  --- ETAPA A: Ler comandos do serial -------------------------------
  //
  //  while (Serial.available() > 0) → "enquanto houver byte esperando".
  //  O while com corpo de uma linha dispensa as chaves { }.
  //
  //  Importante:processar comandos aqui NÃO trava o programa. Se você
  //  usar delay() com dados chegando, o Arduino perde bytes. Por isso o
  //  buffer é lido e a ação é executada sem parar o loop.
  //
  //  Mas note: o comando 't' chama modoTeste(), que tem delay() de vários
  //  segundos. Durante o teste o loop fica parado, então o texto enviado
  //  nesse intervalo é perdido. Sem problema — durante o teste não há
  //  joystick para ler de qualquer forma.

  while (Serial.available() > 0) {
    char c = Serial.read();
    switch (c) {
      case 'h': case 'H': mostraAjuda(); break;
      case 't': case 'T': modoTeste();   break;
      case 's': case 'S': status();      break;
      case 'r': case 'R': resetPark();   break;
      case 'd': case 'D':
        debugOn = !debugOn;   // inverte o valor (liga<->desliga)
        Serial.print(F("debug "));
        Serial.println(debugOn ? F("ON") : F("OFF"));
        break;
    }
  }

  //  --- ETAPA B: Ler os quatro potenciômetros -------------------------
  //
  //  analogRead() devolve um número de 0 a 1023, onde 0 = 0 V e
  //  1023 = 5 V. Leva cerca de 100 microssegundos cada.
  //
  //  Guardamos os quatro antes de usar, para todos lerem o mesmo instante.
  //  Se lêssemos um, usássemos, lêssemos outro, teríamos um atraso de
  //  microssegundos entre as juntas — irrelevante aqui, mas ler juntos
  //  deixa o código mais limpo e consistente.

  int joy1X = analogRead(PIN_JOY1_X);
  int joy1Y = analogRead(PIN_JOY1_Y);
  int joy2X = analogRead(PIN_JOY2_X);
  int joy2Y = analogRead(PIN_JOY2_Y);

  //  --- ETAPA C: joystick altera os ÂNGULOS ALVO ----------------------
  //
  //  Repare que alteramos sempre alvo* (nunca ang*). O joystick decide
  //  ONDE a junta deve estar; a suavização (próxima etapa) decide COMO
  //  chegar lá.
  //
  //  Mapeamento dos controles:
  //    joy1Y → Base       (esquerdo, vertical)   gira o braço
  //    joy2Y → Ombro      (direito, vertical)    levanta/abaixa
  //    joy1X → Cotovelo   (esquerdo, horizontal) dobra/estica
  //    joy2X → Garra      (direito, horizontal)  abre/fecha
  //
  //  Esta configuração é uma escolha de projeto, não uma exigência
  //  elétrica. Se ficar mais confortável trocar, por exemplo, a base para o
  //  joystick direito, basta trocar a linha inteira.

  alvoBase = aplicaEixo(joy1Y, alvoBase, limBaseMin, limBaseMax, GANHO_BASE);
  alvoOmbro = aplicaEixo(joy2Y, alvoOmbro, limOmbroMin, limOmbroMax, GANHO_OMBRO);

  alvoCotovelo = aplicaEixo(joy1X, alvoCotovelo, limCotoveloMin, limCotoveloMax, GANHO_COTOVEL);

  //  O sinal de MENOS antes do ganho INVERTE este eixo. O atuador da garra
  //  tem curso de apenas 90°, e invertê-lo ficaria ruim para a ergonomia:
  //  preferimos que empurrar para BAIXO abra a pinça (movimento de soltar
  //  para pegar) e para CIMA feche (movimento de segurar).
  //  Se a garra funcionar ao contrário no seu braço, remova o sinal de menos.
  alvoGarra = aplicaEixo(joy2X, alvoGarra, limGarraMin, limGarraMax, -GANHO_GARRA);

  //  --- ETAPA D: SUAVIZAÇÃO — atual caminha até o alvo -----------------
  //
  //  Aqui está o movimento suave. Cada junta dá um passo de 15% da
  //  distância que falta, a cada volta do loop. Como o loop repete
  //  milhares de vezes por segundo, o resultado é um movimento contínuo.
  //
  //  É aqui que as linhas ang* = suaviza(ang*, alvo*) fazem sentido: o
  //  alvo está longe, mas o atual parte em direção a ele um degrau de cada
  //  vez.

  angBase     = suaviza(angBase, alvoBase);
  angOmbro    = suaviza(angOmbro, alvoOmbro);
  angCotovelo = suaviza(angCotovelo, alvoCotovelo);
  angGarra    = suaviza(angGarra, alvoGarra);

  //  --- ETAPA E: enviar aos servos ------------------------------------
  //
  //  Escrevemos o ÂNGULO ATUAL (não o alvo). É o que produz o movimento
  //  gradual: se escrevêssemos o alvo, o servo saltaria direto.
  //
  //  Note que escrevemos os 4 a CADA volta do loop. A biblioteca servo
  //  cuida de gerar o pulso correto por baixo; só informamos a posição.

  servoBase.write(anguloParaUs(angBase));
  servoOmbro.write(anguloParaUs(angOmbro));
  servoCotovelo.write(anguloParaUs(angCotovelo));
  servoGarra.write(anguloParaUs(angGarra));

  //  --- ETAPA F: leitura dos joysticks no monitor serial --------------
  //
  //  static guarda o valor entre voltas do loop sem alocar memória nova a
  //  cada iteração (seria o mesmo que declarar "unsigned long ultimoPrint"
  //  antes do loop). O static garante que ele existe uma única vez.
  //
  //  A cada 100 ms (10 Hz) mostramos a posição dos joysticks. 10 Hz é
  //  rápido o bastante para ver a resposta, lento o bastante para não
  //  inundar o monitor serial e deixar o Arduino mais lento para os
  //  servos.
  //
  //  Usamos mapADC para mostrar -100..+100 em vez de 0..1023: muito mais
  //  fácil de ver se o joystick está em -30 ou +70.

  static unsigned long ultimoPrint = 0;
  if (debugOn && millis() - ultimoPrint >= 100) {
    ultimoPrint = millis();
    Serial.print(F("joy1=")); Serial.print(mapADC(joy1X, 0, 1023, -100, 100));
    Serial.print(F("/"));     Serial.print(mapADC(joy1Y, 0, 1023, -100, 100));
    Serial.print(F(" joy2=")); Serial.print(mapADC(joy2X, 0, 1023, -100, 100));
    Serial.print(F("/"));     Serial.println(mapADC(joy2Y, 0, 1023, -100, 100));
  }
}

// ===========================================================================
//  15. SETUP — roda UMA VEZ ao ligar/resetar
// ===========================================================================
//  O Arduino executa setup(), depois entra em loop() para sempre.
//  Tudo que precisa acontecer uma única vez fica aqui.

void setup() {
  Serial.begin(BAUD);   // inicia o monitor serial a 115200 baud

  pinMode(PIN_LED, OUTPUT);   // declara o LED como saída

  //  attach() diz ao servo "use este pino". É obrigatório — sem attach o
  //  servo fica sem comando e não se move.
  servoBase.attach(PIN_SERVO_BASE);
  servoOmbro.attach(PIN_SERVO_OMBRO);
  servoCotovelo.attach(PIN_SERVO_COTOVEL);
  servoGarra.attach(PIN_SERVO_GARRA);

  //  Posiciona o braço com segurança e pisca o LED 3 vezes.
  sequenceInicial();

  //  Fala no serial o que está pronto e quais comandos existem.
  Serial.println(F("braco_4dof pronto"));
  mostraAjuda();
}

// ===========================================================================
//  FIM DO ARQUIVO
// ===========================================================================
//  RESUMO DO QUE AJUSTAR PRIMEIRO, na ordem de importância:
//
//  1. limBaseMin/Max, limOmbroMin/Max, limCotoveloMin/Max — ESTE é o mais
//     importante. Errar aqui quebra o PLA e a engrenagem. Confira com o
//     manual, comece com faixa estreita, alargue aos poucos.
//
//  2. US_MIN / US_MAX — se o servo tremer ou aquecer, seu modelo usa
//     outros valores (ex.: MG996R usa 1000/2000).
//
//  3. Sinais de GANHO_* — se uma junta se mover ao contrário, ponha sinal
//     de menos antes do ganho daquela linha no loop().
//
//  4. PASSO_SMOOTH — menor = mais suave e lento. Maior = mais rápido e
//     mais trêmulo.
//
//  5. JOY_DEADZONE — se o braço tremer parado, aumente. Se estiver "preso"
//     e não responder a comandos pequenos, diminua.
//
//  LEMBRE-SE: com 4 SG90, alimente os servos por fonte 5 V / 3 A separada
//  com GND comum ao Arduino. O regulador da placa não dá conta.
