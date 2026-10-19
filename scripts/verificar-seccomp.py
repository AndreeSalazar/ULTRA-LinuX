#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-only
"""Symbolic equivalence of the emitted x86-64 cBPF and an independent contract.

This proves a property of a finite filter model, not kernel/C/hardware correctness.
The syscall grant set comes from the inventory; scalar rules below are the spec.
"""
import argparse
import csv
from functools import lru_cache
import hashlib
import pathlib
import struct
import z3

ALLOW, TRACE, KILL = 0x7fff0000, 0x7ff00000, 0x80000000

def symbolic_filter(raw, words):
    if not raw or len(raw) % 8 or len(raw) > 8 * 256:
        raise ValueError('formato o longitud BPF invalida')
    code = list(struct.iter_unpack('<HBBI', raw))
    @lru_cache(None)
    def run(pc, slot):
        if not 0 <= pc < len(code):
            raise ValueError('salto fuera del filtro')
        op, jt, jf, k = code[pc]
        if op == 0x20:  # LD W ABS; aligned words in seccomp_data.
            if k % 4 or not 0 <= k <= 60:
                raise ValueError('carga fuera de seccomp_data o no alineada')
            return run(pc + 1, k // 4)
        if op == 0x06:  # RET K.
            return z3.BitVecVal(k, 32)
        value = words[slot] if slot >= 0 else z3.BitVecVal(0, 32)
        if op == 0x15: condition = value == k  # JEQ K.
        elif op == 0x35: condition = z3.UGE(value, k)  # JGE unsigned.
        elif op == 0x45: condition = value & k != 0  # JSET K.
        else: raise ValueError(f'opcode no modelado: {op:#x}')
        return z3.If(condition, run(pc + 1 + jt, slot), run(pc + 1 + jf, slot))
    return run(0, -1), len(code)

def contract(words, grants, numbers):
    nr, arch = words[:2]
    arg = lambda n: words[4 + 2 * n]  # low 32 bits; kernel commands are int/u32.
    member = lambda value, choices: z3.Or(*[value == x for x in choices])
    permitted = member(nr, grants)
    restrictions = {
        'ioctl': (1, (0x5401, 0x5402, 0x5403, 0x5404, 0x5413, 0x540f, 0x541b)),
        'fcntl': (1, (0, 1, 2, 3, 4, 1030)),
        'prctl': (0, (3, 15, 16, 21, 39)),
        'arch_prctl': (0, (0x1002, 0x1003, 0x1004)),
        'setuid': (0, (65534,)), 'setgid': (0, (65534,)),
    }
    for name, (position, values) in restrictions.items():
        permitted = z3.And(permitted, z3.Implies(nr == numbers[name], member(arg(position), values)))
    for name in ('mmap', 'mprotect'):
        permitted = z3.And(permitted, z3.Implies(nr == numbers[name], arg(2) & 4 == 0))
    return z3.If(z3.Or(arch != 0xc000003e, z3.UGE(nr, 0x40000000)),
                 z3.BitVecVal(KILL, 32), z3.If(permitted, z3.BitVecVal(ALLOW, 32), z3.BitVecVal(TRACE, 32)))

def main():
    p = argparse.ArgumentParser()
    p.add_argument('--bpf', type=pathlib.Path, required=True)
    p.add_argument('--catalogo', type=pathlib.Path, required=True)
    a = p.parse_args()
    rows = list(csv.DictReader(a.catalogo.open(encoding='utf-8')))
    native = [r for r in rows if r['abi'] in ('common', '64')]
    numbers = {r['nombre']: int(r['numero']) for r in native}
    grants = [int(r['numero']) for r in native if r['politica'].startswith('permitida')]
    if len(grants) != 46 or len(set(grants)) != 46:
        raise ValueError('cambio de declaracion: revisar contrato de 46 llamadas')
    raw = a.bpf.read_bytes()
    words = [z3.BitVec(f'data_{i}', 32) for i in range(16)]
    actual, size = symbolic_filter(raw, words)
    expected = contract(words, grants, numbers)
    solver = z3.Solver()
    solver.set(timeout=10000)
    solver.add(actual != expected)
    result = solver.check()
    if result != z3.unsat:
        raise SystemExit(f'ROJO: equivalencia no demostrada ({result})' + (f' {solver.model()}' if result == z3.sat else ''))
    # Mutation control: accepting everything must yield a counterexample.
    mutation = z3.Solver()
    mutation.set(timeout=10000)
    mutated, _ = symbolic_filter(struct.pack('<HBBI', 0x06, 0, 0, ALLOW), words)
    mutation.add(mutated != expected)
    if mutation.check() != z3.sat:
        raise SystemExit('ROJO: control de mutacion del contrato fallo')
    print(f'FORMAL-OK: Z3 {z3.get_version_string()}; {size} instrucciones; equivalencia UNSAT para seccomp_data de 512 bits')
    print('CONTROL-OK: filtro permitir-todo rechazado por contraejemplo')
    print(f'BPF-SHA256: {hashlib.sha256(raw).hexdigest()}')
    print('ALCANCE: semantica cBPF modelada y contrato declarado; no prueba correccion del kernel, C, hardware ni datos ya concedidos')

if __name__ == '__main__':
    main()
