#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Linux integration checks with disposable homes and synthetic secrets only."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]


def executable(path, text):
    path.write_text(text)
    path.chmod(0o700)


class Persistence(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(prefix="ultra-llavero-")
        self.addCleanup(self.temp.cleanup)
        self.base = Path(self.temp.name)
        self.home = self.base / "home con espacios"
        self.bin = self.base / "bin"
        self.home.mkdir(mode=0o700)
        self.bin.mkdir(mode=0o700)
        self.env = {**os.environ, "HOME": str(self.home),
                    "XDG_CONFIG_HOME": str(self.home / ".config"),
                    "XDG_DATA_HOME": str(self.home / ".local/share"),
                    "XDG_CACHE_HOME": str(self.home / ".cache"),
                    "WAYLAND_DISPLAY": "wayland-fixture",
                    "PATH": f"{self.bin}:{os.environ['PATH']}"}
        # Never connect a test to an inherited real desktop bus or keyring.
        for key in ("DBUS_SESSION_BUS_ADDRESS", "DBUS_STARTER_ADDRESS",
                    "DBUS_STARTER_BUS_TYPE", "GNOME_KEYRING_CONTROL"):
            self.env.pop(key, None)
        runtime = self.base / "runtime"
        runtime.mkdir(mode=0o700)
        self.env["XDG_RUNTIME_DIR"] = str(runtime)

    def run_cmd(self, args, **kwargs):
        return subprocess.run(args, env=self.env, capture_output=True,
                              text=True, timeout=30, **kwargs)

    def test_real_keyring_across_desktop_sessions(self):
        # Audio and compositor are fixtures. Since SSH has no graphical prompt,
        # inject a synthetic unlock password at daemon startup in this test only.
        # The supervisor, DBus, keyring implementation and secret-tool are real.
        executable(self.bin / "gnome-keyring-daemon", '''#!/usr/bin/env python3
import os, sys
reader, writer = os.pipe()
os.write(writer, b"ULTRA-fixture-password-only")
os.close(writer)
os.dup2(reader, 0)
os.close(reader)
args = ["--unlock" if arg == "--start" else arg for arg in sys.argv[1:]]
os.execv("/usr/bin/gnome-keyring-daemon", ["gnome-keyring-daemon", *args])
''')
        for program in ("pipewire", "pipewire-pulse", "wireplumber", "wpctl"):
            executable(self.bin / program, "#!/bin/sh\nexit 0\n")
        executable(self.bin / "start-hyprland", '''#!/usr/bin/env python3
import os, subprocess, time
for attempt in range(50):
    ping = subprocess.run(["dbus-send", "--session", "--print-reply",
        "--reply-timeout=500", "--dest=org.freedesktop.secrets",
        "/org/freedesktop/secrets", "org.freedesktop.DBus.Peer.Ping"],
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    if ping.returncode == 0:
        break
    time.sleep(.1)
else:
    raise SystemExit("fixture: Secret Service unavailable")
if os.environ["ULTRA_TEST_PHASE"] == "store":
    subprocess.run(["secret-tool", "store", "--label=ULTRA synthetic test",
        "ultra-fixture", "persistence"], input="ULTRA-synthetic-value-only",
        text=True, check=True, timeout=10)
else:
    result = subprocess.run(["secret-tool", "lookup", "ultra-fixture",
        "persistence"], capture_output=True, text=True, check=True, timeout=10)
    assert result.stdout.strip() == "ULTRA-synthetic-value-only"
''')
        script = ROOT / "src/rootfs/bin/ultra-desktop-session"
        for phase in ("store", "lookup"):
            # A new runtime and bus for each login, with the same persistent HOME.
            runtime = self.base / f"runtime-{phase}"
            runtime.mkdir(mode=0o700)
            self.env.update(XDG_RUNTIME_DIR=str(runtime), ULTRA_TEST_PHASE=phase)
            result = self.run_cmd(["dbus-run-session", "--", "bash", str(script)])
            self.assertEqual(result.returncode, 0, result.stderr)
            # Session supervisor must reap the daemon before the next login.
            self.assertFalse((runtime / "keyring/control").exists())
        keyrings = Path(self.env["XDG_DATA_HOME"]) / "keyrings"
        self.assertEqual(keyrings.stat().st_mode & 0o777, 0o700)
        data = (keyrings / "login.keyring").read_bytes()
        self.assertTrue(data.startswith(b"GnomeKeyring\n\r\x00\n"))
        self.assertNotIn(b"ULTRA-synthetic-value-only", data)
        self.assertNotIn(b"ULTRA-fixture-password-only", data)

    def test_launcher_reuses_profile_and_requires_service(self):
        fake = self.bin / "brave-fixture"
        executable(fake, '''#!/usr/bin/env python3
import json, os, sys
from pathlib import Path
Path(os.environ["HOME"], "arguments.json").write_text(json.dumps(sys.argv[1:]))
''')
        # Only the browser binary path is substituted in this disposable copy;
        # the argument validation, profile selection and service gate are real.
        script = self.base / "ultra-brave"
        text = (ROOT / "src/rootfs/bin/ultra-brave").read_text()
        text = text.replace("/opt/brave/brave", '"' + str(fake) + '"')
        executable(script, text)
        executable(self.bin / "dbus-send", "#!/bin/sh\nexit ${ULTRA_TEST_DBUS_EXIT:-0}\n")
        profile = Path(self.env["XDG_CONFIG_HOME"]) / "BraveSoftware/Brave-Browser"
        profile.mkdir(parents=True)
        sentinel = profile / "existing-profile-fixture"
        sentinel.write_text("keep")
        for _ in range(2):
            result = self.run_cmd(["bash", str(script), "https://example.invalid/"])
            self.assertEqual(result.returncode, 0, result.stderr)
            args = json.loads((self.home / "arguments.json").read_text())
            self.assertIn(f"--user-data-dir={profile}", args)
            self.assertIn("--password-store=gnome-libsecret", args)
            self.assertNotIn("--no-sandbox", args)
            self.assertEqual(sentinel.read_text(), "keep")
        (self.home / "arguments.json").unlink()
        self.env["ULTRA_TEST_DBUS_EXIT"] = "1"
        self.assertEqual(self.run_cmd(["bash", str(script)]).returncode, 1)
        self.assertFalse((self.home / "arguments.json").exists())
        self.env["ULTRA_TEST_DBUS_EXIT"] = "0"
        self.assertEqual(self.run_cmd(["bash", str(script), "--incognito"]).returncode, 2)
        self.assertFalse((self.home / "arguments.json").exists())

    def test_desktop_starts_service_without_password(self):
        for program in ("pipewire", "pipewire-pulse", "wireplumber", "wpctl"):
            executable(self.bin / program, "#!/bin/sh\nexit 0\n")
        executable(self.bin / "start-hyprland", '''#!/bin/sh
exec dbus-send --session --print-reply --reply-timeout=5000 \\
 --dest=org.freedesktop.secrets /org/freedesktop/secrets org.freedesktop.DBus.Peer.Ping
''')
        result = self.run_cmd(["dbus-run-session", "--", "bash",
                              str(ROOT / "src/rootfs/bin/ultra-desktop-session")])
        self.assertEqual(result.returncode, 0, result.stderr)
        keyrings = Path(self.env["XDG_DATA_HOME"]) / "keyrings"
        self.assertFalse((keyrings / "login.keyring").exists(),
                         "autologin must not create an empty-password keyring")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=ROOT)
    options = parser.parse_args()
    ROOT = options.source_root.resolve()
    if os.getuid() != 1000:
        parser.error("run as UID 1000, as ULTRA's desktop does")
    for required in ("bash", "dbus-run-session", "dbus-send",
                     "gnome-keyring-daemon", "secret-tool"):
        if not shutil.which(required):
            parser.error(f"missing dependency: {required}")
    unittest.main(argv=[__file__], verbosity=2)
