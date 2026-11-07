/*
 * BRAÇO ROBÓTICO 4-DOF — Firmware novo (braço só se move com joystick)
 *
 * Diferenças do braco_4dof.ino antigo:
 *   - SEM retorno automático ao repouso (era isso que mexia o braço sozinho
 *     depois de 3 s parado). O braço fica onde você deixa.
 *   - Servo da base no pino correto D9 (o fio laranja vai por jumper;
 *     o socket da base na shield não existe). O antigo usava D7.
 *   - Suavização por taxa fixa ( passo a cada 20 ms ) em vez de depender
 *     da velocidade do loop.
 *
 * PINAGEM
 *   Servo Base     D9
 *   Servo Ombro    D10      Joystick esquerdo X A0 → Cotovelo
 *   Servo Cotovelo D11      Joystick esquerdo Y A1 → Base
 *   Servo Garra    D6       Joystick direito  X A2 → Garra
 *   LED            D13      Joystick direito  Y A3 → Ombro
 *
 * Serial 115200: h=ajuda t=teste s=status r=reset
 *
 * ⚠️ Servos sempre em fonte 5 V/3 A separada, GND comum ao Arduino.
 */

#include <Servo.h>

// ---- Pinos ---------------------------------------------------------------
const uint8_t PIN_SERVO_BASE    = 9;    // fio laranja da base vai aqui por jumper
const uint8_t PIN_SERVO_OMBRO   = 10;
const uint8_t PIN_SERVO_COTOVEL = 11;
const uint8_t PIN_SERVO_GARRA   = 6;
const uint8_t PIN_LED           = 13;

const uint8_t PIN_JOY1_X = A0;   // Cotovelo
const uint8_t PIN_JOY1_Y = A1;   // Base
const uint8_t PIN_JOY2_X = A2;   // Garra
const uint8_t PIN_JOY2_Y = A3;   // Ombro

// ---- Ajuste fino ---------------------------------------------------------
const int  JOY_CENTER   = 512;   // centro do joystick (0..1023)
const int  JOY_DEADZONE = 30;    // zona morta: abaixo disso ignora
const float GANHO       = 2.5;   // graus por leitura fora da zona morta
const float PASSO_SMOOTH = 0.15; // fração do caminho por passo (20 ms)

// Repouso e limites mecânicos (eras do manual — ajuste antes do teste 't')
const float PARK_BASE = 90, PARK_OMBRO = 90, PARK_COTOVEL = 90, PARK_GARRA = 0;
float limBaseMin = 0,   limBaseMax = 180;
float limOmbroMin = 30, limOmbroMax = 150;
float limCotoveloMin = 20, limCotoveloMax = 160;
float limGarraMin = 0,  limGarraMax = 90;

Servo servoBase, servoOmbro, servoCotovelo, servoGarra;

// Ângulos ATUAL (o que o servo mostra) e ALVO (pedido pelo joystick)
float angBase = PARK_BASE,       alvoBase = PARK_BASE;
float angOmbro = PARK_OMBRO,     alvoOmbro = PARK_OMBRO;
float angCotovelo = PARK_COTOVEL, alvoCotovelo = PARK_COTOVEL;
float angGarra = PARK_GARRA,     alvoGarra = PARK_GARRA;

// ---- Helpers -------------------------------------------------------------

// Sinal negativo no ganho (`-GANHO`) inverte o sentido do eixo se preciso
void aplicaEixo(int leitura, float &alvo, float minA, float maxA, float ganho) {
  int e = leitura - JOY_CENTER;
  if (abs(e) <= JOY_DEADZONE) return;   // centrado: mantém
  int excesso = e - (e > 0 ? JOY_DEADZONE : -JOY_DEADZONE);
  alvo += excesso * ganho;
  if (alvo < minA) alvo = minA;
  if (alvo > maxA) alvo = maxA;
}

// Caminha o ATUAL na direção do ALVO, um passo suave
float suaviza(float atual, float alvo) {
  float d = alvo - atual;
  if (d > -0.5 && d < 0.5) return alvo;
  return atual + d * PASSO_SMOOTH;
}

void enviaServos() {
  servoBase.write((int)angBase);
  servoOmbro.write((int)angOmbro);
  servoCotovelo.write((int)angCotovelo);
  servoGarra.write((int)angGarra);
}

// Libera o braço de um servo por vez, com pausa
void testaServo(Servo &s, float minA, float maxA, const __FlashStringHelper *nome) {
  Serial.print(F("testando ")); Serial.println(nome);
  for (int i = 0; i <= 20; i++) {
    s.write(minA + (maxA - minA) * i / 20.0);
    delay(50);
  }
  for (int i = 20; i >= 0; i--) {
    s.write(minA + (maxA - minA) * i / 20.0);
    delay(50);
  }
}

// ---- Setup / loop --------------------------------------------------------

void setup() {
  Serial.begin(115200);
  pinMode(PIN_LED, OUTPUT);

  servoBase.attach(PIN_SERVO_BASE);
  servoOmbro.attach(PIN_SERVO_OMBRO);
  servoCotovelo.attach(PIN_SERVO_COTOVEL);
  servoGarra.attach(PIN_SERVO_GARRA);

  // Sequência segura: garra fecha sozinha, depois o resto vai ao centro
  angGarra = alvoGarra = PARK_GARRA;
  servoGarra.write(PARK_GARRA);
  delay(400);

  angBase = alvoBase = PARK_BASE;
  angOmbro = alvoOmbro = PARK_OMBRO;
  angCotovelo = alvoCotovelo = PARK_COTOVEL;
  enviaServos();
  delay(400);

  for (int i = 0; i < 3; i++) {   // 3 piscadas = pronto
    digitalWrite(PIN_LED, HIGH); delay(120);
    digitalWrite(PIN_LED, LOW);  delay(120);
  }

  Serial.println(F("braco_novo pronto"));
  Serial.println(F("comandos: h=ajuda t=teste s=status r=reset"));
  enviaServos();
}

void loop() {
  // 1. Comandos do serial (não-trava: lê e executa à mão)
  while (Serial.available() > 0) {
    switch (Serial.read()) {
      case 't': case 'T': {
        Serial.println(F("=== MODO TESTE: libere o mecanismo ==="));
        delay(2000);
        testaServo(servoBase,    limBaseMin,    limBaseMax,    F("base"));
        enviaServos(); delay(600);
        testaServo(servoOmbro,   limOmbroMin,   limOmbroMax,   F("ombro"));
        enviaServos(); delay(600);
        testaServo(servoCotovelo, limCotoveloMin, limCotoveloMax, F("cotovelo"));
        enviaServos(); delay(600);
        testaServo(servoGarra,   limGarraMin,   limGarraMax,   F("garra"));
        Serial.println(F("=== TESTE CONCLUIDO ==="));
        break;
      }
      case 's': case 'S':
        Serial.print(F("B=")); Serial.print((int)angBase);
        Serial.print(F(" O=")); Serial.print((int)angOmbro);
        Serial.print(F(" C=")); Serial.print((int)angCotovelo);
        Serial.print(F(" G=")); Serial.println((int)angGarra);
        break;
      case 'r': case 'R':
        alvoBase = PARK_BASE;   alvoOmbro = PARK_OMBRO;
        alvoCotovelo = PARK_COTOVEL; alvoGarra = PARK_GARRA;
        Serial.println(F("voltando ao repouso..."));
        break;
    }
  }

  // 2. Lê os 4 eixos
  int joy1X = analogRead(PIN_JOY1_X);
  int joy1Y = analogRead(PIN_JOY1_Y);
  int joy2X = analogRead(PIN_JOY2_X);
  int joy2Y = analogRead(PIN_JOY2_Y);

  // 3. Joystick → alvos (a cada 20 ms, taxa fixa)
  static unsigned long ultimoTick = 0;
  if (millis() - ultimoTick >= 20) {
    ultimoTick = millis();
    aplicaEixo(joy1X, alvoCotovelo, limCotoveloMin, limCotoveloMax, GANHO);
    aplicaEixo(joy1Y, alvoBase,     limBaseMin,     limBaseMax,     GANHO);
    aplicaEixo(joy2Y, alvoOmbro,    limOmbroMin,    limOmbroMax,    GANHO);
    aplicaEixo(joy2X, alvoGarra,    limGarraMin,    limGarraMax,    GANHO);

    // 4. Suaviza: ATUAL caminha até o ALVO
    angBase     = suaviza(angBase,      alvoBase);
    angOmbro    = suaviza(angOmbro,     alvoOmbro);
    angCotovelo = suaviza(angCotovelo,  alvoCotovelo);
    angGarra    = suaviza(angGarra,     alvoGarra);

    enviaServos();
  }
}
