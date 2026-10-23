if [[ -r /home/builder/linux-reconstruido/recursos-prod-0983bec7bad24b89bcef0b34770f3ff8/benchmark/proc/meminfo ]]; then
    while read -r key value unit rest; do
        [[ $value =~ ^[0-9]+$ ]] || continue
        case "$key" in
            MemTotal:|MemAvailable:|AnonPages:|Cached:|Buffers:|Shmem:|Slab:|SReclaimable:|SUnreclaim:|Dirty:|Writeback:|PageTables:|KernelStack:|Unevictable:|Mlocked:|SwapTotal:|SwapFree:|SwapCached:|AnonHugePages:|HugePages_Total:|HugePages_Free:|Hugepagesize:)
                printf 'MEM %s %s %s\n' "$key" "$value" "$unit";;
        esac
    done < /home/builder/linux-reconstruido/recursos-prod-0983bec7bad24b89bcef0b34770f3ff8/benchmark/proc/meminfo
else printf 'MEM NA\n'; fi
if [[ -r /home/builder/linux-reconstruido/recursos-prod-0983bec7bad24b89bcef0b34770f3ff8/benchmark/proc/vmstat ]]; then
    while read -r key value rest; do
        [[ $value =~ ^[0-9]+$ ]] || continue
        case "$key" in
            pgfault|pgmajfault|pswpin|pswpout|pgscan_*|pgsteal_*|allocstall*|compact_stall|compact_fail|compact_success|oom_kill|workingset_refault_*|thp_fault_alloc|thp_fault_fallback)
                printf 'VM %s=%s\n' "$key" "$value";;
        esac
    done < /home/builder/linux-reconstruido/recursos-prod-0983bec7bad24b89bcef0b34770f3ff8/benchmark/proc/vmstat
else printf 'VM NA\n'; fi
