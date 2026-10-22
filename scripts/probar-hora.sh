#!/bin/bash
# SPDX-License-Identifier: GPL-2.0-only
# Synthetic chronyd result in fresh user/mount/network namespaces; no real clock set.
set -euo pipefail
helper=${1:?helper absoluto}
out=${2:?salida nueva}
[[ -f $helper && ! -e $out && -x /usr/bin/chronyd ]] || exit 1
mkdir -p "$out"
cat > "$out/chronyd" <<'STUB'
#!/bin/bash
echo called >> /run/calls
case $ULTRA_TEST_MODE in fail) exit 1;; busy) sleep 1;; esac
exit 0
STUB
chmod 0755 "$out/chronyd"
cat > "$out/runner" <<'RUNNER'
#!/bin/bash
set -euo pipefail
case $ULTRA_TEST_MODE in
 fail)
    if bash /helper > /evidence/fail.txt; then exit 1; fi
    [[ ! -e /run/ultra-hora/sincronizado ]]
    grep -q 'sincronizacion pendiente' /evidence/fail.txt;;
 success)
    bash /helper > /evidence/success.txt
    grep -Eq '^[0-9]{4}-.*Z$' /run/ultra-hora/sincronizado
    bash /helper >> /evidence/success.txt
    [[ $(wc -l < /run/calls) == 1 ]]
    grep -q 'menos de una hora' /evidence/success.txt;;
 busy)
    bash /helper > /evidence/busy-first.txt & child=$!
    for ((i=0;i<50;i++)); do [[ ! -f /run/calls ]] || break; sleep .02; done
    [[ -f /run/calls ]]
    bash /helper > /evidence/busy-second.txt
    grep -q 'sincronizacion en curso' /evidence/busy-second.txt
    wait "$child"
    [[ $(wc -l < /run/calls) == 1 && -s /run/ultra-hora/sincronizado ]];;
esac
printf 'HORA-TEST-OK: %s; stub sin cambios al reloj\n' "$ULTRA_TEST_MODE"
RUNNER
for mode in fail success busy; do
    timeout -k 2 10 bwrap --unshare-all --die-with-parent --new-session --uid 0 --gid 0 \
        --ro-bind /usr /usr --symlink usr/bin /bin --symlink usr/lib /lib --symlink usr/lib /lib64 \
        --proc /proc --dev /dev --tmpfs /run --tmpfs /tmp \
        --ro-bind "$out/chronyd" /usr/bin/chronyd --ro-bind "$helper" /helper \
        --ro-bind "$out/runner" /runner --bind "$out" /evidence \
        --clearenv --setenv PATH /usr/bin --setenv ULTRA_TEST_MODE "$mode" \
        /bin/bash /runner
done
