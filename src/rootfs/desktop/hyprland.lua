-- SPDX-License-Identifier: GPL-2.0-only
-- ULTRA baseline for Hyprland 0.56.2; conservative effects, explicit shortcuts.
hl.monitor({ output = "", mode = "preferred", position = "auto", scale = 1 })
hl.config({
    debug = { enable_stdout_logs = true },
    general = { gaps_in = 5, gaps_out = 10, border_size = 2, layout = "dwindle" },
    decoration = { rounding = 3, blur = { enabled = false }, shadow = { enabled = false } },
    animations = { enabled = false },
    input = { kb_layout = "us", follow_mouse = 1 },
    misc = { disable_hyprland_logo = true, force_default_wallpaper = 0 },
})
hl.config({ general = { col = {
    active_border = { colors = {"rgba(00e5ffff)", "rgba(48ffb4ff)"}, angle = 35 },
    inactive_border = "rgba(23364dff)",
} } })
hl.on("hyprland.start", function()
    hl.exec_cmd("swaybg -i /etc/ultra/wallpaper.svg -m fill")
    hl.exec_cmd("waybar -c /etc/ultra/waybar.jsonc -s /etc/ultra/waybar.css")
    hl.exec_cmd("foot --title=ULTRA-Bienvenida /usr/bin/less /etc/ultra/AYUDA.txt")
    hl.exec_cmd("foot --title=ULTRA-Terminal")
end)
hl.bind("SUPER + Return", hl.dsp.exec_cmd("foot"))
hl.bind("SUPER + R", hl.dsp.exec_cmd("fuzzel"))
hl.bind("SUPER + E", hl.dsp.exec_cmd("thunar"))
hl.bind("SUPER + N", hl.dsp.exec_cmd("mousepad"))
hl.bind("SUPER + H", hl.dsp.exec_cmd("foot /usr/bin/less /etc/ultra/AYUDA.txt"))
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
