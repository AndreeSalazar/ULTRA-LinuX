#!/usr/bin/env python3
"""Audit exported Ethernet facts. Does not connect to or configure any device."""
import argparse
import hashlib
import json
from pathlib import Path
import re
from resumir_red import summarize


def audit(text):
    result = []
    pieces = re.split(r'\[INTERFAZ ([a-zA-Z0-9_.:-]+):[^\n]*\]\s*', text)
    for index in range(1,len(pieces),2):
        name, block = pieces[index], pieces[index+1].split('\n[',1)[0]
        fields = {}
        for key in ('driver','firmware-version','Speed','Duplex','Auto-negotiation',
                    'rx_errors','tx_errors','rx_missed','align_errors'):
            match = re.search(r'^\s*'+re.escape(key)+r':\s*([^\n]*)',block,re.M)
            fields[key] = match[1].strip() if match else None
        modes = {k:[] for k in ('Supported link modes','Advertised link modes','Link partner advertised link modes')}
        current = None
        for line in block.splitlines():
            match = re.match(r'\s*(Supported link modes|Advertised link modes|Link partner advertised link modes):\s*(.*)',line)
            if match:
                current, value = match[1], match[2]
            elif current and re.match(r'^\s+\d+base',line):
                value = line.strip()
            else:
                current = None
                continue
            modes[current].extend(re.findall(r'\d+base\S+',value))
        features = {}
        for key in ('rx-checksumming','tx-checksumming','tcp-segmentation-offload',
                    'generic-segmentation-offload','generic-receive-offload','receive-hashing'):
            match = re.search(r'^'+re.escape(key)+r':\s*([^\n]+)',block,re.M)
            features[key] = match[1].strip() if match else None
        own = [int(re.match(r'\d+',m)[0]) for m in modes['Advertised link modes']]
        peer = [int(re.match(r'\d+',m)[0]) for m in modes['Link partner advertised link modes']]
        result.append({'interfaz':name,'campos':fields,'modos':modes,'offloads':features,
                       'max_anunciado_local_Mbps':max(own,default=None),
                       'max_anunciado_otro_extremo_Mbps':max(peer,default=None)})
    rates = summarize(text)
    samples = rates['intervalos']
    softnet = text.split('[SOFTNET:',1)
    dropped, squeeze, cpus = 0,0,0
    if len(softnet) == 2:
        for line in softnet[1].split('[INTERFAZ',1)[0].splitlines():
            words = line.split()
            if len(words)>=3 and all(re.fullmatch(r'[0-9a-fA-F]{8}',v) for v in words):
                cpus += 1; dropped += int(words[1],16); squeeze += int(words[2],16)
    return {'interfaces':result,'pico_RX_intervalo_Mbps':max((s['rx_Mbit_s'] for s in samples),default=None),
            'intervalos_validos':len(samples),'discontinuidades':rates['discontinuidades_o_muestras_invalidas'],
            'softnet_CPUs_observadas':cpus,'softnet_descartes':dropped if cpus else None,
            'softnet_time_squeeze':squeeze if cpus else None,
            'limites':['Capacidades anunciadas no prueban la capacidad fisica maxima del puerto remoto.',
                       'NIC sin errores no equivale a Internet sin perdidas o retransmisiones.',
                       'No identifica el modelo del router ni prueba el cable.',
                       'El pico es de la interfaz completa y promediado entre muestras.']}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('informe',type=Path)
    parser.add_argument('--kernel-config',type=Path,required=True)
    parser.add_argument('--salida',type=Path,required=True)
    args = parser.parse_args()
    with args.informe.open('rb') as stream:
        raw = stream.read(16*1024*1024+1)
    if len(raw)>16*1024*1024:
        raise ValueError('informe demasiado grande')
    result = audit(raw.decode('utf-8-sig',errors='replace'))
    result['informe_sha256'] = hashlib.sha256(raw).hexdigest()
    config = args.kernel_config.read_text()
    result['kernel_config_sha256'] = hashlib.sha256(args.kernel_config.read_bytes()).hexdigest()
    keys = ('NET_SCH_FQ_CODEL','NET_SCH_FQ','NET_SCH_CAKE','TCP_CONG_BBR',
            'TCP_CONG_CUBIC','RPS','RFS_ACCEL','XPS','BQL','NET_FLOW_LIMIT')
    result['configuracion_kernel'] = {}
    for key in keys:
        match = re.search(r'^CONFIG_'+key+r'=(.*)$',config,re.M)
        result['configuracion_kernel'][key] = match[1] if match else 'no habilitado'
    args.salida.parent.mkdir(parents=True,exist_ok=True)
    args.salida.write_text(json.dumps(result,indent=2,ensure_ascii=False)+'\n',encoding='utf-8',newline='\n')
    print(json.dumps(result,indent=2,ensure_ascii=False))


if __name__ == '__main__':
    main()
