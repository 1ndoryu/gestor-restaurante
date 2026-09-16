/* Harness de carga PERF-VPS (169A-4, Fase B).
 * Cero dependencias: solo fetch + timers de Node 18+.
 * Todo contra staging standalone (sin BDP real). Escrituras solo en la BD
 * del staging (ventas/gastos de prueba con descripcion "perf-harness").
 *
 * Uso:
 *   node scripts/perf/harness.mjs --url https://perf.wandori.us \
 *     --escenario base|pico|sostenido [--usuarios N] [--duracion S] \
 *     --salida scripts/perf/resultados/<tier>-<escenario>.json
 *
 * Escenarios: base {1u,300s} · pico {10u,600s} · sostenido {3u,1800s}
 */
import { writeFileSync } from "node:fs";

const ESCENARIOS = {
  base: { usuarios: 1, duracion_s: 300 },
  pico: { usuarios: 10, duracion_s: 600 },
  sostenido: { usuarios: 3, duracion_s: 1800 },
};

const CUENTAS = [
  { email: "demo@restaurante.com", password: "demo1234", login: "/api/auth/login" },
  { email: "sara.lopez@demo.com", password: "trabajador123", login: "/api/auth/login-trabajador" },
  { email: "marta.gil@demo.com", password: "trabajador123", login: "/api/auth/login-trabajador" },
  { email: "tomas.ruiz@demo.com", password: "trabajador123", login: "/api/auth/login-trabajador" },
];

/* Operaciones "mínimas reales" con peso (B1-B6 del plan). */
const OPS = [
  { nombre: "dashboard_resumen", metodo: "GET", ruta: "/api/dashboard/resumen?year=2026&month=9", peso: 3 },
  { nombre: "ventas_listar", metodo: "GET", ruta: "/api/ventas?per_page=20", peso: 3 },
  { nombre: "article_maps", metodo: "GET", ruta: "/api/bdp/article-maps", peso: 2 },
  { nombre: "article_stock", metodo: "GET", ruta: "/api/bdp/article-stock", peso: 2 },
  { nombre: "clientes_listar", metodo: "GET", ruta: "/api/clientes?per_page=20", peso: 1 },
  { nombre: "reservas_listar", metodo: "GET", ruta: "/api/reservas?per_page=20", peso: 1 },
  { nombre: "gastos_listar", metodo: "GET", ruta: "/api/gastos?per_page=20", peso: 1 },
  {
    nombre: "venta_crear", metodo: "POST", ruta: "/api/ventas", peso: 1,
    cuerpo: () => ({
      fecha: "2026-09-16", iva_porcentaje: "10", turno: "mediodia",
      canal: "comedor", metodo_pago: "tarjeta",
      importe_base: "100.00", importe_iva: "10.00",
      descripcion: "perf-harness",
    }),
  },
  {
    nombre: "gasto_crear", metodo: "POST", ruta: "/api/gastos", peso: 1,
    cuerpo: () => ({
      fecha: "2026-09-16", tipo_documento: "factura",
      importe_base: "50.00", importe_iva: "10.50", proveedor: "perf-harness",
    }),
  },
];

const BOLSA = OPS.flatMap((op) => Array(op.peso).fill(op));

function args() {
  const m = {};
  for (let i = 2; i < process.argv.length; i += 2) {
    m[process.argv[i].replace(/^--/, "")] = process.argv[i + 1];
  }
  return m;
}

async function login(base, cuenta) {
  const r = await fetch(base + cuenta.login, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: cuenta.email, password: cuenta.password }),
  });
  if (!r.ok) throw new Error(`login ${cuenta.email}: HTTP ${r.status}`);
  const j = await r.json();
  return j.token;
}

async function unaOp(base, token, op) {
  const t0 = performance.now();
  try {
    const r = await fetch(base + op.ruta, {
      method: op.metodo,
      headers: {
        Authorization: `Bearer ${token}`,
        ...(op.metodo === "POST" ? { "Content-Type": "application/json" } : {}),
      },
      ...(op.metodo === "POST" ? { body: JSON.stringify(op.cuerpo()) } : {}),
    });
    await r.text();
    const ms = performance.now() - t0;
    return { op: op.nombre, ms, ok: r.ok, status: r.status };
  } catch (e) {
    return { op: op.nombre, ms: performance.now() - t0, ok: false, status: 0, error: String(e).slice(0, 120) };
  }
}

function percentiles(xs) {
  if (!xs.length) return { p50: 0, p95: 0, p99: 0, n: 0 };
  const s = [...xs].sort((a, b) => a - b);
  const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  return { p50: Math.round(q(0.5)), p95: Math.round(q(0.95)), p99: Math.round(q(0.99)), n: s.length };
}

async function trabajador(id, base, token, fin, metricas) {
  /* Ramp-up: escalonar arranque 5 s por VU. */
  await new Promise((r) => setTimeout(r, id * 1000));
  let i = id;
  while (Date.now() < fin) {
    const op = BOLSA[i++ % BOLSA.length];
    const m = await unaOp(base, token, op);
    metricas.push(m);
  }
}

async function main() {
  const a = args();
  const base = (a.url ?? "https://perf.wandori.us").replace(/\/$/, "");
  const esc = ESCENARIOS[a.escenario];
  if (!esc) throw new Error("escenario debe ser base|pico|sostenido");
  const usuarios = parseInt(a.usuarios ?? esc.usuarios, 10);
  const duracion = parseInt(a.duracion ?? esc.duracion_s, 10);
  const salida = a.salida ?? `scripts/perf/resultados/${a.escenario}.json`;

  const tokens = [];
  for (let i = 0; i < usuarios; i++) {
    tokens.push(await login(base, CUENTAS[i % CUENTAS.length]));
  }
  const metricas = [];
  const t0 = Date.now();
  const fin = t0 + duracion * 1000;
  await Promise.all(tokens.map((tk, i) => trabajador(i, base, tk, fin, metricas)));
  const reales_s = (Date.now() - t0) / 1000;

  const porOp = {};
  for (const op of OPS) {
    const ms = metricas.filter((m) => m.op === op.nombre && m.ok).map((m) => m.ms);
    const fallos = metricas.filter((m) => m.op === op.nombre && !m.ok).length;
    porOp[op.nombre] = { ...percentiles(ms), fallos };
  }
  const informe = {
    url: base, escenario: a.escenario, usuarios, duracion_s: duracion,
    reales_s: Math.round(reales_s),
    peticiones: metricas.length,
    req_s: +(metricas.length / reales_s).toFixed(2),
    errores: metricas.filter((m) => !m.ok).length,
    global_ms: percentiles(metricas.filter((m) => m.ok).map((m) => m.ms)),
    por_op_ms: porOp,
    generado: new Date().toISOString(),
  };
  writeFileSync(salida, JSON.stringify(informe, null, 2));
  console.log(JSON.stringify(informe, null, 2));
}

main().catch((e) => { console.error("HARNESS-ERROR:", e.message); process.exit(1); });
