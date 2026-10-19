# Aureon

**Finanzas personales, en tu equipo y en tu idioma.**

Aureon es una aplicación de escritorio para administrar tus finanzas personales desde
un solo lugar: cuentas, movimientos, compras en cuotas, suscripciones, cuentas por
cobrar y por pagar, presupuestos, reportes y calendario de compromisos.

Todos los datos se guardan en tu computadora, en una base de datos SQLite. Aureon
funciona completamente **sin internet**, sin cuentas y sin servidores. No se envía
ningún dato financiero a ningún servicio externo.

---

## Índice

1. [Funcionalidades](#funcionalidades)
2. [Instalación](#instalación)
3. [Dónde se guardan tus datos](#dónde-se-guardan-tus-datos)
4. [Compilación y desarrollo](#compilación-y-desarrollo)
5. [Pruebas](#pruebas)
6. [Arquitectura](#arquitectura)
7. [Decisiones de diseño](#decisiones-de-diseño)
8. [Estructura del proyecto](#estructura-del-proyecto)
9. [Seguridad y privacidad](#seguridad-y-privacidad)
10. [Limitaciones conocidas](#limitaciones-conocidas)

---

## Funcionalidades

### Panel principal
Saldo total, ingresos, gastos y balance del período, dinero por cobrar y por pagar,
cuotas pendientes, suscripciones activas y su costo mensual estimado, próximos
pagos, depósitos pendientes, compromisos vencidos, movimientos recientes y gráficos
de ingresos/gastos, saldo, categorías y comercio. El período es configurable.

### Movimientos
Ingresos, gastos, transferencias entre cuentas y devoluciones. Cada movimiento
tiene descripción, monto, fecha, categoría, cuenta, comercio o persona, método de
pago, notas y etiquetas. Filtros por período, tipo, categoría, cuenta, persona,
método de pago y rango de montos, con búsqueda y ordenamiento.

**Reglas:** una transferencia entre cuentas propias no es ingreso ni gasto; un gasto
afecta el saldo una sola vez; una devolución debe vincularse a su gasto original y no
puede superarlo; los movimientos se anulan lógicamente para conservar la auditoría.

### Cuentas
Efectivo, corriente, vista, RUT, ahorro, tarjeta de crédito y otros medios. El saldo
se deriva del saldo inicial más los movimientos, por lo que nunca se desincroniza.
Las cuentas archivadas conservan su historial. Las tarjetas de crédito se tratan
como cuentas con saldo negativo: la compra es un gasto y el pago de la tarjeta es una
transferencia, no un segundo gasto.

### Categorías y presupuestos
Categorías propias de ingresos y gastos, y presupuestos mensuales por categoría con
monto gastado, saldo restante, porcentaje utilizado e indicadores visuales al acercarse
o superar el límite. Un presupuesto nunca genera movimientos.

### Compras y cuotas
Compras al contado o financiadas, con calendario de cuotas generado automáticamente.
Admite cualquier frecuencia de pago (semanal, quincenal, mensual, bimestral,
trimestral, semestral, anual o personalizada). Cada cuota se puede pagar total o
parcialmente, y el pago se registra en una transacción atómica junto con el gasto
correspondiente. No se puede pagar dos veces la misma cuota. Incluye cancelación de
compras y devoluciones.

### Suscripciones
Netflix, Crunchyroll, Spotify, Disney+, YouTube Premium, Amazon Prime, nube,
software, membresías, telefonía o cualquier otro servicio. Frecuencias normalizadas
para estimar el costo mensual y anual (las estimaciones se identifican como tales).
Historial de cobros que se conserva al cambiar el precio, pausar o cancelar.
Permite compartir una suscripción entre varias personas.

### Cuentas por cobrar y por pagar
Registro de personas y entidades con motivo, total, fechas, saldo pendiente, pagos
parciales, historial y estado. El dinero por cobrar **no** cuenta como disponible
hasta que se recibe; el dinero por pagar se descuenta de la cuenta sólo cuando se
paga.

### Depósitos pendientes
Transferencias, depósitos y pagos que tienes que realizar, con destinatario, monto,
fecha límite, cuenta de origen, institución de destino y estado (pendiente,
programado, completado, vencido, cancelado). Crear un depósito no altera ningún
saldo; al completarlo puedes registrar el movimiento real o no, y elegir entre
transferencia entre tus cuentas o pago a un tercero.

### Lista de deseos
Productos con URL, tienda, precio estimado, categoría, prioridad y meta de ahorro.
Puedes pegar sólo el enlace y completar el resto a mano, sin conexión. Al convertir un
producto en compra se piden los datos que falten y no se registra ningún gasto hasta
que exista una operación real.

### Historial y reportes
Períodos: hoy, ayer, semana actual y anterior, mes actual y anterior, año actual y
anterior, y rango personalizado. Indicadores de ingresos, gastos, balance, devoluciones,
transferencias (presentadas aparte), compras, pagos de cuotas, pagos de suscripciones,
dinero cobrado y pagado, con comparación contra el período anterior. Exportación a CSV
y a PDF.

### Calendario
Vencimientos de cuotas, próximos cobros de suscripciones, cuentas por pagar, cuentas
por cobrar y depósitos pendientes, con vistas de día, semana y mes. Marca claramente
los vencidos, los próximos y los completados. Nunca marca un pago como realizado de
forma automática.

### Copias de seguridad
Crear una copia consistente de la base de datos (incluso con la aplicación abierta),
elegir dónde guardarla, validarla antes de restaurarla y crear automáticamente una
copia preventiva de los datos actuales antes de sobrescribirlos.

---

## Instalación

1. Descarga el instalador `Aureon_<versión>_x64-setup.exe` (carpeta
   `src-tauri/target/release/bundle/nsis/` tras compilar, o el artefacto entregado).
2. Ejecuta el instalador. Se instala para el usuario actual, sin permisos de
   administrador.
3. Abre Aureon desde el menú Inicio o desde el acceso directo del escritorio.
No necesitas instalar Node.js, Rust ni ninguna otra herramienta: el instalador
incluye todo.

**Al desinstalar**, los datos financieros **no** se borran automáticamente. El
instalador informa de ello durante la desinstalación. Para eliminarlos manualmente,
borra la carpeta:

```
C:\Users\<tu usuario>\AppData\Local\com.aureon.app
```

---

## Dónde se guardan tus datos

| Elemento | Ruta en Windows |
|---|---|
| Base de datos | `%LOCALAPPDATA%\com.aureon.app\aureon.db` |
| Archivos WAL / SHM | `%LOCALAPPDATA%\com.aureon.app\aureon.db-wal`, `aureon.db-shm` |
| Documentos adjuntos | `%LOCALAPPDATA%\com.aureon.app\attachments\` |
| Copias de seguridad | La carpeta que elijas al crearlas |

La base de datos **se crea la primera vez que guardas información** (una cuenta, un
movimiento, un ajuste…). Antes de eso puedes abrir la aplicación, navegar por todas
las pantallas y ver los estados vacíos sin que se cree ningún archivo.

---

## Compilación y desarrollo

### Requisitos previos

| Herramienta | Versión mínima | Para qué |
|---|---|---|
| [Node.js](https://nodejs.org) | 18 o superior | interfaz (React + Vite) |
| [Rust](https://rustup.rs) | 1.77 o superior | backend nativo (Tauri 2) |
| Visual Studio Build Tools | 2019 o superior | compilador MSVC en Windows |
| Windows 10/11 | 64 bits | WebView2 ya viene instalado en Windows 11 y en las actualizaciones recientes de Windows 10 |

Instalación de Rust en Windows:

```powershell
winget install Rustlang.Rustup
```

Verificación:

```powershell
node --version
cargo --version
```

### Comandos

```powershell
# Instalar dependencias de la interfaz
npm install

# Desarrollo: abre la aplicación con recarga en caliente
npm run tauri dev

# Comprobar tipos de la interfaz
npm run typecheck

# Compilar la interfaz (dist/)
npm run build

# Pruebas del backend (reglas financieras sobre SQLite real)
npm run test

# Generar el instalador de Windows (NSIS) en src-tauri/target/release/bundle/nsis
npm run tauri build
```

El primer `npm run tauri build` tarda bastante (compila Rust en modo release con
optimización y LTO). Las compilaciones siguientes son mucho más rápidas.

### Estructura de `src-tauri`

```
src-tauri/
├── src/
│   ├── core/            lógica de negocio, sin depender de Tauri
│   │   ├── db.rs        apertura de SQLite y pragmas
│   │   ├── migrations.rs migraciones versionadas
│   │   ├── money.rs     parseo y formato de importes
│   │   ├── dates.rs     fechas, períodos y frecuencias
│   │   ├── models.rs    estructuras de dominio
│   │   ├── repos/       consultas SQL parametrizadas
│   │   └── services/    reglas financieras y operaciones
│   ├── commands/        comandos Tauri (una capa fina sobre los servicios)
│   ├── state.rs         estado de la aplicación y ciclo de vida de la base
│   ├── lib.rs           registro de comandos y plugins
│   └── main.rs
├── migrations/          archivos SQL de las migraciones
├── capabilities/        permisos mínimos de la ventana
├── tests/               pruebas de integración
├── windows/             hook de instalación (NSIS)
└── tauri.conf.json
```

### Regenerar los iconos

```powershell
python tools\generar_iconos.py
```

---

## Pruebas

Las pruebas del backend se ejecutan sobre una base de datos SQLite real, creada en un
directorio temporal, y no requieren la interfaz.

```powershell
cd src-tauri
cargo test
```

Cubren, entre otros:

- creación diferida de la base de datos y persistencia tras cerrar la aplicación;
- migraciones de esquema sin destruir datos;
- cálculo de saldos, ingresos, gastos y transferencias sin doble contabilización;
- devoluciones ligadas a su gasto original, con límite de monto;
- compras al contado, generación de cuotas, reparto de montos no divisibles;
- pagos de cuotas, pagos duplicados y pagos parciales;
- vencimientos detectados sin marcarse como pagados;
- cobros de suscripciones, cambio de precio y cancelación con historial intacto;
- cuentas por cobrar y por pagar, con saldos pendientes;
- depósitos pendientes y su finalización;
- conversión de productos deseados en compras;
- filtros del historial, exportaciones CSV y copias de seguridad con restauración;
- errores de escritura y atomicidad de las operaciones.

Además, `npm run typecheck` comprueba el tipado estricto de la interfaz.

---

## Arquitectura

```
┌──────────────────────── Interfaz (React + TypeScript) ────────────────────────┐
│  pages / features   ·   componentes   ·   servicios (API tipada)   ·   utils   │
└───────────────────────────────────┬───────────────────────────────────────────┘
                                    │  invoke() con payloads tipados
┌───────────────────────────────────▼───────────────────────────────────────────┐
│  Commands (Tauri) — capa fina: traduce tipos, no contiene reglas de negocio   │
└───────────────────────────────────┬───────────────────────────────────────────┘
┌───────────────────────────────────▼───────────────────────────────────────────┐
│  Servicios — reglas financieras, validaciones, atomicidad, agregados          │
└───────────────────────────────────┬───────────────────────────────────────────┘
┌───────────────────────────────────▼───────────────────────────────────────────┐
│  Repositorios — consultas SQL parametrizadas, sin reglas de negocio           │
└───────────────────────────────────┬───────────────────────────────────────────┘
┌───────────────────────────────────▼───────────────────────────────────────────┐
│  SQLite (WAL, claves foráneas activas, migraciones versionadas)               │
└───────────────────────────────────────────────────────────────────────────────┘
```

Reglas de la arquitectura:

- **La interfaz nunca consulta SQL.** Todo pasa por comandos tipados en
  `src/services/api.ts`.
- **Los comandos son finos.** Validan la forma del dato y delegan en los servicios.
- **Los servicios concentran las reglas.** Validación de montos, fechas, cuentas,
  estados y coherencia de saldos.
- **Los repositorios sólo ejecutan SQL.** Todas las consultas usan parámetros
  vinculados, nunca concatenación de cadenas.
- **El núcleo (`core/`) no depende de Tauri**, por lo que se puede probar de forma
  aislada y reutilizar.

---

## Decisiones de diseño

### Importes como enteros

Los importes se guardan como enteros en la **unidad mínima** de la moneda: pesos
enteros para CLP (exponente 0) y centavos para USD/EUR (exponente 2). El exponente
vive en la tabla `currencies`. Nunca se usa punto flotante para dinero, de modo que
no hay errores de redondeo acumulado.

La convención de entrada y salida es chilena: punto para los miles y coma para los
decimales (`$1.234.567`, `US$1.234,56`). El motor de parseo acepta ambas formas
habituales y está documentado en `core/money.rs` y `utils/format.ts`.

### No se convierte entre monedas

No existe conversión automática: cada cuenta, movimiento y compromiso tiene su
moneda, y los agregados se calculan **por moneda**. En el panel principal se muestra
la moneda principal y el resto de monedas con información propia. Es la única forma
de no inventar cifras.

### Saldo derivado, no almacenado

El saldo de una cuenta se calcula siempre como `saldo inicial + movimientos`. No hay
una columna de saldo que pueda quedar desincronizada; es imposible que el saldo y los
movimientos discrepen.

### Creación diferida de la base de datos

`AppState` mantiene la conexión opcional. Al arrancar, si el archivo existe se abre
y se migra; si no existe, la aplicación funciona en modo «sin datos». La primera
operación que necesita persistencia crea el archivo, aplica el esquema y registra la
migración. Las tareas de mantenimiento (detectar vencimientos) no crean la base de
datos: no hacen nada si todavía no hay información.

### Anulación lógica

Los movimientos nunca se borran: se anulan (`voided = 1`) y se conservan para
auditoría. Los pagos de cuotas y suscripciones quedan enlazados a su origen
(`linked_type` / `linked_id`), y por eso no se pueden editar ni anular desde el
módulo de movimientos: hay que hacerlo desde el módulo correspondiente para no
romper la coherencia.

### Atomicidad

Toda operación que afecta a varios registros (pago de cuota, cobro, pago de
obligación, finalización de depósito) se ejecuta dentro de una transacción de SQLite:
si algo falla, no queda ningún registro a medias.

### Tabla de pagos por separado

Cuotas, suscripciones, cuentas por cobrar y por pagar guardan sus pagos en tablas
própias (`installment_payments`, `subscription_payments`, `receivable_payments`,
`payable_payments`). Así el historial sobrevive a los cambios de precio, a las
cancelaciones y a los ajustes de cuotas.

### Importes que no dividen exactamente

Al repartir el total de una compra en cuotas, los pesos que no dividen de forma exacta
se reparten de a uno entre las primeras cuotas, de modo que **la suma de las cuotas
es siempre exactamente igual al total** (por ejemplo, $100.000 en 3 cuotas son
$33.334 + $33.333 + $33.333).

### Errores sin detalle técnico

La interfaz nunca muestra trazas. `AppError` traduce cada fallo a un mensaje en
español comprensible; el detalle técnico se registra sólo en la consola de
desarrollo.

### Navegación y rutas

Se usa `HashRouter` porque la aplicación se sirve desde un protocolo propio y no
dispone de servidor de desarrollo con *fallback* de rutas.

---

## Estructura del proyecto

```
src/
├── components/     componentes reutilizables (modales, avisos, gráficos, iconos)
├── features/       una carpeta por módulo de la aplicación
│   ├── dashboard/      inicio y panel principal
│   ├── transactions/   movimientos
│   ├── accounts/       cuentas
│   ├── budgets/        presupuestos
│   ├── purchases/      compras y cuotas
│   ├── subscriptions/  suscripciones
│   ├── receivables/    cuentas por cobrar
│   ├── payables/       cuentas por pagar
│   ├── deposits/       depósitos pendientes
│   ├── wishlist/       lista de deseos
│   ├── history/        historial y reportes
│   ├── calendar/       calendario financiero
│   └── settings/       configuración
├── hooks/          hooks reutilizables
├── layouts/        estructura de la ventana
├── services/       API tipada y estado global de referencia
├── styles/         variables de diseño, base y componentes
├── types/          tipos de dominio
└── utils/          formato de dinero, fechas y PDF
```

La identidad visual vive en `src/styles/tokens.css`: colores, tipografías,
espaciados, radios y sombras como variables CSS, con un tema oscuro equivalente.
El dorado (`--accent`) se usa como detalle de identidad, nunca como color dominante.

---

## Seguridad y privacidad

- Los datos permanecen en el equipo del usuario: no hay servidores, cuentas ni
  sincronización.
- No hay telemetría ni envío de datos a servicios de inteligencia artificial.
- Todas las consultas SQL usan parámetros vinculados.
- Los permisos de Tauri son mínimos: `core:default` y `dialog:default`. El plugin de
  apertura de enlaces se usa **desde Rust**, no desde la interfaz, y sólo acepta
  `http` y `https`.
- Las rutas de archivo se validan antes de leer o escribir; los adjuntos se copian a
  la carpeta de datos de la aplicación.
- El contenido de la aplicación (`Content-Security-Policy`) permite únicamente
  recursos locales.
- Los errores se muestran en español, sin trazas ni detalles internos.
- **La base de datos no está cifrada.** Los datos de la cuenta de Windows ya
  protegen los archivos del usuario. Si necesitas cifrado, usa una unidad cifrada
  del sistema para la carpeta `%LOCALAPPDATA%\com.aureon.app`.

---

## Limitaciones conocidas

- **Las notificaciones fuera de la aplicación no están implementadas.** El calendario
  y los avisos de vencimiento funcionan mientras Aureon está abierto. No se promete
  recordatorios en segundo plano.
- **La base de datos no está cifrada** (ver sección anterior).
- **La lista de deseos no descarga automáticamente la información del producto.** Se
  guarda el enlace y el usuario completa nombre, tienda y precio a mano. Así el
  registro funciona siempre, incluso sin internet.
- **El PDF de reportes se genera en el equipo**, sin enviar nada a ningún servicio.
- **macOS y Linux no están verificados.** El proyecto está organizado para
  empaquetarlos (`npm run tauri build -- --bundles dmg` o `appimage`), pero sólo se
  ha probado el instalador de Windows.
- La aplicación no permite registrar dos movimientos idénticos a la vez: es
  intencional, para evitar duplicados accidentales al pulsar dos veces.

---

## Licencia

Uso interno. Todo el código y los datos pertenecen al usuario que ejecuta la
aplicación.
