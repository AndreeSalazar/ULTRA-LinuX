# SPDX-License-Identifier: GPL-2.0-only
# ssh splits option values on spaces, and this folder name has one: use a relative path.
Set-Location -LiteralPath $PSScriptRoot
& ssh.exe -i 'vm/builder_ed25519' -p 12222 -o 'StrictHostKeyChecking=accept-new' -o 'UserKnownHostsFile=vm/known_hosts' builder@127.0.0.1
