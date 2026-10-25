# Casos aislados del museo para SUPER GRAFO

Cada carpeta es una ficha del museo (docs/MUSEO-UB.md y Museo/README.md):

- `mal.*`: el patron de la ficha, lo mas corto posible. El detector debe saltar.
- `bien.*`: el control valido que propone la ficha. El detector debe callarse.

No se compilan ni se ejecutan: son texto que lee `tests/museo.rs`. Las fichas sin
carpeta (UB-006, UB-010, UB-013, UB-016, UB-019, UB-020, ASM-006 y los contrastes
NOUB) no tienen detector: sin compilar no se pueden ver con honestidad.
