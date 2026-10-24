-- SPDX-License-Identifier: GPL-2.0-only
-- ULTRA / BMO-X Neon: shell original para Hyprland 0.56.2. Acento hielo, esquinas rectas.
local vivo = os.getenv("ULTRA_EFECTOS") ~= "sobrio"
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1 })
hl.config({
    debug = { enable_stdout_logs = true },
    general = { gaps_in = 6, gaps_out = 12, border_size = 2, layout = "dwindle" },
    decoration = {
        rounding = 0,
        blur = { enabled = vivo, size = 3, passes = 1 },
        -- Brillo hielo en la ventana activa; sombra negra en las demas.
        shadow = { enabled = vivo, range = 18, render_power = 3, color = "rgba(5ad8ff40)", color_inactive = "rgba(00000099)" },
    },
    animations = { enabled = vivo },
    input = { kb_layout = "us", follow_mouse = 1 },
    cursor = { enable_hyprcursor = false },
    misc = { disable_hyprland_logo = true, force_default_wallpaper = 0 },
})
hl.curve("ultraEase", { type = "bezier", points = { {0.16, 1}, {0.3, 1} } })
hl.curve("lineal", { type = "bezier", points = { {0, 0}, {1, 1} } })
hl.animation({ leaf = "windows", enabled = true, speed = 4, bezier = "ultraEase", style = "popin 90%" })
hl.animation({ leaf = "windowsOut", enabled = true, speed = 2, bezier = "ultraEase", style = "popin 94%" })
hl.animation({ leaf = "windowsMove", enabled = true, speed = 3.5, bezier = "ultraEase" })
hl.animation({ leaf = "fade", enabled = true, speed = 2, bezier = "ultraEase" })
hl.animation({ leaf = "layers", enabled = true, speed = 2.5, bezier = "ultraEase", style = "slide" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 4, bezier = "ultraEase", style = "slide" })
hl.animation({ leaf = "specialWorkspace", enabled = true, speed = 4, bezier = "ultraEase", style = "slidevert" })
hl.animation({ leaf = "border", enabled = true, speed = 3, bezier = "ultraEase" })
-- Borde que gira (6 s por vuelta). Redibuja mientras haya una ventana con borde visible;
-- con efectos reducidos (Super+Shift+A) se detiene junto con el resto de animaciones.
hl.animation({ leaf = "borderangle", enabled = true, speed = 60, bezier = "lineal", style = "loop" })
hl.config({ general = { col = {
    active_border = { colors = {"rgba(5ad8ffff)", "rgba(ff2a4fff)", "rgba(00f0ffff)"}, angle = 45 },
    inactive_border = "rgba(2a2733ff)",
} } })
hl.on("hyprland.start", function()
    hl.exec_cmd("/usr/local/libexec/ultra-iniciar-escritorio")
end)
hl.bind("SUPER + Return", hl.dsp.exec_cmd("foot"))
hl.bind("SUPER + R", hl.dsp.exec_cmd("ultra-personalizar lanzador"))
hl.bind("SUPER + E", hl.dsp.exec_cmd("thunar"))
hl.bind("SUPER + N", hl.dsp.exec_cmd("mousepad"))
hl.bind("SUPER + B", hl.dsp.exec_cmd("ultra-brave"))
hl.bind("SUPER + G", hl.dsp.exec_cmd("ultra-brave brave://gpu"))
hl.bind("SUPER + H", hl.dsp.exec_cmd("foot /usr/bin/less /etc/ultra/AYUDA.txt"))
hl.bind("SUPER + SPACE", hl.dsp.exec_cmd("ultra-ambiente menu"))
hl.bind("SUPER + SHIFT + A", hl.dsp.exec_cmd("ultra-ambiente alternar"))
hl.bind("SUPER + SHIFT + P", hl.dsp.exec_cmd("ultra-personalizar menu"))
hl.bind("SUPER + SHIFT + I", hl.dsp.exec_cmd("ultra-incidente guardar"))
hl.bind("Print", hl.dsp.exec_cmd("ultra-captura pantalla"))
hl.bind("SUPER + SHIFT + S", hl.dsp.exec_cmd("ultra-captura region"))
hl.bind("SUPER + Print", hl.dsp.exec_cmd("ultra-captura ventana"))
hl.bind("SUPER + C", hl.dsp.exec_cmd("makoctl dismiss -a"))
hl.bind("SUPER + A", hl.dsp.exec_cmd("ultra-audio menu"))
hl.bind("SUPER + J", hl.dsp.exec_cmd("ultra-juegos menu"))
hl.bind("SUPER + F10", hl.dsp.exec_cmd("foot --title='ULTRA / Juegos en vivo' ultra-juegos-vivo panel"))
hl.bind("SUPER + F9", hl.dsp.exec_cmd("foot --title='ULTRA / Equipo en vivo' watch -n 2 ultra-equipo texto"))
hl.bind("SUPER + Escape", hl.dsp.exec_cmd("foot --fullscreen --title='ULTRA / Aplicaciones' ultra-aplicaciones"))
hl.bind("SUPER + F11", hl.dsp.exec_cmd("ultra-diagnostico-juego iniciar"))
hl.bind("SUPER + SHIFT + F11", hl.dsp.exec_cmd("ultra-diagnostico-juego marcar"))
hl.bind("SUPER + CTRL + F11", hl.dsp.exec_cmd("foot --title='ULTRA / Warframe diagnostico' ultra-diagnostico-juego panel"))
hl.bind("SUPER + SHIFT + G", hl.dsp.exec_cmd("ultra-graficos menu"))
hl.bind("SUPER + P", hl.dsp.exec_cmd("ultra-pantallas menu"))
hl.bind("SUPER + SHIFT + E", hl.dsp.exec_cmd("ultra-windows"))
hl.bind("XF86AudioRaiseVolume", hl.dsp.exec_cmd("ultra-audio subir"), { repeating = true })
hl.bind("XF86AudioLowerVolume", hl.dsp.exec_cmd("ultra-audio bajar"), { repeating = true })
hl.bind("XF86AudioMute", hl.dsp.exec_cmd("ultra-audio silencio"))
hl.bind("SUPER + Q", hl.dsp.window.close())
hl.bind("SUPER + V", hl.dsp.window.float({ action = "toggle" }))
hl.bind("SUPER + SHIFT + M", hl.dsp.exit())
for _, direction in ipairs({"left", "right", "up", "down"}) do
    hl.bind("SUPER + " .. direction, hl.dsp.focus({ direction = direction }))
end
for i = 1, 9 do
    hl.bind("SUPER + " .. i, hl.dsp.focus({ workspace = i }))
    hl.bind("SUPER + SHIFT + " .. i, hl.dsp.window.move({ workspace = i }))
end
hl.bind("SUPER + mouse:272", hl.dsp.window.drag(), { mouse = true })
hl.bind("SUPER + mouse:273", hl.dsp.window.resize(), { mouse = true })
