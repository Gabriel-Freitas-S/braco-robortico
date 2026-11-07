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
 *  150°. Mas o ATUAL não vira 150° de uma vez — ele avança a uma velocidade
 *  FIXA (VEL_SUAVE, em graus/segundo), nunca de um salto. Essa constante é
 *  o que elimina o tranco.
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
 *    Joystick esquerdo X A0   → Base (esquerda/direita)
 *    Joystick esquerdo Y A1   → Cotovelo (frente/trás)
 *    Joystick direito  X A3   → Garra (esquerda fecha / direita abre)
 *    Joystick direito  Y A2   → Ombro (frente/trás)
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
//  Pinos digitais livres candidatos ao 4º conector de servo na Shield Joystick
//  (sem D2, D3 e D4, que na Shield PD07 da Casa da Robótica são botões e LED):
const uint8_t PIN_BASE_ALT_D5   = 5;
const uint8_t PIN_BASE_ALT_D7   = 7;
const uint8_t PIN_BASE_ALT_D8   = 8;
const uint8_t PIN_BASE_ALT_D12  = 12;

const uint8_t PIN_SERVO_OMBRO   = 10;
const uint8_t PIN_SERVO_COTOVEL = 11;
const uint8_t PIN_SERVO_GARRA   = 6;
const uint8_t PIN_LED           = 13;

Servo servoBase;
Servo servoBaseD5;
Servo servoBaseD7;
Servo servoBaseD8;
Servo servoBaseD12;
Servo servoOmbro;
Servo servoCotovelo;
Servo servoGarra;

void escreveBase(int pulsoUs) {
  if (!servoBase.attached()) {
    servoBase.attach(PIN_SERVO_BASE);
    servoBaseD5.attach(PIN_BASE_ALT_D5);
    servoBaseD7.attach(PIN_BASE_ALT_D7);
    servoBaseD8.attach(PIN_BASE_ALT_D8);
    servoBaseD12.attach(PIN_BASE_ALT_D12);
  }
  servoBase.writeMicroseconds(pulsoUs);
  servoBaseD5.writeMicroseconds(pulsoUs);
  servoBaseD7.writeMicroseconds(pulsoUs);
  servoBaseD8.writeMicroseconds(pulsoUs);
  servoBaseD12.writeMicroseconds(pulsoUs);
}

void desligaBase() {
  if (servoBase.attached()) {
    servoBase.detach();
    servoBaseD5.detach();
    servoBaseD7.detach();
    servoBaseD8.detach();
    servoBaseD12.detach();
  }
}

// ===========================================================================
//  2. JOYSTICKS — leitura analógica (Shield Casa da Robótica Ref: PD07)
// ===========================================================================

const uint8_t PIN_JOY1_X = A0;   // joystick esquerdo: esquerda/direita → Base
const uint8_t PIN_JOY1_Y = A1;   // joystick esquerdo: frente/trás      → Cotovelo
const uint8_t PIN_JOY2_X = A3;   // joystick direito:  esquerda/direita → Garra (esq=fecha, dir=abre)
const uint8_t PIN_JOY2_Y = A2;   // joystick direito:  frente/trás      → Ombro

const int JOY_CENTER = 512;

//  Zona morta reduzida (25 ≈ 4.8%) combinada com média de 4 leituras ADC
//  para resposta imediata ao tocar no joystick, sem atraso na partida.
const int JOY_DEADZONE = 25;

//  Intervalo fixo de atualização do controle (20 ms = 50 Hz)
const unsigned long INTERVALO_CONTROLE_MS = 20;

// ===========================================================================
//  3. POSIÇÃO DE REPOUSO ("parking")
// ===========================================================================

const float PARK_BASE    = 90.0;
const float PARK_OMBRO   = 90.0;
const float PARK_COTOVEL = 90.0;
const float PARK_GARRA   = 35.0;  // repousa levemente aberta

// ===========================================================================
//  4. OS DOIS ÂNGULOS DE CADA JUNTA
// ===========================================================================

float angBase     = PARK_BASE;
float angOmbro    = PARK_OMBRO;
float angCotovelo = PARK_COTOVEL;
float angGarra    = PARK_GARRA;

float alvoBase     = PARK_BASE;
float alvoOmbro    = PARK_OMBRO;
float alvoCotovelo = PARK_COTOVEL;
float alvoGarra    = PARK_GARRA;

// ===========================================================================
//  5. LIMITES MECÂNICOS
// ===========================================================================

float limBaseMin     = 5.0,   limBaseMax     = 175.0;
float limOmbroMin    = 10.0,  limOmbroMax    = 170.0;
float limCotoveloMin = 10.0,  limCotoveloMax = 170.0;
float limGarraMin    = 0.0,   limGarraMax    = 90.0;   // 0° permite fechamento completo para segurar objetos

// ===========================================================================
//  6. GANHOS — velocidade e precisão fina de cada junta
// ===========================================================================
//  Ganhos calibrados para precisão milimétrica perto do centro e velocidade
//  máxima moderada (~45°/s) no fim de curso, sem movimentos bruscos.
const float GANHO_BASE    = 0.95;
const float GANHO_OMBRO   = 0.90;
const float GANHO_COTOVEL = 0.90;
const float GANHO_GARRA   = 0.80;

// ===========================================================================
//  7. VELOCIDADE MÁXIMA E RAMPA DE SUAVIZAÇÃO
// ===========================================================================

const float VEL_SUAVE = 0.045;         // máx 45°/s: elimina trancos e reduz pico de corrente na fonte
const float FATOR_SUAVIZACAO = 0.18;   // aceleração e frenagem progressivas

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
//  Os valores 544 e 2336 evitam bater no fim de curso interno do SG90
//  (que consome >650 mA e derruba a alimentação USB do Arduino).

const float US_MIN = 544.0;   // microssegundos para 0°
const float US_MAX = 2336.0;  // microssegundos para 180°

//  Velocidade do monitor serial. 9600 é o padrão do Serial Monitor do
//  Arduino IDE (evita caracteres quebrados ao abrir o monitor).
const unsigned long BAUD = 9600;

//  Liga/desliga a leitura contínua dos joysticks pelo comando 'd'.
//  Inicia desligado (false) para a tela ficar parada esperando sua resposta!
bool debugOn = false;

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

//  --- Um passo de suavização com LIMITE DE VELOCIDADE
//
//  Anda na direção do alvo no máximo "passoMax" graus nesta volta do loop
//  (passoMax = velocidade da junta × tempo decorrido em ms). Se o que
//  falta é menor que isso, chega direto ao alvo e descansa — sem a
//  condição de parada a junta "tremularia" os últimos milésimos de grau
//  para sempre, gastando CPU.

float suaviza(float atual, float alvo, float passoMax) {
  float erro = alvo - atual;
  if (erro > -0.05 && erro < 0.05) return alvo;
  float d = erro * FATOR_SUAVIZACAO;
  if (d > passoMax) d = passoMax;
  if (d < -passoMax) d = -passoMax;
  return atual + d;
}

//  --- Converte uma faixa de valores em outra (versão do "map" nativo)
//
//  Usada só para o texto do monitor serial: transforma a leitura crua
//  0..1023 em -100..+100, muito mais fácil de ler que "512".
//  Não afeta o controle dos servos.

int mapADC(int x, int in_min, int in_max, int out_min, int out_max) {
  return (int)((long)(x - in_min) * (out_max - out_min) / (in_max - in_min) + out_min);
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

int centroJoy1X = 512;
int centroJoy1Y = 512;
int centroJoy2X = 512;
int centroJoy2Y = 512;

void calibraJoysticks() {
  long s1x = 0, s1y = 0, s2x = 0, s2y = 0;
  for (uint8_t i = 0; i < 16; i++) {
    s1x += analogRead(PIN_JOY1_X);
    s1y += analogRead(PIN_JOY1_Y);
    s2x += analogRead(PIN_JOY2_X);
    s2y += analogRead(PIN_JOY2_Y);
    delay(5);
  }
  int c1x = (int)(s1x / 16);
  int c1y = (int)(s1y / 16);
  int c2x = (int)(s2x / 16);
  int c2y = (int)(s2y / 16);
  if (c1x > 350 && c1x < 670) centroJoy1X = c1x;
  if (c1y > 350 && c1y < 670) centroJoy1Y = c1y;
  if (c2x > 350 && c2x < 670) centroJoy2X = c2x;
  if (c2y > 350 && c2y < 670) centroJoy2Y = c2y;
}

void sequenceInicial() {
  // Inicializa as variáveis de ângulo em repouso SEM energizar os 4 servos
  // de uma vez na partida (evita que o robô se mexa sozinho ao ligar e evita
  // queda de tensão / reset em loop na porta USB).
  alvoGarra    = angGarra    = PARK_GARRA;
  alvoBase     = angBase     = PARK_BASE;
  alvoOmbro    = angOmbro    = PARK_OMBRO;
  alvoCotovelo = angCotovelo = PARK_COTOVEL;

  desligaBase();
  if (servoOmbro.attached())    servoOmbro.detach();
  if (servoCotovelo.attached()) servoCotovelo.detach();
  if (servoGarra.attached())    servoGarra.detach();

  // --- 3 piscadas de confirmação
  for (int i = 0; i < 3; i++) {
    digitalWrite(PIN_LED, HIGH);
    delay(100);
    digitalWrite(PIN_LED, LOW);
    delay(100);
  }
}

// ===========================================================================
//  11. RESET E STATUS — comandos simples pelo Monitor Serial
// ===========================================================================
//  Aperte 'r' para levar o braço suavemente de volta ao repouso (sem tranco).

void resetPark() {
  alvoBase = PARK_BASE;
  alvoOmbro = PARK_OMBRO;
  alvoCotovelo = PARK_COTOVEL;
  alvoGarra = PARK_GARRA;
  Serial.println(F("voltando suavemente para posicao de repouso"));
}

void mostraAjuda() {
  Serial.println(F("\n========================================================"));
  Serial.println(F("   BRACO ROBOTICO 4-DOF — CONTROLE POR JOYSTICK (9600)"));
  Serial.println(F("========================================================"));
  Serial.println(F("  s = Mostrar angulos atuais das 4 juntas"));
  Serial.println(F("  r = Voltar suavemente para posicao de repouso (90 graus)"));
  Serial.println(F("  d = Ligar/desligar leitura continua dos joysticks"));
  Serial.println(F("  h = Mostrar este menu"));
  Serial.println(F("========================================================"));
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
//  12. LEITURA DO JOYSTICK → ÂNGULO ALVO
// ===========================================================================

//  Média de 4 amostras consecutivas para eliminar ruído analógico sem atraso
int leEixoSuave(uint8_t pino) {
  long soma = 0;
  for (uint8_t i = 0; i < 4; i++) {
    soma += analogRead(pino);
  }
  return (int)(soma >> 2);
}

float aplicaEixo(int leitura, int centro, float atual, float min, float max, float ganho) {
  int e = leitura - centro;
  const int dz = JOY_DEADZONE;

  if (abs(e) > dz) {
    int excesso = e - (e > 0 ? dz : -dz);
    float alcance = (float)(512 - dz);
    float norm = constrainAngle((float)excesso / alcance, -1.0, 1.0);
    float absNorm = norm < 0.0 ? -norm : norm;
    // Curva quadrática suave: muito precisa perto do centro, sem arranque brusco
    float curva = 0.18 * norm + 0.82 * norm * absNorm;
    float delta = curva * ganho;
    return constrainAngle(atual + delta, min, max);
  }

  return atual;   // joystick no centro: segura a posição onde parou
}

bool ativoBase     = false;
bool ativoOmbro    = false;
bool ativoCotovelo = false;
bool ativoGarra    = false;

// ===========================================================================
//  13. LOOP — roda para sempre, milhares de vezes por segundo
// ===========================================================================

void loop() {

  //  --- ETAPA A: Ler comandos do serial -------------------------------
  while (Serial.available() > 0) {
    char c = Serial.read();
    switch (c) {
      case 'h': case 'H': mostraAjuda(); break;
      case 's': case 'S': status();      break;
      case 'r': case 'R':
        resetPark();
        ativoBase = ativoOmbro = ativoCotovelo = ativoGarra = true;
        break;
      case 'd': case 'D':
        debugOn = !debugOn;   // inverte o valor (liga<->desliga)
        Serial.print(F("debug "));
        Serial.println(debugOn ? F("ON") : F("OFF"));
        break;
    }
  }

  //  --- ETAPA B: Atualização em taxa fixa de 50 Hz (a cada 20 ms) -----
  unsigned long agora = millis();
  static unsigned long ultimoCicloMs = 0;
  if (agora - ultimoCicloMs < INTERVALO_CONTROLE_MS) {
    return;
  }
  ultimoCicloMs = agora;

  int joy1X = leEixoSuave(PIN_JOY1_X);
  int joy1Y = leEixoSuave(PIN_JOY1_Y);
  int joy2X = leEixoSuave(PIN_JOY2_X);
  int joy2Y = leEixoSuave(PIN_JOY2_Y);

  //  --- ETAPA C: joystick altera os ALVOS proporcionalmente -----------
  //    joy1X (A0) → Base       (esquerdo, esquerda/direita) gira a base
  //    joy1Y (A1) → Cotovelo   (esquerdo, cima/baixo)       movimenta o cotovelo
  //    joy2Y (A2) → Ombro      (direito,  cima/baixo)       sobe/desce o ombro
  //    joy2X (A3) → Garra      (direito,  esquerda/direita) esquerda=fecha, direita=abre

  alvoBase     = aplicaEixo(joy1X, centroJoy1X, alvoBase, limBaseMin, limBaseMax, GANHO_BASE);
  alvoCotovelo = aplicaEixo(joy1Y, centroJoy1Y, alvoCotovelo, limCotoveloMin, limCotoveloMax, GANHO_COTOVEL);
  alvoOmbro    = aplicaEixo(joy2Y, centroJoy2Y, alvoOmbro, limOmbroMin, limOmbroMax, GANHO_OMBRO);
  alvoGarra    = aplicaEixo(joy2X, centroJoy2X, alvoGarra, limGarraMin, limGarraMax, GANHO_GARRA);

  //  --- ETAPA D: SUAVIZAÇÃO + RETENÇÃO CONTÍNUA DE TORQUE -------------
  //  Assim que cada junta recebe o primeiro movimento, seu servo permanece
  //  conectado (attached) enviando pulso PWM contínuo em microssegundos.
  //  Isso garante que:
  //    1. A garra mantenha a força segurando o objeto ao soltar o joystick;
  //    2. O ombro e o cotovelo não cedam com o peso nem deem tranco ao religar.

  float passoMax = VEL_SUAVE * (float)INTERVALO_CONTROLE_MS;

  if (abs(alvoBase - angBase) > 0.03) {
    angBase = suaviza(angBase, alvoBase, passoMax);
    ativoBase = true;
  }
  if (ativoBase) {
    escreveBase(anguloParaUs(angBase));
  }

  if (abs(alvoOmbro - angOmbro) > 0.03) {
    angOmbro = suaviza(angOmbro, alvoOmbro, passoMax);
    ativoOmbro = true;
  }
  if (ativoOmbro) {
    if (!servoOmbro.attached()) servoOmbro.attach(PIN_SERVO_OMBRO);
    servoOmbro.writeMicroseconds(anguloParaUs(angOmbro));
  }

  if (abs(alvoCotovelo - angCotovelo) > 0.03) {
    angCotovelo = suaviza(angCotovelo, alvoCotovelo, passoMax);
    ativoCotovelo = true;
  }
  if (ativoCotovelo) {
    if (!servoCotovelo.attached()) servoCotovelo.attach(PIN_SERVO_COTOVEL);
    servoCotovelo.writeMicroseconds(anguloParaUs(angCotovelo));
  }

  if (abs(alvoGarra - angGarra) > 0.03) {
    angGarra = suaviza(angGarra, alvoGarra, passoMax * 0.85);
    ativoGarra = true;
  }
  if (ativoGarra) {
    if (!servoGarra.attached()) servoGarra.attach(PIN_SERVO_GARRA);
    servoGarra.writeMicroseconds(anguloParaUs(angGarra));
  }

  //  --- ETAPA F: leitura dos joysticks no monitor serial --------------

  static unsigned long ultimoPrint = 0;
  if (debugOn && agora - ultimoPrint >= 500) {
    ultimoPrint = agora;
    Serial.print(F("[JOYSTICKS] B=")); Serial.print((int)angBase);
    Serial.print(F(" O="));            Serial.print((int)angOmbro);
    Serial.print(F(" C="));            Serial.print((int)angCotovelo);
    Serial.print(F(" G="));            Serial.print((int)angGarra);
    Serial.print(F(" | joy1="));       Serial.print(mapADC(joy1X, 0, 1023, -100, 100));
    Serial.print(F("/"));              Serial.print(mapADC(joy1Y, 0, 1023, -100, 100));
    Serial.print(F(" joy2="));         Serial.print(mapADC(joy2X, 0, 1023, -100, 100));
    Serial.print(F("/"));              Serial.println(mapADC(joy2Y, 0, 1023, -100, 100));
  }
}

// ===========================================================================
//  14. SETUP — roda UMA VEZ ao ligar/resetar
// ===========================================================================

void setup() {
  Serial.begin(BAUD);   // inicia o monitor serial a 9600 baud
  pinMode(PIN_LED, OUTPUT);

  calibraJoysticks();
  sequenceInicial();
  mostraAjuda();
  Serial.println(F("braco_4dof pronto — parado aguardando movimento nos joysticks!"));
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
//  4. VEL_SUAVE — menor = mais suave e lento (graus por ms). Maior = mais
//     rápido e mais tranco.
//
//  5. JOY_DEADZONE — se o braço tremer parado, aumente. Se estiver "preso"
//     e não responder a comandos pequenos, diminua.
//
//  LEMBRE-SE: com 4 SG90, alimente os servos por fonte 5 V / 3 A separada
//  com GND comum ao Arduino. O regulador da placa não dá conta.
