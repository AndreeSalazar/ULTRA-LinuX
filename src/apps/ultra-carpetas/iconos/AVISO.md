# Iconos de terceros

| Carpeta | Origen | Licencia |
| --- | --- | --- |
| `material/` | [Material Icon Theme](https://github.com/material-extensions/vscode-material-icon-theme) (los iconos de archivos de VS Code), carpeta `icons/` | MIT, ver `material/LICENSE` |
| `lucide/` | [Lucide](https://github.com/lucide-icons/lucide), carpeta `icons/` | ISC, ver `lucide/LICENSE` |

Ambas licencias permiten copiar, modificar y redistribuir, también dentro de un
programa GPL-2.0, conservando el aviso de copyright.

Derivados aquí (no están tal cual en el repositorio de origen):

- `material/folder.svg`: la forma de carpeta de `folder-src.svg`, sin el dibujo
  interior y en el gris azulado `#90a4ae` de Material.
- `material/file.svg`: la hoja de `document.svg`, sin las líneas y en `#90a4ae`.

Los iconos de Lucide usan `stroke="currentColor"`; el programa lo cambia por blanco
al cargarlos y los tiñe con el color del tipo de archivo del tema.
