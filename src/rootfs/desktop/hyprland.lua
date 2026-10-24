-- SPDX-License-Identifier: GPL-2.0-only
-- ULTRA / BMO-X: original neon shell for Hyprland 0.56.2.
local vivo = os.getenv("ULTRA_EFECTOS") ~= "sobrio"
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1 })
hl.config({
    debug = { enable_stdout_logs = true },
    general = { gaps_in = 6, gaps_out = 14, border_size = 2, layout = "dwindle" },
    decoration = {
        rounding = 8,
        blur = { enabled = vivo, size = 3, passes = 1 },
        shadow = { enabled = vivo, range = 12, render_power = 3, color = "rgba(00081499)" },
    },
    animations = { enabled = vivo },
    input = { kb_layout = "us", follow_mouse = 1 },
    cursor = { enable_hyprcursor = false },
    misc = { disable_hyprland_logo = true, force_default_wallpaper = 0 },
})
hl.curve("ultraEase", { type = "bezier", points = { {0.16, 1}, {0.3, 1} } })
hl.animation({ leaf = "windows", enabled = true, speed = 3, bezier = "ultraEase", style = "popin 96%" })
hl.animation({ leaf = "fade", enabled = true, speed = 2, bezier = "ultraEase" })
hl.animation({ leaf = "workspaces", enabled = true, speed = 3, bezier = "ultraEase", style = "slide" })
hl.config({ general = { col = {
    active_border = { colors = {"rgba(43deebff)", "rgba(a987e6ff)", "rgba(68bd9dff)"}, angle = 35 },
    inactive_border = "rgba(29473eff)",
} } })
hl.on("hyprland.start", function()
    hl.exec_cmd("/usr/local/libexec/ultra-iniciar-escritorio")
end)
hl.bind("SUPER + Return", hl.dsp.exec_cmd("foot"))
hl.bind("SUPER + R", hl.dsp.exec_cmd("fuzzel"))
hl.bind("SUPER + E", hl.dsp.exec_cmd("thunar"))
hl.bind("SUPER + N", hl.dsp.exec_cmd("mousepad"))
hl.bind("SUPER + B", hl.dsp.exec_cmd("ultra-brave"))
hl.bind("SUPER + G", hl.dsp.exec_cmd("ultra-brave brave://gpu"))
hl.bind("SUPER + H", hl.dsp.exec_cmd("foot /usr/bin/less /etc/ultra/AYUDA.txt"))
hl.bind("SUPER + SPACE", hl.dsp.exec_cmd("ultra-ambiente menu"))
hl.bind("SUPER + SHIFT + A", hl.dsp.exec_cmd("ultra-ambiente alternar"))
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
