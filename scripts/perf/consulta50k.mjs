/* Harness "consulta50k" (169A-5, D4). Mezcla lectora 95/5 aprox., captura
 * `cf-cache-status` por petición y para solo ante p95 > 800 ms o
 * errores > 0,1 % (D4.3). Cero BDP: staging standalone; las pocas escrituras
 * caen en la BD del staging con marca "perf-harness" (como en 169A-4).
 *
 * Uso:
 *   node scripts/perf/consulta50k.mjs --usuarios 100 --duracion 180 \
 *     --salida scripts/perf/resultados/50k-t1-u100.json [--origen-directo]
 *
 * --origen-directo: envía `Cache-Control: no-cache` en los GET para forzar
 *   MISS/BYPASS en el borde y medir el camino completo a origen (control D5.1).
 *   Nota honesta: la app ignora no-cache (resumen_cache 30 s sigue activo).
 */
import { writeFileSync } from "node:fs";

const CUENTA = { email: "demo@restaurante.com", password: "demo1234", login: "/api/auth/login" };

/* 95 lecturas ponderadas: compartido (borde) + privado/listados (origen).
 * --cacheable-peso N (def. 60): peso conjunto maps+stock. Con 90 se emula el
 * escenario 50k (borde absorbe ~95 %, origen ve ~5-10 %). */
const GETS_BASE = [
  { nombre: "dashboard_resumen", ruta: "/api/dashboard/resumen?year=2026&month=9", peso: 10 },
  { nombre: "ventas_listar", ruta: "/api/ventas?per_page=20", peso: 10 },
  { nombre: "gastos_listar", ruta: "/api/gastos?per_page=20", peso: 5 },
  { nombre: "clientes_listar", ruta: "/api/clientes?per_page=20", peso: 5 },
  { nombre: "reservas_listar", ruta: "/api/reservas?per_page=20", peso: 5 },
]; /* resto (35) */
const GETS_CACHEABLE = [
  { nombre: "article_maps", ruta: "/api/bdp/article-maps" },
  { nombre: "article_stock", ruta: "/api/bdp/article-stock" },
];
const POSTS = [
  {
    nombre: "venta_crear", ruta: "/api/ventas",
    cuerpo: () => ({
      fecha: "2026-09-17", iva_porcentaje: "10", turno: "mediodia",
      canal: "comedor", metodo_pago: "tarjeta",
      importe_base: "100.00", importe_iva: "10.00", descripcion: "perf-harness",
    }),
  },
  {
    nombre: "gasto_crear", ruta: "/api/gastos",
    cuerpo: () => ({
      fecha: "2026-09-17", tipo_documento: "factura",
      importe_base: "50.00", importe_iva: "10.50", proveedor: "perf-harness",
    }),
  },
];

/* (Bolsas construidas en main según --cacheable-peso.) */

const STOP = { p95_ms: 800, err_ratio: 0.001 };

function args() {
  const m = {};
  const argv = process.argv.slice(2);
  for (let i = 0; i < argv.length; i++) {
    const k = argv[i].replace(/^--/, "");
    if (i + 1 < argv.length && !argv[i + 1].startsWith("--")) m[k] = argv[++i];
    else m[k] = true;
  }
  return m;
}

async function login(base) {
  const r = await fetch(base + CUENTA.login, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ email: CUENTA.email, password: CUENTA.password }),
  });
  if (!r.ok) throw new Error(`login: HTTP ${r.status}`);
  return (await r.json()).token;
}

async function unaOp(base, token, op, directo) {
  const esGet = !op.cuerpo;
  const t0 = performance.now();
  try {
    const headers = { Authorization: `Bearer ${token}` };
    if (!esGet) headers["Content-Type"] = "application/json";
    if (directo && esGet) { headers["Cache-Control"] = "no-cache"; headers.Pragma = "no-cache"; }
    const r = await fetch(base + op.ruta, {
      method: esGet ? "GET" : "POST",
      headers,
      ...(!esGet ? { body: JSON.stringify(op.cuerpo()) } : {}),
    });
    await r.text();
    return {
      op: op.nombre, ms: performance.now() - t0, ok: r.ok, status: r.status,
      cf: r.headers.get("cf-cache-status") ?? "n/a",
    };
  } catch (e) {
    return { op: op.nombre, ms: performance.now() - t0, ok: false, status: 0, cf: "n/a", error: String(e).slice(0, 100) };
  }
}

function percentiles(xs) {
  if (!xs.length) return { p50: 0, p95: 0, p99: 0, n: 0 };
  const s = [...xs].sort((a, b) => a - b);
  const q = (p) => s[Math.min(s.length - 1, Math.floor(p * s.length))];
  const r = (v) => Math.round(v * 10) / 10;
  return { p50: r(q(0.5)), p95: r(q(0.95)), p99: r(q(0.99)), n: s.length };
}

async function trabajador(id, base, fin, estado, directo, rampaMs, bolsaDe) {
  await new Promise((r) => setTimeout(r, id * rampaMs));
  let token;
  try {
    token = await login(base);
  } catch (e) {
    estado.fallosLogin++;
    return;
  }
  const bolsa = bolsaDe(id);
  let i = id;
  while (Date.now() < fin.t && !fin.stop) {
    const op = bolsa[i++ % bolsa.length];
    const m = await unaOp(base, token, op, directo);
    estado.lat.push(m.ms);
    if (!m.ok) estado.errores++;
    estado.total++;
    const agg = estado.porOp[m.op] ?? (estado.porOp[m.op] = { lat: [], ok: 0, fail: 0, cf: {} });
    agg.lat.push(m.ms);
    if (m.ok) agg.ok++; else agg.fail++;
    agg.cf[m.cf] = (agg.cf[m.cf] ?? 0) + 1;
    estado.cf[m.cf] = (estado.cf[m.cf] ?? 0) + 1;
  }
}

async function vigilante(fin, estado, intervaloMs) {
  while (Date.now() < fin.t && !fin.stop) {
    await new Promise((r) => setTimeout(r, intervaloMs));
    if (estado.total < 200) continue;
    const { p95 } = percentiles(estado.lat);
    const errRatio = estado.errores / estado.total;
    if (p95 > STOP.p95_ms) { fin.stop = `p95 ${p95} ms > ${STOP.p95_ms}`; break; }
    if (errRatio > STOP.err_ratio) { fin.stop = `errores ${(errRatio * 100).toFixed(2)} % > 0,1 %`; break; }
  }
}

async function main() {
  const a = args();
  const base = (a.url ?? "https://perf.wandori.us").replace(/\/$/, "");
  const usuarios = parseInt(a.usuarios ?? "100", 10);
  const duracion = parseInt(a.duracion ?? "180", 10);
  const rampaMs = parseInt(a["rampa-ms"] ?? "50", 10);
  const directo = a["origen-directo"] === true || a["origen-directo"] === "1";
  const salida = a.salida ?? `scripts/perf/resultados/50k-u${usuarios}.json`;

  /* Bolsa según mezcla pedida (escritores: VUs id % 20 === 0). */
  const cachePeso = parseInt(a["cacheable-peso"] ?? "60", 10);
  const mitad = Math.floor(cachePeso / 2);
  const bolsaGet = [
    ...GETS_CACHEABLE.flatMap((op) => Array(mitad).fill(op)),
    ...GETS_BASE.flatMap((op) => Array(op.peso).fill(op)),
  ];
  const bolsaWriter = [...bolsaGet, ...POSTS];
  const bolsaDe = (id) => (id % 20 === 0 ? bolsaWriter : bolsaGet);

  const estado = { lat: [], total: 0, errores: 0, fallosLogin: 0, porOp: {}, cf: {} };
  const t0 = Date.now();
  const fin = { t: t0 + duracion * 1000, stop: null };
  await Promise.all([
    vigilante(fin, estado, 10000),
    ...Array.from({ length: usuarios }, (_, i) => trabajador(i, base, fin, estado, directo, rampaMs, bolsaDe)),
  ]);
  const reales_s = (Date.now() - t0) / 1000;

  const porOp = {};
  for (const [nombre, agg] of Object.entries(estado.porOp)) {
    porOp[nombre] = { ...percentiles(agg.lat), ok: agg.ok, fallos: agg.fail, cf: agg.cf };
  }
  const informe = {
    url: base, usuarios, duracion_s: duracion, reales_s: Math.round(reales_s),
    origen_directo: directo,
    parado_antes: fin.stop,
    peticiones: estado.total,
    req_s: +(estado.total / reales_s).toFixed(2),
    errores: estado.errores,
    err_ratio: +(estado.errores / Math.max(1, estado.total)).toFixed(5),
    fallos_login: estado.fallosLogin,
    global_ms: percentiles(estado.lat),
    cf_totales: estado.cf,
    por_op: porOp,
    generado: new Date().toISOString(),
  };
  writeFileSync(salida, JSON.stringify(informe, null, 2));
  console.log(JSON.stringify({ ...informe, por_op: Object.fromEntries(Object.entries(porOp).map(([k, v]) => [k, { ...v, lat: undefined }])) }, null, 2));
}

main().catch((e) => { console.error("HARNESS-ERROR:", e.message); process.exit(1); });
