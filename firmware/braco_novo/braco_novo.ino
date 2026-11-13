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
 *    Servo Base (Yaw)    D11    0..180    neutro 90
 *    Servo Ombro         D10   30..150    neutro 90
 *    Servo Cotovelo      D9    20..160    neutro 90
 *    Servo Garra         D5      0..90    neutro  0 (fechada)
 *
 *    Joystick esquerdo X A0   → Base (esquerda/direita)
 *    Joystick esquerdo Y A1   → Cotovelo (cima/baixo)
 *    Joystick direito  X A3   → Garra (esquerda fecha / direita abre)
 *    Joystick direito  Y A2   → Ombro (cima/baixo)
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
 *    b → varre os pinos candidatos e mostra qual deles move a base
 *
 *  PROTOCOLO DO APP (prefixo #, linha inteira terminada em newline)
 *    #P<b>;<o>;<c>;<g>  alvos em graus (20 Hz; reenviar é o heartbeat)
 *    #V<lx>;<ly>;<rx>;<ry> sticks virtuais do app (-100..100)
 *    #L0 / #L1          libera / trava a pressão da garra
 *    #M assume | #J devolve ao joystick | #H segura | #S status | #B diagnóstico
 *
 * ============================================================================
 */

#define FW_VER "1.3.3"

#include <Servo.h>
#include <EEPROM.h>

// ===========================================================================
//  1. ATUADORES — os quatro servomotores
// ===========================================================================
//  uint8_t = inteiro de 8 bits (0..255). É o tipo certo para número de
//  pino: ocupa 1 byte em vez de 2, economizando memória no Arduino.

const uint8_t PIN_SERVO_BASE    = 11;
const uint8_t PIN_SERVO_OMBRO   = 10;
const uint8_t PIN_SERVO_COTOVEL = 9;
//  Pinos extras da fileira da Shield espelhados para o Cotovelo (D9/D3/D7/D8/D12).
//  O D5 saiu do espelho: agora e o canal da Garra.
const uint8_t PIN_COTOV_ALT_D3  = 3;
const uint8_t PIN_COTOV_ALT_D7  = 7;
const uint8_t PIN_COTOV_ALT_D8  = 8;
const uint8_t PIN_COTOV_ALT_D12 = 12;
const uint8_t PIN_SERVO_GARRA   = 5;
const uint8_t PIN_LED           = 13;

Servo servoBase;
Servo servoOmbro;
Servo servoCotovelo;
Servo servoCotovD3;
Servo servoCotovD7;
Servo servoCotovD8;
Servo servoCotovD12;
Servo servoGarra;

void escreveBase(int pulsoUs) {
  if (!servoBase.attached()) {
    servoBase.attach(PIN_SERVO_BASE);
  }
  servoBase.writeMicroseconds(pulsoUs);
}

void desligaBase() {
  if (servoBase.attached()) {
    servoBase.detach();
  }
}

void escreveCotovelo(int pulsoUs) {
  if (!servoCotovelo.attached()) {
    servoCotovelo.attach(PIN_SERVO_COTOVEL);
    servoCotovD3.attach(PIN_COTOV_ALT_D3);
    servoCotovD7.attach(PIN_COTOV_ALT_D7);
    servoCotovD8.attach(PIN_COTOV_ALT_D8);
    servoCotovD12.attach(PIN_COTOV_ALT_D12);
  }
  servoCotovelo.writeMicroseconds(pulsoUs);
  servoCotovD3.writeMicroseconds(pulsoUs);
  servoCotovD7.writeMicroseconds(pulsoUs);
  servoCotovD8.writeMicroseconds(pulsoUs);
  servoCotovD12.writeMicroseconds(pulsoUs);
}

void desligaCotovelo() {
  if (servoCotovelo.attached()) {
    servoCotovelo.detach();
    servoCotovD3.detach();
    servoCotovD7.detach();
    servoCotovD8.detach();
    servoCotovD12.detach();
  }
}

// ===========================================================================
//  2. JOYSTICKS — leitura analógica (Shield Casa da Robótica Ref: PD07)
// ===========================================================================

const uint8_t PIN_JOY1_X = A0;   // joystick esquerdo: esquerda/direita → Base
const uint8_t PIN_JOY1_Y = A1;   // joystick esquerdo: cima/baixo       → Cotovelo
const uint8_t PIN_JOY2_X = A3;   // joystick direito:  esquerda/direita → Garra (esq=fecha, dir=abre)
const uint8_t PIN_JOY2_Y = A2;   // joystick direito:  cima/baixo       → Ombro

//  Botões dos joysticks na Shield Casa da Robótica (Ref: PD07)
//  - Botão 1 (PIN_BTN_JOY1 = D2, Joystick Esquerdo):
//      * Clique rápido  -> vai suavemente para a Posição 0 salva
//      * Segurar por 2s -> salva a posição atual como nova Posição 0 na EEPROM
//  - Botão 2 (PIN_BTN_JOY2 = D4, Joystick Direito — lado da Garra):
//      * 1º clique -> TRAVA A GARRA COM PRESSÃO (mantém torque contínuo no objeto e bloqueia abertura)
//      * 2º clique -> LIBERA A GARRA (permite abrir novamente pelo joystick)
const uint8_t PIN_BTN_JOY1 = 2;  // botão do joystick esquerdo (INPUT_PULLUP)
const uint8_t PIN_BTN_JOY2 = 4;  // botão do joystick direito  (INPUT_PULLUP)

const int JOY_CENTER = 512;

//  Zona morta calibrada (28 ≈ 5.5%) para resposta rápida sem captar inclinação involuntária.
const int JOY_DEADZONE = 28;

//  Histerese: um eixo já engatado só solta abaixo deste valor. Evita o
//  liga/desliga no limite da zona morta, que faz o giro sair aos trancos.
const int JOY_DZ_SOLTA = 14;

//  Intervalo fixo de atualização do controle (20 ms = 50 Hz)
const unsigned long INTERVALO_CONTROLE_MS = 20;

// ===========================================================================
//  3. POSIÇÃO 0 / REPOUSO ("parking" — ajustável e salva na EEPROM)
// ===========================================================================
//  Dica: posicione o braço onde quiser com os joysticks e SEGURE o botão do
//  joystick por 2 segundos para gravar essa posição como a nova Posição 0!
//  1 clique rápido no botão leva o braço para a Posição 0 salva.

float posZeroBase     = 90.0;
float posZeroOmbro    = 60.0;
float posZeroCotovelo = 45.0;
float posZeroGarra    = 35.0;

const uint16_t EEPROM_ZERO_MAGIC = 0x5A30;
const uint16_t EEPROM_LIM_MAGIC  = 0x4C4D;

// ===========================================================================
//  4. OS DOIS ÂNGULOS DE CADA JUNTA (protegidos em .noinit contra brownout)
// ===========================================================================
//  Com fonte 12V no Jack do Arduino, picos de corrente podem causar micro-reset
//  (brownout). Guardando os ângulos em .noinit, o Arduino nunca perde a posição
//  em que o braço estava no alto e não despenca para 90°.

const uint16_t NOINIT_MAGIC = 0xB4B5;
const uint16_t BOOT_MAGIC   = 0xB007;
uint16_t __attribute__((section(".noinit"))) bootMagic;
uint16_t __attribute__((section(".noinit"))) bootCount;
uint16_t __attribute__((section(".noinit"))) savedMagic;
float __attribute__((section(".noinit"))) angBase;
float __attribute__((section(".noinit"))) angOmbro;
float __attribute__((section(".noinit"))) angCotovelo;
float __attribute__((section(".noinit"))) angGarra;
uint8_t __attribute__((section(".noinit"))) garraTravada;

float alvoBase     = 90.0;
float alvoOmbro    = 60.0;
float alvoCotovelo = 45.0;
float alvoGarra    = 35.0;

// ===========================================================================
//  5. LIMITES MECÂNICOS (proteção contra batida em parafuso e fim de curso)
// ===========================================================================
//  Na Base, 12° a 168° evita que a mesa giratória bata na cabeça do parafuso
//  ou force o fim de curso interno do servo SG90. Você também pode gravar o
//  limite exato antes do parafuso segurando o Botão Direito (D4) por 2s!

float limBaseMin     = 12.0,  limBaseMax     = 168.0;
float limOmbroMin    = 0.0,   limOmbroMax    = 180.0;
float limCotoveloMin = 0.0,   limCotoveloMax = 180.0;
float limGarraMin    = 0.0,   limGarraMax    = 90.0;   // 0° permite fechamento 100% completo da garra

// ===========================================================================
//  6. GANHOS — resposta sem delay, porém lenta e precisa (especialmente Cotovelo)
// ===========================================================================
//  Em ciclos de 20 ms (50 Hz):
//    GANHO 0.50 = máx 25°/s (Cotovelo: movimento lento e milimétrico)
//    GANHO 0.65 = máx 32°/s (Ombro: subida e descida controlada)
//    GANHO 0.75 = máx 37°/s (Base)
//    GANHO 1.35 = máx 67°/s (Garra: abre e fecha com firmeza e agilidade)
const float GANHO_BASE    = 0.75;
const float GANHO_OMBRO   = 0.65;
const float GANHO_COTOVEL = 0.50;
const float GANHO_GARRA   = 1.35;

// ===========================================================================
//  7. VELOCIDADE MÁXIMA E RAMPA DE SUAVIZAÇÃO (zero delay + velocidade contida)
// ===========================================================================

const float VEL_SUAVE = 0.038;         // teto de 38°/s nas juntas do braço
const float FATOR_SUAVIZACAO = 0.50;   // acompanha o alvo na hora (sem delay ao mexer ou parar)

// ===========================================================================
//  8. PULSO DO SERVO — o que realmente comanda o motor
// ===========================================================================
//  Um servo NÃO recebe "90 graus". Ele recebe um PULSO de largura variável,
//  repetido a cada ~20 ms. A posição do eixo depende da duração desse pulso:
//
//      pulso curto (~544 µs) → eixo todo para um lado   (0°)
//      pulso médio (~1500 µs) → eixo no meio            (90°)
//      pulso longo (~2400 µs) → eixo todo para o outro (180°)
//
//  Por que usar microssegundos em vez de servo.write(graus)?
//  servo.write() só aceita INTEIROS de 0 a 180, então guarda a posição com
//  resolução de 1 grau — o servo fica parado dentro dessa faixa, sem
// Interpolação. writeMicroseconds() dá resolução de ~0.3°, o servo pode
//  assumir qualquer posição intermediária. Para um braço com 4 juntas
//  combinando, essa diferença é visível: os cantos ficam mais contínuos.
//
//  Os valores 544 e 2400 respeitam a faixa padrão do SG90 no Arduino nas juntas
//  (evitando forçar o fim de curso interno da Base), enquanto a Garra usa 500 µs
//  para garantir fechamento total com pressão firme.

const float US_MIN       = 544.0;   // pulso mínimo padrão SG90 para Base/Ombro/Cotovelo (não força o batente)
const float US_MIN_GARRA = 500.0;   // pulso mínimo estendido exclusivo da Garra (fechamento 100%)
const float US_MAX       = 2400.0;  // pulso máximo padrão SG90 (180°)

//  Velocidade do monitor serial. 115200 é o padrão do app de controle por USB;
//  a 9600 uma linha de status levaria ~40 ms e travaria a malha de 50 Hz.
const unsigned long BAUD = 115200;

//  Liga/desliga a leitura contínua dos joysticks pelo comando 'd'.
//  Inicia desligado (false) para a tela ficar parada esperando sua resposta!
bool debugOn = false;

//  Quem está no controle do braço: false = joystick, true = app USB.
//  Declarado aqui em cima porque resetPark() também consulta.
bool modoPC = false;

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

int anguloParaUsGarra(float ang) {
  float us = US_MIN_GARRA + (ang / 180.0) * (US_MAX - US_MIN_GARRA);
  if (us < US_MIN_GARRA) us = US_MIN_GARRA;
  if (us > US_MAX)       us = US_MAX;
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
  if (erro > -0.005 && erro < 0.005) return alvo;
  float d = erro * FATOR_SUAVIZACAO;
  if (d > passoMax) d = passoMax;
  if (d < -passoMax) d = -passoMax;
  return atual + d;
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

int __attribute__((section(".noinit"))) centroJoy1X;
int __attribute__((section(".noinit"))) centroJoy1Y;
int __attribute__((section(".noinit"))) centroJoy2X;
int __attribute__((section(".noinit"))) centroJoy2Y;

unsigned long ativoBaseMs     = 0;
unsigned long ativoOmbroMs    = 0;
unsigned long ativoCotoveloMs = 0;
unsigned long ativoGarraMs    = 0;
const unsigned long TEMPO_HOLD_MS = 240;
//  A base sustenta o braco inteiro: solta o motor mais devagar para nao despencar
//  quando voce para de empurrar por um instante no meio do giro.
const unsigned long TEMPO_HOLD_BASE_MS = 600;

void calibraJoysticks() {
  // Se foi um micro-reset (brownout) com os centros já válidos em .noinit,
  // preserva a calibração para não gravar a posição inclinada da mão do usuário
  if (savedMagic == NOINIT_MAGIC &&
      centroJoy1X > 400 && centroJoy1X < 620 &&
      centroJoy1Y > 400 && centroJoy1Y < 620 &&
      centroJoy2X > 400 && centroJoy2X < 620 &&
      centroJoy2Y > 400 && centroJoy2Y < 620) {
    return;
  }
  long s1x = 0, s1y = 0, s2x = 0, s2y = 0;
  for (uint8_t i = 0; i < 8; i++) {
    analogRead(PIN_JOY1_X); s1x += analogRead(PIN_JOY1_X);
    analogRead(PIN_JOY1_Y); s1y += analogRead(PIN_JOY1_Y);
    analogRead(PIN_JOY2_X); s2x += analogRead(PIN_JOY2_X);
    analogRead(PIN_JOY2_Y); s2y += analogRead(PIN_JOY2_Y);
    delay(2);
  }
  int c1x = (int)(s1x / 8);
  int c1y = (int)(s1y / 8);
  int c2x = (int)(s2x / 8);
  int c2y = (int)(s2y / 8);
  if (c1x > 400 && c1x < 620) centroJoy1X = c1x; else centroJoy1X = 512;
  if (c1y > 400 && c1y < 620) centroJoy1Y = c1y; else centroJoy1Y = 512;
  if (c2x > 400 && c2x < 620) centroJoy2X = c2x; else centroJoy2X = 512;
  if (c2y > 400 && c2y < 620) centroJoy2Y = c2y; else centroJoy2Y = 512;
}

void carregaPosicaoZeroEEPROM() {
  uint16_t limMagic = 0;
  EEPROM.get(18, limMagic);
  if (limMagic == EEPROM_LIM_MAGIC) {
    float bMin, bMax;
    EEPROM.get(20, bMin);
    EEPROM.get(24, bMax);
    if (bMin >= 0.0 && bMax <= 180.0 && (bMax - bMin) >= 20.0) {
      limBaseMin = bMin;
      limBaseMax = bMax;
    }
  }

  uint16_t magic = 0;
  EEPROM.get(0, magic);
  if (magic == EEPROM_ZERO_MAGIC) {
    float b, o, c, g;
    EEPROM.get(2,  b);
    EEPROM.get(6,  o);
    EEPROM.get(10, c);
    EEPROM.get(14, g);
    if (b >= 0.0 && b <= 180.0 && o >= limOmbroMin && o <= limOmbroMax &&
        c >= limCotoveloMin && c <= limCotoveloMax && g >= limGarraMin && g <= limGarraMax) {
      posZeroBase     = constrainAngle(b, limBaseMin, limBaseMax);
      posZeroOmbro    = o;
      posZeroCotovelo = c;
      posZeroGarra    = g;
    }
  }
}

void piscaConfirmacaoLED() {
  for (uint8_t i = 0; i < 4; i++) {
    digitalWrite(PIN_LED, HIGH); delay(60);
    digitalWrite(PIN_LED, LOW);  delay(60);
  }
  digitalWrite(PIN_LED, garraTravada ? HIGH : LOW);
}

void salvaLimitesBaseEEPROM() {
  EEPROM.put(18, EEPROM_LIM_MAGIC);
  EEPROM.put(20, limBaseMin);
  EEPROM.put(24, limBaseMax);
  Serial.print(F("LIMITES DA BASE SALVOS: Min=")); Serial.print((int)limBaseMin);
  Serial.print(F(" Max=")); Serial.println((int)limBaseMax);
  piscaConfirmacaoLED();
}

void salvaPosicaoZeroEEPROM() {
  posZeroBase     = angBase;
  posZeroOmbro    = angOmbro;
  posZeroCotovelo = angCotovelo;
  posZeroGarra    = angGarra;
  EEPROM.put(0,  EEPROM_ZERO_MAGIC);
  EEPROM.put(2,  posZeroBase);
  EEPROM.put(6,  posZeroOmbro);
  EEPROM.put(10, posZeroCotovelo);
  EEPROM.put(14, posZeroGarra);
  Serial.print(F("NOVA POSICAO 0 SALVA: B=")); Serial.print((int)posZeroBase);
  Serial.print(F(" O=")); Serial.print((int)posZeroOmbro);
  Serial.print(F(" C=")); Serial.print((int)posZeroCotovelo);
  Serial.print(F(" G=")); Serial.println((int)posZeroGarra);
  piscaConfirmacaoLED();
}

void sequenceInicial() {
  desligaBase();

  carregaPosicaoZeroEEPROM();

  // Se foi um micro-reset por queda de tensão (brownout da fonte 12V),
  // restaura imediatamente os ângulos e mantém o torque ativo para o braço não cair!
  if (savedMagic == NOINIT_MAGIC &&
      angBase >= 0.0 && angBase <= 180.0 &&
      angOmbro >= limOmbroMin && angOmbro <= limOmbroMax &&
      angCotovelo >= limCotoveloMin && angCotovelo <= limCotoveloMax &&
      angGarra >= limGarraMin && angGarra <= limGarraMax) {
    angBase      = constrainAngle(angBase, limBaseMin, limBaseMax);
    alvoBase     = angBase;
    alvoOmbro    = angOmbro;
    alvoCotovelo = angCotovelo;
    alvoGarra    = angGarra;
    if (garraTravada > 1) garraTravada = 0;
    unsigned long agora = millis();
    if (agora == 0) agora = 1;
    ativoOmbroMs    = agora;
    ativoCotoveloMs = agora;
    ativoGarraMs    = agora;
    servoOmbro.attach(PIN_SERVO_OMBRO);
    servoOmbro.writeMicroseconds(anguloParaUs(angOmbro));
    escreveCotovelo(anguloParaUs(angCotovelo));
    if (garraTravada) {
      servoGarra.attach(PIN_SERVO_GARRA);
      servoGarra.writeMicroseconds(anguloParaUsGarra(angGarra));
    }
    digitalWrite(PIN_LED, garraTravada ? HIGH : LOW);
    return;
  }

  if (servoOmbro.attached()) servoOmbro.detach();
  desligaCotovelo();
  if (servoGarra.attached()) servoGarra.detach();

  // Primeira ligação a frio: inicia na Posição 0
  alvoGarra    = angGarra    = posZeroGarra;
  alvoBase     = angBase     = constrainAngle(posZeroBase, limBaseMin, limBaseMax);
  alvoOmbro    = angOmbro    = posZeroOmbro;
  alvoCotovelo = angCotovelo = posZeroCotovelo;
  garraTravada = 0;
  savedMagic   = NOINIT_MAGIC;

  // Pulso curto de destravamento na garra ao iniciar (um servo por vez)
  servoGarra.attach(PIN_SERVO_GARRA);
  servoGarra.writeMicroseconds(anguloParaUsGarra(angGarra));
  delay(220);
  servoGarra.detach();

  // Posiciona a Base no centro inicial (um servo por vez para não exigir corrente alta)
  escreveBase(anguloParaUs(angBase));
  delay(300);
  desligaBase();

  for (int i = 0; i < 3; i++) {
    digitalWrite(PIN_LED, HIGH);
    delay(80);
    digitalWrite(PIN_LED, LOW);
    delay(80);
  }
}

// ===========================================================================
//  11. RESET E STATUS — comandos simples pelo Monitor Serial e Botões
// ===========================================================================

bool voltandoZero = false;
bool btn1Pressionado = false;
bool btn1SalvouNoHold = false;
unsigned long inicioPressaoBtn1Ms = 0;

bool btn2Pressionado = false;
bool btn2SalvouNoHold = false;
unsigned long inicioPressaoBtn2Ms = 0;

void resetPark() {
  if (modoPC) {
    Serial.println(F("PC no controle: irei a Posicao 0 so depois de soltar no app"));
    return;
  }
  voltandoZero = true;
  Serial.println(F("botao/comando: indo suavemente para a Posicao 0"));
}

void mostraAjuda() {
  Serial.println(F("\n========================================================"));
  Serial.println(F("   BRACO ROBOTICO 4-DOF — CONTROLE POR JOYSTICK (9600)"));
  Serial.println(F("========================================================"));
  Serial.println(F("  Botao Joy Esq (D2): clique = Ir Pos 0 | segurar 2s = Salvar Pos 0"));
  Serial.println(F("  Botao Joy Dir (D4): clique = Travar/Liberar pressao da Garra"));
  Serial.println(F("                      segurar 2s = Salvar limite lateral da Base (anti-parafuso)"));
  Serial.println(F("  [ / ] = Salvar limite Min / Max da Base no angulo atual"));
  Serial.println(F("  l = Restaurar limites amplos da Base (5 a 175 graus)"));
  Serial.println(F("  s = Mostrar angulos e limites atuais"));
  Serial.println(F("  r = Voltar suavemente para posicao de repouso (Pos 0)"));
  Serial.println(F("  d = Ligar/desligar leitura continua dos joysticks (valores crus A0/A1/A3/A2)"));
  Serial.println(F("  b = Teste: a base pisca N vezes; conte para saber o pino"));
  Serial.println(F("  h = Mostrar este menu"));
  Serial.println(F("  app USB: #M assume | #P<b>;<o>;<c>;<g> alvos | #J solta | #H segura | #S status"));
  Serial.println(F("========================================================"));
}

//  Mostra o ângulo ATUAL das quatro juntas (não o alvo — o que o servo
//  está fazendo de verdade agora).
void status() {
  Serial.print(F("B=")); Serial.print((int)angBase);
  Serial.print(F(" (Lim:")); Serial.print((int)limBaseMin);
  Serial.print(F("..")); Serial.print((int)limBaseMax);
  Serial.print(F(") O=")); Serial.print((int)angOmbro);
  Serial.print(F(" C=")); Serial.print((int)angCotovelo);
  Serial.print(F(" G=")); Serial.print((int)angGarra);
  Serial.print(F(" TravaGarra=")); Serial.println(garraTravada ? F("SIM") : F("NAO"));
  //  Centros calibrados: se um deles estiver longe de ~512, a zona morta fica
  //  assimétrica (responde mais fácil para um lado que para o outro).
  Serial.print(F("centros: A0=")); Serial.print(centroJoy1X);
  Serial.print(F(" A1=")); Serial.print(centroJoy1Y);
  Serial.print(F(" A3=")); Serial.print(centroJoy2X);
  Serial.print(F(" A2=")); Serial.println(centroJoy2Y);
}

// ===========================================================================
//  12. LEITURA DO JOYSTICK → ÂNGULO ALVO
// ===========================================================================

//  Descarta a 1ª leitura após troca de canal do mux ADC do ATmega328P
//  (evita acoplamento capacitivo entre A0↔A1 e A3↔A2) e tira média de 4 amostras
int leEixoSuave(uint8_t pino) {
  analogRead(pino);
  long soma = 0;
  for (uint8_t i = 0; i < 4; i++) {
    soma += analogRead(pino);
  }
  return (int)(soma >> 2);
}

//  Trava exclusiva de eixo (modo cruz '+' estrito):
//    0 = parado no centro
//    1 = travado no eixo HORIZONTAL (Esquerda/Direita) -> bloqueia 100% o eixo Vertical (Cima/Baixo)
//    2 = travado no eixo VERTICAL   (Cima/Baixo)       -> bloqueia 100% o eixo Horizontal (Esquerda/Direita)
//  Enquanto você estiver movendo para a esquerda ou direita, mesmo que o polegar
//  incline na diagonal, o movimento vertical fica 100% travado (só libera Cima/Baixo
//  quando você soltar o movimento lateral e empurrar claramente para cima ou para baixo).
uint8_t eixoTravaJoy1 = 0;
uint8_t eixoTravaJoy2 = 0;
unsigned long ultimoBaseJoy1Ms = 0;

//  No Joystick Esquerdo (Base + Cotovelo):
//  - Prioridade total para Esquerda/Direita (Base): qualquer movimento lateral (dx >= 65)
//    bloqueia 100% o Cotovelo e NUNCA fica preso no modo vertical.
//  - O Cotovelo (Cima/Baixo) só se move quando a alavanca está alinhada no centro
//    horizontal (dx < 65) e é empurrada claramente para cima ou para baixo (dy > 130).
const int JOY1_DZ_Y_COTOVEL = 80;
const int JOY_DZ_Y_SOLTA = 45;

void isolaEixoJoy1Esquerdo(int &leituraX, int centroX, int &leituraY, int centroY, unsigned long agora) {
  int dx = abs(leituraX - centroX);
  int dy = abs(leituraY - centroY);

  // Histerese: engata em 28 mas só solta abaixo de 14. Sem isso, com a alavanca
  // perto do limite da zona morta o eixo liga/desliga a cada ciclo e o giro sai
  // aos trancos.
  int dzX = (eixoTravaJoy1 == 1) ? JOY_DZ_SOLTA : JOY_DEADZONE;
  int dzY = (eixoTravaJoy1 == 2) ? JOY_DZ_Y_SOLTA : JOY1_DZ_Y_COTOVEL;

  // Qualquer desvio lateral acima da zona morta engata a Base e zera o Cotovelo.
  // (antes exigia dx >= 70, e o comando entre 29 e 69 era descartado sem mover nada)
  if (dx > dzX) {
    eixoTravaJoy1 = 1;
    ultimoBaseJoy1Ms = agora;
    leituraY = centroY;
    return;
  }

  // Só libera o Cotovelo (Cima/Baixo) se o joystick NÃO estiver inclinado para os lados,
  // for empurrado claramente para cima/baixo e não estiver apenas cruzando o centro
  // durante uma varredura esquerda<->direita.
  bool cruzandoCentroLateral = (ultimoBaseJoy1Ms != 0 && (agora - ultimoBaseJoy1Ms < 250));
  int limiarY = (cruzandoCentroLateral && eixoTravaJoy1 != 2) ? 230 : dzY;

  if (dy > limiarY) {
    eixoTravaJoy1 = 2;
    leituraX = centroX;
    return;
  }

  eixoTravaJoy1 = 0;
  leituraX = centroX;
  leituraY = centroY;
}

void isolaEixoExclusivo(int &leituraX, int centroX, int &leituraY, int centroY, uint8_t &eixoTrava) {
  int dx = abs(leituraX - centroX);
  int dy = abs(leituraY - centroY);
  const int dzX = JOY_DEADZONE;
  const int dzY = 48; // zona morta vertical mais firme contra inclinação involuntária

  if (eixoTrava == 0) {
    if (dx > dzX && (long)dx * 10 >= (long)dy * 8) {
      eixoTrava = 1; // engatou Esquerda/Direita
    } else if (dy > dzY && (long)dy * 10 > (long)dx * 14) {
      eixoTrava = 2; // engatou Cima/Baixo (exige movimento claramente vertical)
    } else {
      leituraX = centroX;
      leituraY = centroY;
      return;
    }
  }

  if (eixoTrava == 1) {
    if (dx <= dzX) {
      if (dy > 90) {
        eixoTrava = 2;
        leituraX = centroX;
      } else {
        eixoTrava = 0;
        leituraX = centroX;
        leituraY = centroY;
      }
    } else {
      // Enquanto estiver movendo para Esquerda/Direita, trava Cima/Baixo em 0!
      leituraY = centroY;
    }
  } else if (eixoTrava == 2) {
    if (dy <= dzX) {
      if (dx > 90) {
        eixoTrava = 1;
        leituraY = centroY;
      } else {
        eixoTrava = 0;
        leituraX = centroX;
        leituraY = centroY;
      }
    } else {
      // Enquanto estiver movendo para Cima/Baixo, trava Esquerda/Direita em 0!
      leituraX = centroX;
    }
  }
}

float aplicaEixoZona(int leitura, int centro, float atual, float min, float max, float ganho, int dz) {
  int e = leitura - centro;

  if (abs(e) > dz) {
    int excesso = e - (e > 0 ? dz : -dz);
    float alcance = (float)(512 - dz);
    float norm = constrainAngle((float)excesso / alcance, -1.0, 1.0);
    float absNorm = norm < 0.0 ? -norm : norm;
    // Curva progressiva (70% linear + 30% quadrática): responde já na metade
    // inicial do curso, sem o "nada... nada... dispara" da versão anterior.
    float curva = 0.7 * norm + 0.3 * norm * absNorm;
    float delta = curva * ganho;
    return constrainAngle(atual + delta, min, max);
  }

  return atual;   // joystick no centro: segura a posição onde parou
}

float aplicaEixo(int leitura, int centro, float atual, float min, float max, float ganho) {
  return aplicaEixoZona(leitura, centro, atual, min, max, ganho, JOY_DEADZONE);
}

//  --- Teste de pino do servo da Base (comando 'b') --------------------------
//
//  Move UM pino candidato por vez, devagar, e escreve no monitor qual está
//  sendo testado. Você olha qual deles gira o CORPO do robô e nos diz.
//  Serve para descobrir em que conector da shield o servo da base está ligado
//  sem ficar trocando o código no chute. Não grava nada na EEPROM.
void descobrePinoBase() {
  const uint8_t candidatos[] = { 11, 10, 6, 9, 5, 7 };
  const uint8_t total = sizeof(candidatos);

  Serial.println(F("\n>>> TESTE DA BASE — o braco se mexe sozinho por ~16s."));
  Serial.println(F(">>> CONTE quantas vezes o CORPO (a base) gira. Comeca em 3s."));
  delay(3000);

  desligaBase();
  desligaCotovelo();
  if (servoOmbro.attached()) servoOmbro.detach();
  if (servoGarra.attached()) servoGarra.detach();

  //  Cada pino pisca um número diferente de vezes: o servo ligado àquele pino
  //  pisca igual. Contando as piscadas do corpo você nos diz qual pino é a base
  //  sem precisar ler nada no monitor.  D11=1, D10=2, D6=3, D9=4, D5=5, D7=6.
  for (uint8_t i = 0; i < total; i++) {
    Serial.print(F("  D")); Serial.print(candidatos[i]);
    Serial.print(F(" -> ")); Serial.print(i + 1); Serial.println(F(" piscada(s)"));
    Servo teste;
    teste.attach(candidatos[i]);
    for (uint8_t n = 0; n <= i; n++) {
      teste.writeMicroseconds(anguloParaUs(70));  delay(300);
      teste.writeMicroseconds(anguloParaUs(110)); delay(300);
    }
    teste.writeMicroseconds(anguloParaUs(90));  delay(200);
    teste.detach();
    delay(250);
  }

  //  Devolve o torque a todas as juntas no ângulo em que elas já estavam.
  ativoBaseMs = ativoOmbroMs = ativoCotoveloMs = ativoGarraMs = millis();

  Serial.println(F(">>> FIM. Quantas vezes o CORPO girou?"));
}

// ===========================================================================
//  12b. PROTOCOLO SERIAL PARA O APP (prefixo '#')
// ===========================================================================
//  O PC não gera PWM: ele só propõe ALVOS. A rampa, os limites e o hold
//  continuam aqui, então um comando perdido não vira tranco.
//
//    #P<b>;<o>;<c>;<g>   alvos das 4 juntas, em graus (o app manda a 20 Hz).
//                        O próprio reenvio é o heartbeat.
//    #M                  assume o controle (parte do ângulo atual, sem salto)
//    #J                  devolve o controle ao joystick
//    #H                  segura agora (STOP)
//    #S                  manda o status uma vez
//    #B                  diagnóstico: versão, boots e limites
//
//  Regras de segurança:
//    - 500 ms sem nenhum #P -> segura as juntas e volta ao joystick
//    - com o PC no controle o joystick físico fica TRAVADO (não interfere);
//      #J, #H, o timeout ou a queda do cabo devolvem o controle

unsigned long ultimoComandoPcMs = 0;
const unsigned long PC_TIMEOUT_MS = 500;

//  Sticks virtuais do app (toque): -1..+1 por eixo, no lugar da leitura do ADC.
//  Entram no MESMO caminho dos joysticks físicos (curva, zona morta, histerese),
//  para o toque ter exatamente a mesma sensação da alavanca de verdade.
bool modoVirtual = false;
float vJoy1X = 0, vJoy1Y = 0, vJoy2X = 0, vJoy2Y = 0;

//  Único lugar que liga/desliga a trava de pressão da garra: o botão físico
//  (D4) e o comando #L do app passam por aqui, então o estado nunca diverge.
void aplicaTravaGarra(bool travar) {
  garraTravada = travar ? 1 : 0;
  ativoGarraMs = millis();
  if (garraTravada) {
    // Aperto extra de 2.5° para firmar a pressão sobre o objeto
    alvoGarra = constrainAngle(angGarra - 2.5, limGarraMin, limGarraMax);
    digitalWrite(PIN_LED, HIGH);
    Serial.println(F("GARRA TRAVADA: mantendo pressao continua"));
  } else {
    alvoGarra = angGarra;
    digitalWrite(PIN_LED, LOW);
    Serial.println(F("GARRA LIBERADA: abertura habilitada"));
  }
}

void seguraJuntas() {
  alvoBase     = angBase;
  alvoOmbro    = angOmbro;
  alvoCotovelo = angCotovelo;
  alvoGarra    = angGarra;
}

//  Status de 1 linha: ângulos atuais + quem está no controle.
//  Buffer montado de uma vez para não segurar o loop com 10 Serial.print.
void enviaStatus() {
  char buf[48];
  int n = snprintf_P(buf, sizeof(buf), PSTR("S%d;%d;%d;%d;%d;%d\n"),
                     (int)angBase, (int)angOmbro, (int)angCotovelo, (int)angGarra,
                     modoPC ? 1 : 0, garraTravada ? 1 : 0);
  Serial.write((const uint8_t *)buf, n);
}

void enviaDiag() {
  char buf[110];
  int n = snprintf_P(buf, sizeof(buf),
                     PSTR("B%s;boots=%u;limb=%d;%d;limo=%d;%d;limc=%d;%d;limg=%d;%d\n"),
                     FW_VER, (unsigned)bootCount,
                     (int)limBaseMin, (int)limBaseMax,
                     (int)limOmbroMin, (int)limOmbroMax,
                     (int)limCotoveloMin, (int)limCotoveloMax,
                     (int)limGarraMin, (int)limGarraMax);
  Serial.write((const uint8_t *)buf, n);
}

//  Lê "#P90;60;45;20". Qualquer campo inválido descarta a linha inteira, para
//  um comando corrompido nunca virar alvo no meio do caminho.
//  Lê exatamente 4 inteiros separados por ';' a partir de `inicio`.
bool leQuatroInteiros(const char *l, uint8_t n, uint8_t inicio, int *v) {
  uint8_t k = 0, i = inicio;
  while (i < n && k < 4) {
    bool negativo = false;
    if (l[i] == '-') { negativo = true; i++; }
    bool temDigito = false;
    int val = 0;
    while (i < n && l[i] >= '0' && l[i] <= '9') {
      val = val * 10 + (l[i] - '0');
      i++;
      temDigito = true;
    }
    if (!temDigito) return false;            // campo vazio ou lixo: descarta
    v[k++] = negativo ? -val : val;
    while (i < n && l[i] == ';') i++;
  }
  return k == 4;
}

void processaComandoPc(const char *l, uint8_t n) {
  int v[4];
  switch (l[1]) {
    case 'P':                                // alvos absolutos (replay/ajuste fino)
      if (!leQuatroInteiros(l, n, 2, v)) return;
      modoPC = true;
      modoVirtual = false;
      alvoBase     = constrainAngle(v[0], limBaseMin, limBaseMax);
      alvoOmbro    = constrainAngle(v[1], limOmbroMin, limOmbroMax);
      alvoCotovelo = constrainAngle(v[2], limCotoveloMin, limCotoveloMax);
      alvoGarra    = constrainAngle(v[3], limGarraMin, limGarraMax);
      //  Com a trava ativa a garra só aperta: nunca solta o objeto sozinha.
      if (garraTravada && alvoGarra > angGarra) alvoGarra = angGarra;
      ultimoComandoPcMs = millis();
      break;

    case 'V':                                // sticks virtuais, -100..100
      if (!leQuatroInteiros(l, n, 2, v)) return;
      modoPC = true;
      modoVirtual = true;
      vJoy1X = constrain(v[0], -100, 100) / 100.0;
      vJoy1Y = constrain(v[1], -100, 100) / 100.0;
      vJoy2X = constrain(v[2], -100, 100) / 100.0;
      vJoy2Y = constrain(v[3], -100, 100) / 100.0;
      ultimoComandoPcMs = millis();
      break;

    case 'L':                                // #L1 trava a garra, #L0 libera
      if (l[2] == '1') aplicaTravaGarra(true);
      else if (l[2] == '0') aplicaTravaGarra(false);
      break;

    case 'M': modoPC = true; modoVirtual = false; ultimoComandoPcMs = millis(); break;
    case 'J': modoPC = false; modoVirtual = false; vJoy1X = vJoy1Y = vJoy2X = vJoy2Y = 0; break;
    case 'H': modoPC = false; modoVirtual = false; seguraJuntas(); break;
    case 'S': enviaStatus(); break;
    case 'B': enviaDiag();   break;
  }
}

// ===========================================================================
//  13. LOOP — roda para sempre, milhares de vezes por segundo
// ===========================================================================

void loop() {

  //  --- ETAPA A: Ler comandos do serial -------------------------------
  //  Comandos de 1 letra (h, s, r, d, b, l, [ , ]) continuam funcionando como
  //  sempre. Linhas começando com '#' são o protocolo do app e só executam no
  //  fim da linha, para um comando pela metade nunca ser interpretado.
  static char linha[40];
  static uint8_t lenLinha = 0;

  while (Serial.available() > 0) {
    char c = Serial.read();

    if (c == '\n' || c == '\r') {
      if (lenLinha > 0) {
        linha[lenLinha] = '\0';
        if (linha[0] == '#') processaComandoPc(linha, lenLinha);
        lenLinha = 0;
      }
      continue;
    }

    if (lenLinha > 0 || c == '#') {
      if (lenLinha < sizeof(linha) - 1) linha[lenLinha++] = c;
      continue;
    }

    switch (c) {
      case 'h': case 'H': mostraAjuda(); break;
      case 's': case 'S': status();      break;
      case 'r': case 'R': resetPark();   break;
      case '[':
        limBaseMin = constrainAngle(angBase + 1.5, 0.0, posZeroBase - 5.0);
        alvoBase = angBase = limBaseMin;
        salvaLimitesBaseEEPROM();
        break;
      case ']':
        limBaseMax = constrainAngle(angBase - 1.5, posZeroBase + 5.0, 180.0);
        alvoBase = angBase = limBaseMax;
        salvaLimitesBaseEEPROM();
        break;
      case 'l': case 'L':
        limBaseMin = 5.0;
        limBaseMax = 175.0;
        salvaLimitesBaseEEPROM();
        break;
      case 'd': case 'D':
        debugOn = !debugOn;   // inverte o valor (liga<->desliga)
        Serial.print(F("debug "));
        Serial.println(debugOn ? F("ON") : F("OFF"));
        break;
      case 'b': case 'B':
        descobrePinoBase();
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

  //  --- ETAPA B0: heartbeat do app (500 ms sem #P = solta e segura) -----
  if (modoPC && (agora - ultimoComandoPcMs > PC_TIMEOUT_MS)) {
    modoPC = false;
    seguraJuntas();
  }

  int joy1X = leEixoSuave(PIN_JOY1_X);
  int joy1Y = leEixoSuave(PIN_JOY1_Y);
  int joy2X = leEixoSuave(PIN_JOY2_X);
  int joy2Y = leEixoSuave(PIN_JOY2_Y);

  //  --- ETAPA B2: Botão 1 (Joystick Esquerdo, D2) -> Localização (Posição 0)
  //      * Segurar por ~2s (1800 ms) = Salva localização atual na EEPROM
  //      * Clique rápido             = Vai suavemente para a localização salva
  bool btn1Agora = (digitalRead(PIN_BTN_JOY1) == LOW);
  if (btn1Agora && !btn1Pressionado) {
    btn1Pressionado = true;
    btn1SalvouNoHold = false;
    inicioPressaoBtn1Ms = agora;
  } else if (btn1Agora && btn1Pressionado) {
    if (!btn1SalvouNoHold && (agora - inicioPressaoBtn1Ms >= 1800)) {
      btn1SalvouNoHold = true;
      voltandoZero = false;
      salvaPosicaoZeroEEPROM();
    }
  } else if (!btn1Agora && btn1Pressionado) {
    btn1Pressionado = false;
    if (!btn1SalvouNoHold && (agora - inicioPressaoBtn1Ms >= 40)) {
      resetPark();
    }
  }

  //  --- ETAPA B3: Botão 2 (Joystick Direito, D4) -> Trava de Pressão da Garra + Limite da Base
  //      * 1º clique rápido = Indica objeto na garra: mantém pressão contínua e bloqueia abertura
  //      * 2º clique rápido = Libera a trava para permitir abrir a garra novamente
  //      * Segurar por ~2s  = Salva o limite lateral da Base antes do parafuso (com recuo de 1.5°
  //                           para nunca forçar), ou restaura limites se a Base estiver no centro
  bool btn2Agora = (digitalRead(PIN_BTN_JOY2) == LOW);
  if (btn2Agora && !btn2Pressionado) {
    btn2Pressionado = true;
    btn2SalvouNoHold = false;
    inicioPressaoBtn2Ms = agora;
  } else if (btn2Agora && btn2Pressionado) {
    if (!btn2SalvouNoHold && (agora - inicioPressaoBtn2Ms >= 1800)) {
      btn2SalvouNoHold = true;
      if (angBase < posZeroBase - 10.0) {
        limBaseMin = constrainAngle(angBase + 1.5, 0.0, posZeroBase - 5.0);
        alvoBase = angBase = limBaseMin;
        ativoBaseMs = agora;
      } else if (angBase > posZeroBase + 10.0) {
        limBaseMax = constrainAngle(angBase - 1.5, posZeroBase + 5.0, 180.0);
        alvoBase = angBase = limBaseMax;
        ativoBaseMs = agora;
      } else {
        limBaseMin = 5.0;
        limBaseMax = 175.0;
      }
      salvaLimitesBaseEEPROM();
    }
  } else if (!btn2Agora && btn2Pressionado) {
    btn2Pressionado = false;
    if (!btn2SalvouNoHold && (agora - inicioPressaoBtn2Ms >= 40)) {
      aplicaTravaGarra(!garraTravada);
    }
  }

  //  Se o usuário mexer em qualquer joystick durante o retorno, assume controle manual
  if (voltandoZero) {
    if (abs(joy1X - centroJoy1X) > JOY_DEADZONE * 2 ||
        abs(joy1Y - centroJoy1Y) > JOY_DEADZONE * 2 ||
        abs(joy2X - centroJoy2X) > JOY_DEADZONE * 2 ||
        abs(joy2Y - centroJoy2Y) > JOY_DEADZONE * 2) {
      voltandoZero = false;
      alvoBase     = angBase;
      alvoOmbro    = angOmbro;
      alvoCotovelo = angCotovelo;
      alvoGarra    = angGarra;
    } else {
      // Move UMA junta por vez para a Posição 0 salva (evita pico de corrente na fonte 12V).
      // Se a garra estiver travada segurando objeto, NÃO altera a garra para não soltá-lo!
      if (abs(angCotovelo - posZeroCotovelo) > 0.5) {
        alvoCotovelo = posZeroCotovelo;
      } else if (abs(angOmbro - posZeroOmbro) > 0.5) {
        alvoOmbro = posZeroOmbro;
      } else if (abs(angBase - posZeroBase) > 0.5) {
        alvoBase = posZeroBase;
      } else if (!garraTravada && abs(angGarra - posZeroGarra) > 0.5) {
        alvoGarra = posZeroGarra;
      } else {
        voltandoZero = false;
      }
    }
  }

  //  --- ETAPA C: joystick altera os ALVOS proporcionalmente -----------
  //    joy1X (A0) → Base       (esquerdo, esquerda/direita) gira a base (-GANHO_BASE inverte sentido)
  //    joy1Y (A1) → Cotovelo   (esquerdo, cima/baixo)       movimenta o cotovelo
  //    joy2Y (A2) → Ombro      (direito,  cima/baixo)       sobe/desce o ombro
  //    joy2X (A3) → Garra      (direito,  esquerda/direita) esquerda=fecha, direita=abre

  if (!voltandoZero) {
    //  Com o PC no controle o joystick FÍSICO fica travado: mexer a alavanca não
    //  interfere. Quem devolve o controle é o app (#J), o STOP (#H), o timeout de
    //  500 ms sem heartbeat, ou a desconexão do cabo.

    //  Sticks virtuais: substitui só o valor lido, o resto do caminho é o mesmo
    //  (isolação de eixo, zona morta, curva, ganho, suavização). Tem que ser
    //  antes de copiar para j1X/j2X, senão a cópia leva a leitura física.
    if (modoPC && modoVirtual) {
      joy1X = constrain(centroJoy1X + (int)(vJoy1X * 512.0), 0, 1023);
      joy1Y = constrain(centroJoy1Y + (int)(vJoy1Y * 512.0), 0, 1023);
      joy2X = constrain(centroJoy2X + (int)(vJoy2X * 512.0), 0, 1023);
      joy2Y = constrain(centroJoy2Y + (int)(vJoy2Y * 512.0), 0, 1023);
    }

    int j1X = joy1X, j1Y = joy1Y;
    int j2X = joy2X, j2Y = joy2Y;
    isolaEixoJoy1Esquerdo(j1X, centroJoy1X, j1Y, centroJoy1Y, agora);
    isolaEixoExclusivo(j2X, centroJoy2X, j2Y, centroJoy2Y, eixoTravaJoy2);

    if (!modoPC || modoVirtual) {
      if (eixoTravaJoy1 == 1) {
        // Movendo Joystick Esquerdo para Esquerda/Direita: SÓ gira a Base!
        alvoBase     = aplicaEixo(j1X, centroJoy1X, alvoBase, limBaseMin, limBaseMax, -GANHO_BASE);
        alvoCotovelo = angCotovelo;
      } else if (eixoTravaJoy1 == 2) {
        // Movendo Joystick Esquerdo claramente para Cima/Baixo: SÓ move o Cotovelo!
        alvoCotovelo = aplicaEixoZona(j1Y, centroJoy1Y, alvoCotovelo, limCotoveloMin, limCotoveloMax, GANHO_COTOVEL, JOY1_DZ_Y_COTOVEL);
        alvoBase     = angBase;
      }

      if (eixoTravaJoy2 == 1) {
        // Movendo Joystick Direito para Esquerda/Direita: SÓ move a Garra!
        float novoAlvoGarra = aplicaEixo(j2X, centroJoy2X, alvoGarra, limGarraMin, limGarraMax, GANHO_GARRA);
        if (garraTravada) {
          if (novoAlvoGarra < alvoGarra) {
            alvoGarra = novoAlvoGarra;
          }
        } else {
          alvoGarra = novoAlvoGarra;
        }
        alvoOmbro = angOmbro;
      } else if (eixoTravaJoy2 == 2) {
        // Movendo Joystick Direito para Cima/Baixo: SÓ move o Ombro!
        alvoOmbro = aplicaEixo(j2Y, centroJoy2Y, alvoOmbro, limOmbroMin, limOmbroMax, GANHO_OMBRO);
        alvoGarra = angGarra;
      }
    }
  }

  //  --- ETAPA D: SUAVIZAÇÃO + ISOLAMENTO DE JUNTAS + PRESSÃO DA GARRA --
  //    - Base: energiza durante o giro e corta em 120 ms ao atingir o limite
  //      (limBaseMin/limBaseMax) para nunca ficar forçando contra parafuso ou batente.
  //    - Ombro e Cotovelo: energizam quando se movem e continuam recebendo o pulso
  //      do angulo atual enquanto a Base gira (desligados nesse instante, o braco
  //      despencaria por gravidade).
  //    - Garra: quando `garraTravada` está ativa (Botão 2), mantém o servo ligado
  //      continuamente aplicando pressão constante no objeto; quando liberada,
  //      energiza durante o movimento (+500 ms) e depois descansa o motor.

  float passoMax = VEL_SUAVE * (float)INTERVALO_CONTROLE_MS;

  if (abs(alvoBase - angBase) > 0.005) {
    angBase = suaviza(angBase, alvoBase, passoMax);
    ativoBaseMs = agora;
  }
  bool noLimiteBase = (angBase <= limBaseMin + 0.2 || angBase >= limBaseMax - 0.2);
  unsigned long holdBase = noLimiteBase ? 120 : TEMPO_HOLD_BASE_MS;
  bool baseAtiva = (ativoBaseMs != 0 && (agora - ativoBaseMs <= holdBase));
  if (baseAtiva) {
    escreveBase(anguloParaUs(angBase));
  } else {
    desligaBase();
  }

  if (abs(alvoOmbro - angOmbro) > 0.005) {
    angOmbro = suaviza(angOmbro, alvoOmbro, passoMax);
    ativoOmbroMs = agora;
  }
  if (ativoOmbroMs != 0 && (agora - ativoOmbroMs <= TEMPO_HOLD_MS)) {
    if (!servoOmbro.attached()) servoOmbro.attach(PIN_SERVO_OMBRO);
    servoOmbro.writeMicroseconds(anguloParaUs(angOmbro));
  } else if (servoOmbro.attached()) {
    servoOmbro.detach();
  }

  if (abs(alvoCotovelo - angCotovelo) > 0.005) {
    angCotovelo = suaviza(angCotovelo, alvoCotovelo, passoMax);
    ativoCotoveloMs = agora;
  }
  if (ativoCotoveloMs != 0 && (agora - ativoCotoveloMs <= TEMPO_HOLD_MS)) {
    escreveCotovelo(anguloParaUs(angCotovelo));
  } else {
    desligaCotovelo();
  }

  if (abs(alvoGarra - angGarra) > 0.005) {
    angGarra = suaviza(angGarra, alvoGarra, passoMax * 1.5);
    ativoGarraMs = agora;
  }
  bool garraMovendo = (ativoGarraMs != 0 && (agora - ativoGarraMs <= 500));
  if (garraMovendo || garraTravada) {
    if (!servoGarra.attached()) servoGarra.attach(PIN_SERVO_GARRA);
    servoGarra.writeMicroseconds(anguloParaUsGarra(angGarra));
  } else if (servoGarra.attached()) {
    servoGarra.detach();
  }

  //  --- ETAPA E: status para o app (20 Hz quando o PC está no controle) --
  static unsigned long ultimoStatusMs = 0;
  if (modoPC && agora - ultimoStatusMs >= 50) {
    ultimoStatusMs = agora;
    enviaStatus();
  }

  //  --- ETAPA F: leitura dos joysticks no monitor serial --------------

  static unsigned long ultimoPrint = 0;
  if (debugOn && agora - ultimoPrint >= 500) {
    ultimoPrint = agora;
    Serial.print(F("[JOYSTICKS] B=")); Serial.print((int)angBase);
    Serial.print(F(" O="));            Serial.print((int)angOmbro);
    Serial.print(F(" C="));            Serial.print((int)angCotovelo);
    Serial.print(F(" G="));            Serial.print((int)angGarra);
    Serial.print(F(" T="));            Serial.print((int)garraTravada);
    //  Valores CRUS do ADC (0..1023, centro ~512): é isto que mostra se o eixo
    //  do joystick está realmente chegando na placa.
    Serial.print(F(" | esq=")); Serial.print(joy1X);
    Serial.print(F("/"));       Serial.print(joy1Y);
    Serial.print(F(" dir="));   Serial.print(joy2X);
    Serial.print(F("/"));       Serial.println(joy2Y);
  }
}

// ===========================================================================
//  14. SETUP — roda UMA VEZ ao ligar/resetar
// ===========================================================================

void setup() {
  Serial.begin(BAUD);

  //  Contador de resets em .noinit: se ele subir enquanto o braço se move, a
  //  placa reiniciou (brownout da alimentação) e não é o servo que está fraco.
  if (bootMagic == BOOT_MAGIC) {
    bootCount++;
  } else {
    bootMagic = BOOT_MAGIC;
    bootCount = 1;
  }

  Serial.print(F("firmware v")); Serial.print(F(FW_VER));
  Serial.print(F(" boots=")); Serial.println(bootCount);
  pinMode(PIN_LED, OUTPUT);
  pinMode(PIN_BTN_JOY1, INPUT_PULLUP);
  pinMode(PIN_BTN_JOY2, INPUT_PULLUP);
  delay(5);

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
