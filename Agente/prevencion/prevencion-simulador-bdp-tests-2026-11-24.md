# Prevención: el simulador BDP mentía a los tests (24/11/2026)

Cuatro fallos distintos con el mismo síntoma: la suite en verde o en rojo por
motivos que no eran el producto. Todos se descubrieron el 24/11 arreglando
`21AB-5i`; ninguno lo detectaba el gate.

## 1. El test se saltaba en verde sin Python

`Command::new("python")` en la VPS 89 no existe (solo `python3`), así que
`asegurar_simulador()` devolvía `false` y el macro hacía `return`: el test
pasaba **sin ejecutar una sola aserción** contra BDP.

- Arreglo: `interprete_python()` prueba `python3` y luego `python`, y el macro
  hace `assert!` en vez de `return`. Un simulador que no arranca es un fallo,
  no un verde.
- Regla: **un skip silencioso es un falso verde.** Si el testigo no está, el
  test falla y se anota por qué.

## 2. El simulador heredaba el `flock` de `cm-heavy`

`cargo test` corre bajo `cm-heavy` (flock sobre el descriptor 9). El simulador
se lanzaba como hijo y heredaba ese descriptor; como se queda vivo a propósito
entre tests, el lock de builds quedaba **ocupado para siempre**: el siguiente
`cm-heavy` esperaba 600 s y salía 75.

- Arreglo en `tools/bdp-weblink-simulator/server.py`:
  `cerrar_descriptores_heredados()` (cierra de 3 en adelante al arrancar) y
  `/__simulator/shutdown` para poder pararlo sin dejarlo huérfano.
- Diagnóstico: `fuser -v /run/lock/cm-heavy.lock` señala el PID; si su padre es
  1, es un huérfano. Se mata **ese PID**, nunca `pkill`.

## 3. Dos archivos compartían puerto con claves distintas

| Puerto | Archivos | Clave |
|---|---|---|
| 18766 | `bdp_service_integration`, `bdp_seguridad_escritura_simulador` | distintas |
| 18768 | `bdp_lineas_cero_simulador`, `bdp_saldo_remoto_simulador` | distintas |

El segundo en arrancar encontraba el puerto vivo (health OK), se lo quedaba y
su `reset` daba **403** — que `admin_post` se tragaba (`unwrap_or(Value::Null)`).
Los tests seguían corriendo sobre el estado del otro archivo.

- Arreglo: un puerto por archivo (18765–18771) y `admin_reset()` que hace
  `assert!` sobre el código de estado, con mensaje que nombra el puerto.
- Regla: **si dos archivos comparten puerto, la clave de admin tiene que ser la
  misma**; y un `reset` fallido se convierte en fallo del test, no en `Null`.

## 4. Tests en paralelo sobre un simulador compartido

`bdp_seguridad_escritura_simulador` no tenía mutex, así que sus tres tests
reseteaban y leían el historial del mismo simulador a la vez: `solo_lectura`
veía un login ajeno y `automatico` no veía su escritura.

- Arreglo: `static SERIE` y `let _serie = SERIE.lock().await` dentro del macro,
  como el resto de archivos del simulador.

## Lo que se queda como norma

1. Testigo propio (puerto y clave) por archivo de tests.
2. Reset que falla si no se puede limpiar.
3. Turno en exclusiva siempre que se lea el historial completo.
4. Sin skip silencioso: o corre, o falla con su motivo.
5. Cualquier hijo de un proceso bajo `cm-heavy` debe cerrar los descriptores
   heredados antes de quedarse vivo.
