# Sesion fisica: Steam y recursos

Biblioteca registrada: 5 juegos encontrados.
Juegos: Steamworks Common Redistributables, Warframe, Zenless Zone Zero, Left 4 Dead 2, Dota 2.

| Medida observada | Valor |
|---|---|
| RAM maxima muestreada del sistema | 5.22 GiB |
| VRAM maxima muestreada | 660.0 MiB |
| Uso GPU maximo muestreado | 83.0% |
| Temperatura GPU maxima muestreada | 60.0 C |
| Espacio Linux al cerrar | 2.8G libres / 16G |
| Velocidades de enlace observadas | [100] Mbps |

Warframe aparece como proceso: True.
Esto acredita inicio del ejecutable; no confirma partida ni estabilidad.

## Lo que falta

1. Revisar el desglose de almacenamiento antes de borrar caches o ampliar el ext4; ver seguimiento.md.
2. Confirmar una partida y verificar los archivos desde Steam con la compatibilidad elegida.
3. Reproducir el aviso de rectangulo invalido de pixman y distinguirlo del cierre normal del escritorio.
4. Medir tiempos de fotograma y una carga repetible antes de cambiar compilacion o graficos.

Red enp37s0: 2.316 GiB recibidos en 713.926 s medidos; pico medio por intervalo 11.723 MiB/s. No se atribuye a una aplicacion concreta.

Imagen persistente actual: 16.0 GiB; libres en su disco de Windows: 10.567 GiB.
Ampliarla de 16 a 32 GiB requiere 16 GiB adicionales, mas margen. No se amplio ni se borro nada.

## Limites de la medida

- Picos de muestras; no maximos continuos ni FPS.
- Proceso presente no acredita partida, renderizado correcto o estabilidad.
- El informe no desglosa que carpetas consumieron el espacio.
- El trafico es de la interfaz completa, no se atribuye a Warframe.
- Ausencia de error en este informe no prueba ausencia global de fallos.
