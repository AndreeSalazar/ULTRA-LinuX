# SPDX-License-Identifier: GPL-2.0-only
# Explicit opt-in: research flags never enter the production profile.
# Diagnosis: ASan and full UBSan name the exact fault; slower, never shipped.
ULTRA_DIAG=(-O1 -g -fno-omit-frame-pointer -fsanitize=address,undefined,float-cast-overflow,bounds-strict,pointer-compare,pointer-subtract
            -fno-sanitize-recover=all -fcf-protection=full)
export ASAN_OPTIONS=detect_stack_use_after_return=1:detect_leaks=1:detect_invalid_pointer_pairs=2
# Raw: every guardian off, including Arch's default stack protector and RELRO.
# Only for the minefield, to show what each mine does when nobody watches.
ULTRA_CRUDO=(-O2 -fno-stack-protector -fno-stack-clash-protection -fcf-protection=none
             -U_FORTIFY_SOURCE -D_FORTIFY_SOURCE=0 -Wl,-z,norelro -DULTRA_CRUDO)
